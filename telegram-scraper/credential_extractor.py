"""
Credential Extractor Module
Extracts, categorizes, and deduplicates secrets from text content.
Outputs categorized secrets to respective files before posting.
"""

import re
import os
import hashlib
import sqlite3
import logging
from pathlib import Path
from datetime import datetime
from dataclasses import dataclass, field
from typing import Dict, List, Set, Tuple, Optional
from collections import defaultdict

logger = logging.getLogger(__name__)

# Output directory for categorized secrets
OUTPUT_DIR = os.getenv('SECRETS_OUTPUT_DIR', '/opt/skybin/extracted_secrets')

# Database path for deduplication
DB_PATH = os.getenv('SKYBIN_DB_PATH', '/opt/skybin/pastevault.db')

# Server secrets to exclude (loaded from environment or file)
EXCLUDED_SECRETS_FILE = os.getenv('EXCLUDED_SECRETS_FILE', '/opt/skybin/.excluded_secrets')


@dataclass
class ExtractedSecret:
    """Represents an extracted secret with metadata"""
    secret_type: str
    category: str
    value: str
    context: str = ""
    line_number: int = 0
    
    @property
    def hash(self) -> str:
        """Generate unique hash for deduplication"""
        return hashlib.sha256(f"{self.secret_type}:{self.value}".encode()).hexdigest()


@dataclass
class ExtractionResult:
    """Result of credential extraction"""
    secrets: List[ExtractedSecret] = field(default_factory=list)
    new_secrets: List[ExtractedSecret] = field(default_factory=list)  # Not seen before
    duplicate_secrets: List[ExtractedSecret] = field(default_factory=list)  # Already seen
    excluded_secrets: List[ExtractedSecret] = field(default_factory=list)  # Server secrets
    categories: Dict[str, List[ExtractedSecret]] = field(default_factory=lambda: defaultdict(list))
    
    @property
    def total_count(self) -> int:
        return len(self.secrets)
    
    @property
    def new_count(self) -> int:
        return len(self.new_secrets)


# =============================================================================
# CATEGORIZED SECRET PATTERNS
# =============================================================================

SECRET_PATTERNS: Dict[str, Dict[str, re.Pattern]] = {
    # ---------------------------------------------------------------------
    # CLOUD PROVIDER KEYS
    # ---------------------------------------------------------------------
    "AWS_Keys": {
        "AWS_Access_Key": re.compile(r'\b(AKIA[0-9A-Z]{16})\b'),
        "AWS_Secret_Key": re.compile(r'(?i)aws[_-]?secret[_-]?(?:access[_-]?)?key[\'"]?\s*[:=]\s*[\'"]?([A-Za-z0-9/+=]{40})[\'"]?'),
        "AWS_Session_Token": re.compile(r'(?i)aws[_-]?session[_-]?token\s*[:=]\s*[\'"]?([A-Za-z0-9/+=]{100,})[\'"]?'),
        "AWS_MWS_Token": re.compile(r'\b(amzn\.mws\.[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\b'),
    },
    
    "Azure_Keys": {
        "Azure_Storage_Key": re.compile(r'(?i)AccountKey=([A-Za-z0-9+/=]{88})'),
        "Azure_Client_Secret": re.compile(r'(?i)azure[_-]?client[_-]?secret\s*[:=]\s*[\'"]?([a-zA-Z0-9._%+-]{32,})[\'"]?'),
        "Azure_Connection_String": re.compile(r'(?i)(DefaultEndpointsProtocol=https?;AccountName=[^;]+;AccountKey=[A-Za-z0-9+/=]+)'),
    },
    
    "GCP_Keys": {
        "Google_API_Key": re.compile(r'\b(AIza[0-9A-Za-z_-]{35})\b'),
        "Google_OAuth_Token": re.compile(r'\b(ya29\.[0-9A-Za-z_-]+)\b'),
        "GCP_Service_Account": re.compile(r'"type"\s*:\s*"service_account"[^}]+?"private_key"\s*:\s*"([^"]+)"'),
    },
    
    "DigitalOcean_Keys": {
        "DO_Personal_Token": re.compile(r'\b(dop_v1_[a-f0-9]{64})\b'),
        "DO_OAuth_Token": re.compile(r'\b(doo_v1_[a-f0-9]{64})\b'),
        "DO_Refresh_Token": re.compile(r'\b(dor_v1_[a-f0-9]{64})\b'),
    },
    
    # ---------------------------------------------------------------------
    # DEVELOPER PLATFORM TOKENS
    # ---------------------------------------------------------------------
    "GitHub_Tokens": {
        "GitHub_PAT": re.compile(r'\b(ghp_[a-zA-Z0-9]{36})\b'),
        "GitHub_OAuth": re.compile(r'\b(gho_[a-zA-Z0-9]{36})\b'),
        "GitHub_App": re.compile(r'\b(ghu_[a-zA-Z0-9]{36})\b'),
        "GitHub_Server": re.compile(r'\b(ghs_[a-zA-Z0-9]{36})\b'),
        "GitHub_Refresh": re.compile(r'\b(ghr_[a-zA-Z0-9]{36})\b'),
        "GitHub_Fine_Grained": re.compile(r'\b(github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59})\b'),
    },
    
    "GitLab_Tokens": {
        "GitLab_PAT": re.compile(r'\b(glpat-[a-zA-Z0-9_-]{20})\b'),
        "GitLab_Pipeline": re.compile(r'\b(glptt-[a-f0-9]{40})\b'),
        "GitLab_Runner": re.compile(r'\b(GR1348941[a-zA-Z0-9_-]{20})\b'),
    },
    
    "NPM_Tokens": {
        "NPM_Token": re.compile(r'\b(npm_[A-Za-z0-9]{36})\b'),
        "NPM_Auth": re.compile(r'//registry\.npmjs\.org/:_authToken=([a-f0-9-]{36})'),
    },
    
    "Docker_Tokens": {
        "Docker_Config_Auth": re.compile(r'"auth"\s*:\s*"([A-Za-z0-9+/=]{20,})"'),
    },
    
    # ---------------------------------------------------------------------
    # AI/ML API KEYS
    # ---------------------------------------------------------------------
    "OpenAI_Keys": {
        "OpenAI_API_Key": re.compile(r'\b(sk-[a-zA-Z0-9]{48})\b'),
        "OpenAI_Project_Key": re.compile(r'\b(sk-proj-[a-zA-Z0-9_-]{80,})\b'),
        "OpenAI_Org_Key": re.compile(r'\b(org-[a-zA-Z0-9]{24})\b'),
    },
    
    "Anthropic_Keys": {
        "Anthropic_API_Key": re.compile(r'\b(sk-ant-api03-[a-zA-Z0-9_-]{93})\b'),
    },
    
    "Huggingface_Tokens": {
        "HF_Token": re.compile(r'\b(hf_[a-zA-Z0-9]{34})\b'),
    },
    
    # ---------------------------------------------------------------------
    # COMMUNICATION PLATFORM TOKENS
    # ---------------------------------------------------------------------
    "Discord_Tokens": {
        "Discord_Bot_Token": re.compile(r'\b([MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27})\b'),
        "Discord_Webhook": re.compile(r'(https://(?:ptb\.|canary\.)?discord(?:app)?\.com/api/webhooks/\d+/[A-Za-z0-9_-]+)'),
    },
    
    "Slack_Tokens": {
        "Slack_Bot_Token": re.compile(r'\b(xoxb-[0-9]{10,}-[0-9]{10,}-[a-zA-Z0-9]{24})\b'),
        "Slack_User_Token": re.compile(r'\b(xoxp-[0-9]{10,}-[0-9]{10,}-[a-zA-Z0-9]{24,})\b'),
        "Slack_App_Token": re.compile(r'\b(xapp-[0-9]-[A-Z0-9]{10,}-[0-9]{12,}-[a-z0-9]{64})\b'),
        "Slack_Webhook": re.compile(r'(https://hooks\.slack\.com/services/T[A-Z0-9]{8,}/B[A-Z0-9]{8,}/[A-Za-z0-9]{24})'),
    },
    
    "Telegram_Tokens": {
        "Telegram_Bot_Token": re.compile(r'\b(\d{8,10}:[A-Za-z0-9_-]{35})\b'),
    },
    
    "Twilio_Keys": {
        "Twilio_API_Key": re.compile(r'\b(SK[0-9a-fA-F]{32})\b'),
        "Twilio_Account_SID": re.compile(r'\b(AC[a-zA-Z0-9_-]{32})\b'),
        "Twilio_Auth_Token": re.compile(r'(?i)twilio[_-]?auth[_-]?token\s*[:=]\s*[\'"]?([a-f0-9]{32})[\'"]?'),
    },
    
    # ---------------------------------------------------------------------
    # EMAIL SERVICE KEYS
    # ---------------------------------------------------------------------
    "SendGrid_Keys": {
        "SendGrid_API_Key": re.compile(r'\b(SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43})\b'),
    },
    
    "Mailchimp_Keys": {
        "Mailchimp_API_Key": re.compile(r'\b([0-9a-f]{32}-us\d{1,2})\b'),
    },
    
    "Mailgun_Keys": {
        "Mailgun_API_Key": re.compile(r'\b(key-[0-9a-zA-Z]{32})\b'),
    },
    
    "Postmark_Keys": {
        "Postmark_Server_Token": re.compile(r'\b([a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12})\b'),
    },
    
    # ---------------------------------------------------------------------
    # PAYMENT PLATFORM KEYS
    # ---------------------------------------------------------------------
    "Stripe_Keys": {
        "Stripe_Live_Key": re.compile(r'\b(sk_live_[0-9a-zA-Z]{24,})\b'),
        "Stripe_Test_Key": re.compile(r'\b(sk_test_[0-9a-zA-Z]{24,})\b'),
        "Stripe_Restricted_Key": re.compile(r'\b(rk_live_[0-9a-zA-Z]{24,})\b'),
        "Stripe_Publishable_Live": re.compile(r'\b(pk_live_[0-9a-zA-Z]{24,})\b'),
    },
    
    "PayPal_Keys": {
        "PayPal_Client_Secret": re.compile(r'(?i)paypal[_-]?(?:client[_-]?)?secret\s*[:=]\s*[\'"]?([A-Za-z0-9_-]{40,})[\'"]?'),
    },
    
    "Square_Keys": {
        "Square_Access_Token": re.compile(r'\b(sq0atp-[0-9A-Za-z_-]{22})\b'),
        "Square_OAuth_Secret": re.compile(r'\b(sq0csp-[0-9A-Za-z_-]{43})\b'),
    },
    
    # ---------------------------------------------------------------------
    # STREAMING SERVICE CREDENTIALS
    # ---------------------------------------------------------------------
    "Netflix_Credentials": {
        "Netflix_Session": re.compile(r'(?i)NetflixId=([A-Za-z0-9%_-]{50,})'),
        "Netflix_Email_Pass": re.compile(r'(?i)netflix[^\n]*?([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+)[:\s]+([^\s]{6,})'),
    },
    
    "Disney_Credentials": {
        "Disney_Token": re.compile(r'(?i)disney[_-]?(?:plus)?[_-]?(?:token|auth|session)\s*[:=]\s*[\'"]?([A-Za-z0-9_-]{30,})[\'"]?'),
    },
    
    "HBO_Credentials": {
        "HBO_Token": re.compile(r'(?i)hbo[_-]?(?:max)?[_-]?(?:token|auth|session)\s*[:=]\s*[\'"]?([A-Za-z0-9_-]{30,})[\'"]?'),
    },
    
    "Spotify_Credentials": {
        "Spotify_Client_Secret": re.compile(r'(?i)spotify[_-]?(?:client[_-]?)?secret\s*[:=]\s*[\'"]?([a-f0-9]{32})[\'"]?'),
        "Spotify_Refresh_Token": re.compile(r'(?i)spotify[_-]?refresh[_-]?token\s*[:=]\s*[\'"]?([A-Za-z0-9_-]{100,})[\'"]?'),
    },
    
    "Twitch_Credentials": {
        "Twitch_OAuth": re.compile(r'(?i)(oauth:[a-z0-9]{30})'),
        "Twitch_Client_Secret": re.compile(r'(?i)twitch[_-]?client[_-]?secret\s*[:=]\s*[\'"]?([a-z0-9]{30})[\'"]?'),
    },
    
    "Hulu_Credentials": {
        "Hulu_Token": re.compile(r'(?i)hulu[_-]?(?:token|auth|session)\s*[:=]\s*[\'"]?([A-Za-z0-9_-]{30,})[\'"]?'),
    },
    
    "Crunchyroll_Credentials": {
        "Crunchyroll_Token": re.compile(r'(?i)crunchyroll[_-]?(?:token|auth|session)\s*[:=]\s*[\'"]?([A-Za-z0-9_-]{30,})[\'"]?'),
    },
    
    # ---------------------------------------------------------------------
    # DATABASE CREDENTIALS
    # ---------------------------------------------------------------------
    "Database_URLs": {
        "MongoDB_URI": re.compile(r'(mongodb(?:\+srv)?://[^\s]+)'),
        "PostgreSQL_URI": re.compile(r'(postgres(?:ql)?://[^\s]+)'),
        "MySQL_URI": re.compile(r'(mysql://[^\s]+)'),
        "Redis_URI": re.compile(r'(redis://[^\s]+)'),
        "MSSQL_URI": re.compile(r'(mssql://[^\s]+)'),
        "SQLite_URI": re.compile(r'(sqlite://[^\s]+)'),
    },
    
    # ---------------------------------------------------------------------
    # PRIVATE KEYS
    # ---------------------------------------------------------------------
    "Private_Keys": {
        "RSA_Private_Key": re.compile(r'(-----BEGIN RSA PRIVATE KEY-----[\s\S]+?-----END RSA PRIVATE KEY-----)'),
        "DSA_Private_Key": re.compile(r'(-----BEGIN DSA PRIVATE KEY-----[\s\S]+?-----END DSA PRIVATE KEY-----)'),
        "EC_Private_Key": re.compile(r'(-----BEGIN EC PRIVATE KEY-----[\s\S]+?-----END EC PRIVATE KEY-----)'),
        "OpenSSH_Private_Key": re.compile(r'(-----BEGIN OPENSSH PRIVATE KEY-----[\s\S]+?-----END OPENSSH PRIVATE KEY-----)'),
        "PGP_Private_Key": re.compile(r'(-----BEGIN PGP PRIVATE KEY BLOCK-----[\s\S]+?-----END PGP PRIVATE KEY BLOCK-----)'),
    },
    
    # ---------------------------------------------------------------------
    # JWT & OAUTH TOKENS
    # ---------------------------------------------------------------------
    "JWT_Tokens": {
        "JWT": re.compile(r'\b(eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,})\b'),
    },
    
    "Bearer_Tokens": {
        "Bearer_Token": re.compile(r'(?i)(?:authorization|bearer)[:\s]+Bearer\s+([A-Za-z0-9_-]{20,})'),
    },
    
    # ---------------------------------------------------------------------
    # EMAIL:PASSWORD COMBOS
    # ---------------------------------------------------------------------
    "Email_Pass_Combos": {
        "Email_Password": re.compile(r'([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+):([^\s@:]{4,})'),
    },
    
    # ---------------------------------------------------------------------
    # URL:LOGIN:PASS (STEALER LOGS)
    # ---------------------------------------------------------------------
    "Stealer_Logs": {
        "URL_Login_Pass": re.compile(r'(https?://[^\s]+)[\s\t|:]+([^\s@]+)[\s\t|:]+([^\s]{4,})'),
    },
    
    # ---------------------------------------------------------------------
    # SOCIAL MEDIA TOKENS
    # ---------------------------------------------------------------------
    "Facebook_Tokens": {
        "Facebook_Access_Token": re.compile(r'\b(EAA[A-Za-z0-9]{100,})\b'),
    },
    
    "Twitter_Tokens": {
        "Twitter_Bearer": re.compile(r'\b(AAAAAAAAAAAAAAAAAAAAAA[A-Za-z0-9%]{40,})\b'),
        "Twitter_API_Key": re.compile(r'(?i)twitter[_-]?api[_-]?key\s*[:=]\s*[\'"]?([a-zA-Z0-9]{25})[\'"]?'),
    },
    
    "Instagram_Tokens": {
        "Instagram_Access_Token": re.compile(r'(?i)instagram[_-]?(?:access[_-]?)?token\s*[:=]\s*[\'"]?([a-zA-Z0-9.]+)[\'"]?'),
    },
    
    # ---------------------------------------------------------------------
    # GENERIC SECRETS
    # ---------------------------------------------------------------------
    "Generic_Secrets": {
        "Generic_API_Key": re.compile(r'(?i)(?:api[_-]?key|apikey)\s*[:=]\s*[\'"]?([a-zA-Z0-9_-]{20,})[\'"]?'),
        "Generic_Secret": re.compile(r'(?i)(?:secret|secret[_-]?key)\s*[:=]\s*[\'"]?([a-zA-Z0-9_-]{20,})[\'"]?'),
        "Generic_Token": re.compile(r'(?i)(?:token|access[_-]?token|auth[_-]?token)\s*[:=]\s*[\'"]?([a-zA-Z0-9_-]{20,})[\'"]?'),
        "Generic_Password": re.compile(r'(?i)(?:password|passwd|pwd)\s*[:=]\s*[\'"]?([^\s\'"]{8,})[\'"]?'),
    },
}

# Category to filename mapping
CATEGORY_FILES = {
    "AWS_Keys": "AWS_Keys.txt",
    "Azure_Keys": "Azure_Keys.txt",
    "GCP_Keys": "GCP_Keys.txt",
    "DigitalOcean_Keys": "DigitalOcean_Keys.txt",
    "GitHub_Tokens": "GitHub_Tokens.txt",
    "GitLab_Tokens": "GitLab_Tokens.txt",
    "NPM_Tokens": "NPM_Tokens.txt",
    "Docker_Tokens": "Docker_Tokens.txt",
    "OpenAI_Keys": "OpenAI_Keys.txt",
    "Anthropic_Keys": "Anthropic_Keys.txt",
    "Huggingface_Tokens": "Huggingface_Tokens.txt",
    "Discord_Tokens": "Discord_Tokens.txt",
    "Slack_Tokens": "Slack_Tokens.txt",
    "Telegram_Tokens": "Telegram_Tokens.txt",
    "Twilio_Keys": "Twilio_Keys.txt",
    "SendGrid_Keys": "SendGrid_Keys.txt",
    "Mailchimp_Keys": "Mailchimp_Keys.txt",
    "Mailgun_Keys": "Mailgun_Keys.txt",
    "Postmark_Keys": "Postmark_Keys.txt",
    "Stripe_Keys": "Stripe_Keys.txt",
    "PayPal_Keys": "PayPal_Keys.txt",
    "Square_Keys": "Square_Keys.txt",
    "Netflix_Credentials": "Netflix_Credentials.txt",
    "Disney_Credentials": "Disney_Credentials.txt",
    "HBO_Credentials": "HBO_Credentials.txt",
    "Spotify_Credentials": "Spotify_Credentials.txt",
    "Twitch_Credentials": "Twitch_Credentials.txt",
    "Hulu_Credentials": "Hulu_Credentials.txt",
    "Crunchyroll_Credentials": "Crunchyroll_Credentials.txt",
    "Database_URLs": "Database_URLs.txt",
    "Private_Keys": "Private_Keys.txt",
    "JWT_Tokens": "JWT_Tokens.txt",
    "Bearer_Tokens": "Bearer_Tokens.txt",
    "Email_Pass_Combos": "Email_Pass_Combos.txt",
    "Stealer_Logs": "Stealer_Logs.txt",
    "Facebook_Tokens": "Facebook_Tokens.txt",
    "Twitter_Tokens": "Twitter_Tokens.txt",
    "Instagram_Tokens": "Instagram_Tokens.txt",
    "Generic_Secrets": "Generic_Secrets.txt",
}


class CredentialExtractor:
    """
    Extracts, categorizes, and deduplicates credentials from text.
    Outputs new secrets to categorized files.
    """
    
    def __init__(self, db_path: str = DB_PATH, output_dir: str = OUTPUT_DIR):
        self.db_path = db_path
        self.output_dir = Path(output_dir)
        self.excluded_secrets: Set[str] = set()
        self._load_excluded_secrets()
        self._ensure_output_dir()
        self._init_db()
    
    def _load_excluded_secrets(self):
        """Load server secrets to exclude from extraction"""
        # Load from file
        if os.path.exists(EXCLUDED_SECRETS_FILE):
            try:
                with open(EXCLUDED_SECRETS_FILE, 'r') as f:
                    for line in f:
                        line = line.strip()
                        if line and not line.startswith('#'):
                            self.excluded_secrets.add(line)
                logger.info(f"Loaded {len(self.excluded_secrets)} excluded secrets")
            except Exception as e:
                logger.warning(f"Failed to load excluded secrets: {e}")
        
        # Load from environment variables (common secret env vars)
        secret_env_vars = [
            'TELEGRAM_API_ID', 'TELEGRAM_API_HASH',
            'SKYBIN_ADMIN_PASSWORD', 'ADMIN_PASSWORD',
            'DATABASE_URL', 'REDIS_URL',
            'AWS_ACCESS_KEY_ID', 'AWS_SECRET_ACCESS_KEY',
            'OPENAI_API_KEY', 'ANTHROPIC_API_KEY',
            'GITHUB_TOKEN', 'DISCORD_BOT_TOKEN',
        ]
        for var in secret_env_vars:
            val = os.getenv(var)
            if val:
                self.excluded_secrets.add(val)
    
    def _ensure_output_dir(self):
        """Create output directory if it doesn't exist"""
        self.output_dir.mkdir(parents=True, exist_ok=True)
        logger.info(f"Output directory: {self.output_dir}")
    
    def _init_db(self):
        """Initialize seen_secrets table if it doesn't exist"""
        try:
            conn = sqlite3.connect(self.db_path)
            conn.execute('''
                CREATE TABLE IF NOT EXISTS seen_secrets (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    secret_hash TEXT NOT NULL UNIQUE,
                    secret_type TEXT NOT NULL,
                    first_seen INTEGER NOT NULL,
                    last_seen INTEGER NOT NULL,
                    occurrence_count INTEGER DEFAULT 1,
                    source TEXT
                )
            ''')
            conn.execute('CREATE INDEX IF NOT EXISTS idx_seen_secrets_hash ON seen_secrets(secret_hash)')
            conn.commit()
            conn.close()
        except Exception as e:
            logger.error(f"Failed to initialize database: {e}")
    
    def _is_excluded(self, value: str) -> bool:
        """Check if a secret value should be excluded"""
        if not value:
            return True
        # Check against excluded secrets
        for excluded in self.excluded_secrets:
            if excluded in value or value in excluded:
                return True
        return False
    
    def _is_seen(self, secret_hash: str) -> bool:
        """Check if a secret has been seen before"""
        try:
            conn = sqlite3.connect(self.db_path)
            cursor = conn.execute(
                'SELECT id FROM seen_secrets WHERE secret_hash = ?',
                (secret_hash,)
            )
            result = cursor.fetchone()
            conn.close()
            return result is not None
        except Exception as e:
            logger.error(f"Database error checking seen secret: {e}")
            return False
    
    def _mark_seen(self, secret: ExtractedSecret, source: str = "telegram"):
        """Mark a secret as seen in the database"""
        try:
            now = int(datetime.now().timestamp())
            conn = sqlite3.connect(self.db_path)
            
            # Try to update existing
            cursor = conn.execute(
                '''UPDATE seen_secrets 
                   SET last_seen = ?, occurrence_count = occurrence_count + 1
                   WHERE secret_hash = ?''',
                (now, secret.hash)
            )
            
            # If not updated, insert new
            if cursor.rowcount == 0:
                conn.execute(
                    '''INSERT INTO seen_secrets 
                       (secret_hash, secret_type, first_seen, last_seen, source)
                       VALUES (?, ?, ?, ?, ?)''',
                    (secret.hash, secret.secret_type, now, now, source)
                )
            
            conn.commit()
            conn.close()
        except Exception as e:
            logger.error(f"Database error marking secret seen: {e}")
    
    def extract(self, content: str, source: str = "unknown") -> ExtractionResult:
        """
        Extract all secrets from content.
        Returns ExtractionResult with categorized secrets.
        """
        result = ExtractionResult()
        seen_values: Set[str] = set()  # Track values to avoid duplicates within same content
        
        lines = content.split('\n')
        
        for category, patterns in SECRET_PATTERNS.items():
            for secret_type, pattern in patterns.items():
                for line_num, line in enumerate(lines, 1):
                    for match in pattern.finditer(line):
                        # Handle groups - get the first non-None group or full match
                        groups = match.groups()
                        if groups:
                            value = next((g for g in groups if g), match.group(0))
                        else:
                            value = match.group(0)
                        
                        if not value or len(value) < 8:
                            continue
                        
                        # Skip if already processed in this content
                        if value in seen_values:
                            continue
                        seen_values.add(value)
                        
                        # Create secret object
                        secret = ExtractedSecret(
                            secret_type=secret_type,
                            category=category,
                            value=value,
                            context=line[:200] if len(line) > 200 else line,
                            line_number=line_num
                        )
                        
                        result.secrets.append(secret)
                        result.categories[category].append(secret)
                        
                        # Check exclusion
                        if self._is_excluded(value):
                            result.excluded_secrets.append(secret)
                            continue
                        
                        # Check deduplication
                        if self._is_seen(secret.hash):
                            result.duplicate_secrets.append(secret)
                        else:
                            result.new_secrets.append(secret)
                            self._mark_seen(secret, source)
        
        return result
    
    def write_to_files(self, result: ExtractionResult, prepend_timestamp: bool = True):
        """
        Write new secrets to their respective category files.
        Only writes NEW secrets (not duplicates).
        """
        if not result.new_secrets:
            logger.info("No new secrets to write")
            return
        
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        
        # Group new secrets by category
        by_category: Dict[str, List[ExtractedSecret]] = defaultdict(list)
        for secret in result.new_secrets:
            by_category[secret.category].append(secret)
        
        for category, secrets in by_category.items():
            filename = CATEGORY_FILES.get(category, f"{category}.txt")
            filepath = self.output_dir / filename
            
            try:
                with open(filepath, 'a', encoding='utf-8') as f:
                    if prepend_timestamp:
                        f.write(f"\n# --- {timestamp} ---\n")
                    
                    for secret in secrets:
                        # Write format: TYPE | VALUE | CONTEXT
                        f.write(f"{secret.secret_type} | {secret.value}\n")
                
                logger.info(f"Wrote {len(secrets)} secrets to {filename}")
            except Exception as e:
                logger.error(f"Failed to write to {filepath}: {e}")
    
    def get_summary(self, result: ExtractionResult) -> str:
        """Generate a summary string for the extraction result"""
        if not result.secrets:
            return ""
        
        parts = []
        for category, secrets in sorted(result.categories.items()):
            if secrets:
                # Count new vs duplicate
                new_count = sum(1 for s in secrets if s in result.new_secrets)
                if new_count > 0:
                    parts.append(f"{new_count}x {category.replace('_', ' ')}")
        
        return ", ".join(parts[:5])  # Limit to 5 categories in summary
    
    def get_stats(self) -> Dict[str, int]:
        """Get statistics about seen secrets"""
        try:
            conn = sqlite3.connect(self.db_path)
            cursor = conn.execute(
                '''SELECT secret_type, COUNT(*) as count 
                   FROM seen_secrets 
                   GROUP BY secret_type 
                   ORDER BY count DESC'''
            )
            stats = {row[0]: row[1] for row in cursor.fetchall()}
            conn.close()
            return stats
        except Exception as e:
            logger.error(f"Failed to get stats: {e}")
            return {}


# =============================================================================
# CONVENIENCE FUNCTIONS
# =============================================================================

_extractor: Optional[CredentialExtractor] = None

def get_extractor() -> CredentialExtractor:
    """Get singleton extractor instance"""
    global _extractor
    if _extractor is None:
        _extractor = CredentialExtractor()
    return _extractor


def extract_and_save(content: str, source: str = "telegram") -> Tuple[ExtractionResult, str]:
    """
    Extract secrets from content, save new ones to files, return result and summary.
    This is the main entry point for the telegram scraper.
    """
    extractor = get_extractor()
    result = extractor.extract(content, source)
    
    if result.new_secrets:
        extractor.write_to_files(result)
    
    summary = extractor.get_summary(result)
    return result, summary


def extract_credential_summary(content: str, max_samples: int = 10) -> Tuple[str, str]:
    """
    Legacy compatibility function.
    Returns (title, header) tuple for prepending to content.
    """
    extractor = get_extractor()
    result = extractor.extract(content)
    
    # Save new secrets to files
    if result.new_secrets:
        extractor.write_to_files(result)
    
    if not result.secrets:
        return ("", "")
    
    # Build title (counts only)
    title_parts = []
    for category, secrets in sorted(result.categories.items()):
        if secrets:
            title_parts.append(f"{len(secrets)}x {category.replace('_', ' ')}")
    
    title = ", ".join(title_parts[:4])
    
    # Build header
    header_lines = [
        "=" * 60,
        "CREDENTIAL SUMMARY",
        "=" * 60,
        f"Total: {result.total_count} secrets ({result.new_count} new, {len(result.duplicate_secrets)} seen before)",
        ""
    ]
    
    for category, secrets in sorted(result.categories.items()):
        if secrets:
            header_lines.append(f"{category.replace('_', ' ').upper()} ({len(secrets)} total):")
            for secret in secrets[:max_samples]:
                # Mask the value
                val = secret.value
                if len(val) > 20:
                    masked = f"{val[:8]}...{val[-8:]}"
                else:
                    masked = f"{val[:4]}...{val[-4:]}" if len(val) > 8 else val
                header_lines.append(f"  - {secret.secret_type}: {masked}")
            if len(secrets) > max_samples:
                header_lines.append(f"  ... and {len(secrets) - max_samples} more")
            header_lines.append("")
    
    header_lines.extend([
        "=" * 60,
        "                    FULL CONTENT BELOW",
        "-" * 60,
        ""
    ])
    
    header = "\n".join(header_lines)
    return (title, header)


if __name__ == "__main__":
    # Test the extractor
    test_content = """
    AWS Key: AKIAIOSFODNN7EXAMPLE
    GitHub Token: ghp_abcdefghijklmnopqrstuvwxyz123456
    test@example.com:password123
    mongodb://user:pass@localhost:27017/db
    -----BEGIN RSA PRIVATE KEY-----
    MIIEowIBAAKCAQEA...
    -----END RSA PRIVATE KEY-----
    """
    
    result, summary = extract_and_save(test_content, "test")
    print(f"Summary: {summary}")
    print(f"Total: {result.total_count}")
    print(f"New: {result.new_count}")
    print(f"Duplicates: {len(result.duplicate_secrets)}")
