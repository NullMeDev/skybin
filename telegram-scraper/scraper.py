#!/usr/bin/env python3
"""
SkyBin Telegram Scraper Service v3.0

Features:
- 12GB file downloads (streaming to disk for large files)
- Credential-only posting (skips messages without actual credentials)
- Count-based titles ("3x Netflix, 5x API Key") - NO channel names
- Multi-worker parallel pipeline (5 concurrent downloads for 80GB server)
- External file host support (gofile, pixeldrain, mega, mediafire, etc)
- Archive extraction (zip/rar) - password files only
- Strict credential detection (no keyword-only matching)
- Real-time channel monitoring + historical scraping
- Rate limit compliance (Telegram, external hosts)
"""

import asyncio
import re
import os
import io
import json
import logging
import aiohttp
import tempfile
import zipfile
from datetime import datetime, timedelta
from dotenv import load_dotenv

# Try to import rarfile for .rar support
try:
    import rarfile
    HAS_RARFILE = True
except ImportError:
    HAS_RARFILE = False
    logging.warning("rarfile not installed - .rar extraction disabled. Install with: pip install rarfile")
from telethon import TelegramClient, events, functions
from telethon.tl.types import Channel, Chat, User, InputPeerChannel, DocumentAttributeFilename
from telethon.tl.functions.channels import JoinChannelRequest
from telethon.tl.functions.messages import ImportChatInviteRequest
from telethon.errors import (
    UsernameNotOccupiedError, 
    UsernameInvalidError, 
    ChannelPrivateError,
    FloodWaitError,
    InviteHashExpiredError,
    InviteHashInvalidError,
    UserAlreadyParticipantError
)

# Load .env file
load_dotenv()

# Import our credential extractor for categorized extraction and deduplication
try:
    from credential_extractor import (
        extract_and_save,
        extract_credential_summary as new_extract_credential_summary,
        get_extractor
    )
    HAS_CREDENTIAL_EXTRACTOR = True
except ImportError:
    HAS_CREDENTIAL_EXTRACTOR = False
    logging.warning("credential_extractor not found - using legacy extraction")

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Configuration from environment
API_ID = os.getenv('TELEGRAM_API_ID')
API_HASH = os.getenv('TELEGRAM_API_HASH')
SKYBIN_API = os.getenv('SKYBIN_API_URL', 'http://localhost:8082')
SESSION_NAME = os.getenv('SESSION_NAME', 'skybin_scraper')

# Known active leak channels (2024-2025) - mix of usernames and invite links
# Sourced from: SOCRadar, Webz.io, Troy Hunt, KELA, Group-IB, 8BitSecurity research
KNOWN_CHANNELS = [
    # Public channels (by username) - CORE HIGH-VOLUME
    'Daisy_Cloud',              # 34M+ accounts - veteran since 2021
    'snatch_cloud',             # SNATCH LOGS CLOUD - active redistributor
    'MoonCombo',                # Moon Cloud - aggregator hub
    'bugatti_cloud',            # 16M+ accounts - established
    'cuckoo_cloud',             # 14M+ accounts - newer (2025)
    'redcloud_logs',            # 5.3M+ accounts - frequently blocked
    'StarLinkCloud',            # 2.9M+ accounts - veteran
    'observer_cloud',           # Long-standing, free logs
    'OmegaCloud_FreeLogs',      # Omega Cloud
    # ADDITIONAL CHANNELS FROM RESEARCH
    'LOG_SYNC',                 # Hybrid free/premium
    'baseleakz',                # BaseLeak - active redistributor
    'HUBHEAD_LOGS',             # VIP snatch room
    'Zeuscloudfree',            # Zeus Cloud
    'Wooden_Cloud',             # Active logs cloud
    'MariaLogs',                # Maria Logs
    'MOONLOGSFREE',             # Moon Logs Free channel
    'EnotLogs',                 # Enot Logs
    'PremCloud',                # Premium Cloud
    'bender_cloud',             # Bender Cloud
    'HelloKittyCloud',          # Hello Kitty Cloud
    'brutelogs',                # Brute Logs
    'bradmax_cloud',            # BradMax Cloud
    'sigmcloud',                # Sigma Cloud
    'XIII_FREE_LOGS',           # XIII Free Logs
    'smokercloud',              # Smoker Cloud - targeted dev creds
    'freshlogs1only',           # Fresh Logs
    'segacloud',                # Sega Cloud
    'datacloudspace',           # Data Cloud Space
    'joker_reborn',             # Joker Reborn
    'logsdiller_notify',        # Logs Diller
    'RedlineClouds',            # Redline Clouds
    'cloudxlog',                # CloudX Logs
    'ALIEN_TXTBASE',            # ALIEN TXTBASE
    'alpincloud',               # Alpin Cloud
    'akatsukilogs',             # Akatsuki Logs
    'reapercloud',              # Reaper Cloud
    # Invite links (just the hash part)
    'IqEnwfj7CLU1Yjcy',         # Omega Cloud
    't9UsKiIPzJthMGIy',         # Omega Cloud Free Logs
    'oXn6deop_VVhOWIy',         # Moon Cloud ULP/Combo
]

# Credential detection patterns - FOCUSED on actual leaks
# Excludes: Stripe checkout links, payment processing tokens
CREDENTIAL_PATTERNS = [
    # Username:password and email:password combos (PRIMARY)
    re.compile(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,}'),  # email:pass
    re.compile(r'(?i)(?:user(?:name)?|login)\s*[=:]\s*[^\s]{3,}\s+(?:pass(?:word)?|pwd)\s*[=:]\s*[^\s]{4,}'),  # user:pass format
    re.compile(r'\b[a-zA-Z0-9_.+-]+:[^\s@:]{6,}(?:\s|$)'),  # username:password
    
    # URL:login:pass (stealer logs)
    re.compile(r'https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}'),
    
    # API keys (excluding Stripe)
    re.compile(r'ghp_[a-zA-Z0-9]{36}'),  # GitHub PAT
    re.compile(r'gho_[a-zA-Z0-9]{36}'),  # GitHub OAuth
    re.compile(r'github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59}'),  # GitHub Fine-grained
    re.compile(r'sk-[a-zA-Z0-9]{48}'),  # OpenAI (not sk_live/sk_test)
    re.compile(r'sk-proj-[a-zA-Z0-9-_]{80,}'),  # OpenAI Project
    re.compile(r'AKIA[0-9A-Z]{16}'),  # AWS Access Key
    
    # Database connection strings
    re.compile(r'(?i)mongodb(\+srv)?://[^\s]+'),
    re.compile(r'(?i)postgres(ql)?://[^\s]+'),
    re.compile(r'(?i)mysql://[^\s]+'),
    re.compile(r'(?i)redis://[^\s]+'),
    
    # Private keys
    re.compile(r'-----BEGIN (RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----'),
    
    # Communication platform tokens
    re.compile(r'xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+'),  # Slack
    re.compile(r'[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27}'),  # Discord bot
    re.compile(r'[0-9]{17,19}:[A-Za-z0-9_-]{35}'),  # Telegram bot
    
    # Other services (non-payment)
    re.compile(r'SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}'),  # SendGrid
    re.compile(r'AIza[0-9A-Za-z_-]{35}'),  # Firebase/Google
]

# Patterns to EXCLUDE (Stripe checkout URLs only - NOT API keys)
EXCLUDE_PATTERNS = [
    re.compile(r'(?i)checkout\.stripe\.com'),
    re.compile(r'(?i)buy\.stripe\.com'),
    re.compile(r'(?i)stripe\.com/links'),
    re.compile(r'(?i)stripe\.com/pay'),
    # Block checkout session URLs but not the keys themselves
    re.compile(r'(?i)https?://[^\s]*cs_live_[a-zA-Z0-9]+'),
    re.compile(r'(?i)https?://[^\s]*cs_test_[a-zA-Z0-9]+'),
]

# Minimum counts for credential detection
MIN_EMAIL_PASS_COMBOS = 1  # Accept even single email:pass
MIN_CREDENTIAL_PATTERNS = 1  # Accept single API key/token

# File extensions that indicate credential files (downloadable)
CRED_FILE_EXTENSIONS = ['.txt', '.csv', '.json', '.sql', '.log', '.env']

# Archive extensions (will be extracted)
ARCHIVE_EXTENSIONS = ['.zip', '.rar']

# Password file patterns (case-insensitive) - ONLY extract these from archives
PASSWORD_FILE_PATTERNS = [
    'passwords.txt', 'password.txt', 'pass.txt', 'pwd.txt',
    'logins.txt', 'credentials.txt', 'combo.txt', 'accounts.txt',
    'all passwords.txt', 'all_passwords.txt', 'allpasswords.txt',
    'passwords', 'password', 'logins', 'credentials',
]

# External file host patterns - extract download links from messages
FILE_HOST_PATTERNS = [
    # Gofile
    re.compile(r'https?://(?:www\.)?gofile\.io/d/([a-zA-Z0-9]+)'),
    # Pixeldrain
    re.compile(r'https?://(?:www\.)?pixeldrain\.com/(?:u|l)/([a-zA-Z0-9]+)'),
    # Mega.nz
    re.compile(r'https?://mega\.nz/(?:file|folder)/([a-zA-Z0-9_-]+)(?:#([a-zA-Z0-9_-]+))?'),
    # MediaFire
    re.compile(r'https?://(?:www\.)?mediafire\.com/file/([a-zA-Z0-9]+)/([^/\s]+)'),
    # Catbox
    re.compile(r'https?://files\.catbox\.moe/([a-zA-Z0-9]+\.[a-z]+)'),
    # Litterbox
    re.compile(r'https?://litter\.catbox\.moe/([a-zA-Z0-9]+\.[a-z]+)'),
    # Krakenfiles
    re.compile(r'https?://(?:www\.)?krakenfiles\.com/view/([a-zA-Z0-9]+)'),
    # Workupload
    re.compile(r'https?://workupload\.com/file/([a-zA-Z0-9]+)'),
    # Buzzheavier
    re.compile(r'https?://buzzheavier\.com/([a-zA-Z0-9]+)'),
    # 1fichier
    re.compile(r'https?://1fichier\.com/\?([a-zA-Z0-9]+)'),
    # Uploadhaven
    re.compile(r'https?://uploadhaven\.com/download/([a-zA-Z0-9]+)'),
    # Sendspace
    re.compile(r'https?://(?:www\.)?sendspace\.com/file/([a-zA-Z0-9]+)'),
    # Zippyshare replacements
    re.compile(r'https?://(?:www\.)?(send\.cm|send-cm\.com)/([a-zA-Z0-9]+)'),
    # Generic direct download links (archives)
    re.compile(r'https?://[^\s]+\.(zip|rar|7z|tar\.gz)(?:\?[^\s]*)?'),
]

# Max file size for regular files (5MB)
MAX_FILE_SIZE = 5 * 1024 * 1024

# Max file size for archives (20GB - server has 120GB disk total, 16GB RAM)
# Large archives are streamed to disk to avoid memory issues
# With 120GB disk (80GB main + 40GB volume), can handle ~6-8 simultaneous 20GB archives
# Temp files stored on dedicated 40GB volume at /opt/skybin/telegram-cache
MAX_ARCHIVE_SIZE = 20 * 1024 * 1024 * 1024  # 20GB

# Download timeout in seconds (1 hour max per file for large downloads)
DOWNLOAD_TIMEOUT = 3600

# Concurrent file downloads limit (tuned for 16GB RAM / 8vCPU / 120GB disk)
# With dedicated 40GB volume for temp files, can handle more simultaneous large archives
MAX_CONCURRENT_DOWNLOADS = 18  # Up from 12: 1.5GB peak per download, bottleneck is now RAM

# Number of concurrent post workers (SkyBin API uploads)
NUM_POST_WORKERS = 6  # Up from 3: faster queue draining

# Number of concurrent file download workers (archive extraction is CPU-bound)
NUM_FILE_WORKERS = 10  # Up from 5: utilize 8 vCPU better

# Number of concurrent link download workers (external hosts: gofile, pixeldrain, etc)
NUM_LINK_WORKERS = 8  # Up from 5: network I/O bound

# Temp file prefix for cleanup
TEMP_FILE_PREFIX = 'skybin_tg_'

# Temp directory for large file downloads (use dedicated volume if available)
# Falls back to system temp if volume not mounted
TEMP_DIR = os.getenv('TELEGRAM_TEMP_DIR', '/opt/skybin/telegram-cache') 
if not os.path.exists(TEMP_DIR) or not os.access(TEMP_DIR, os.W_OK):
    TEMP_DIR = tempfile.gettempdir()  # Fallback to /tmp
    logger.warning(f"Dedicated volume not available, using {TEMP_DIR} for temp files")
else:
    logger.info(f"Using dedicated volume {TEMP_DIR} for temp files")

def should_exclude(text: str) -> bool:
    """Check if text should be excluded (Stripe checkout links, etc.)"""
    if not text:
        return False
    return any(pattern.search(text) for pattern in EXCLUDE_PATTERNS)

def contains_credentials(text: str) -> bool:
    """Check if text contains potential credentials (excluding payment stuff)"""
    if not text:
        return False
    if should_exclude(text):
        return False
    return any(pattern.search(text) for pattern in CREDENTIAL_PATTERNS)

# Leak-related keywords for additional filtering (focused on credential leaks)
LEAK_KEYWORDS = [
    # Core credential terms
    'leak', 'leaked', 'dump', 'dumped', 'combo', 'combolist', 'breach', 'breached',
    'crack', 'cracked', 'hacked', 'stolen', 'exposed', 'database', 'db dump',
    'credential', 'password', 'passwd', 'login', 'account',
    # Stealer/log references
    'stealer', 'infostealer', 'redline', 'raccoon', 'lumma', 'vidar', 'stealc',
    'log', 'logs', 'fresh logs', 'cloud logs',
    # Credential formats
    'email:pass', 'user:pass', 'mail:pass', 'url:login:pass', 'ulp',
    # Card related (fullz/cvv only - BIN detection is pattern-based below)
    'fullz', 'cvv', 'credit card', 'card dump',
    # Service accounts
    'netflix', 'spotify', 'disney', 'hbo', 'amazon', 'prime', 'vpn', 'nord',
    'express', 'steam', 'fortnite', 'minecraft', 'roblox', 'epic', 'origin',
    'gmail', 'yahoo', 'outlook', 'hotmail', 'proton', 'mail.ru',
    # Technical credentials
    'api key', 'apikey', 'secret', 'private key', 'ssh', 'ftp', 'smtp',
    'cpanel', 'rdp', 'shell', 'root', 'admin', 'panel',
    # Quality indicators
    'fresh', 'valid', 'checked', 'hits', 'capture', 'working', 'premium',
]

# Keywords that indicate we should SKIP the message
SKIP_KEYWORDS = [
    'checkout.stripe.com', 'buy.stripe.com', 'stripe.com/links',
    'payment link', 'pay now', 'subscribe now', 'buy now',
    'donation', 'donate', 'tip jar', 'ko-fi', 'patreon',
]

def has_actual_bin_data(text: str) -> bool:
    """
    Check for actual BIN/card data patterns, not just keywords.
    BINs are 6-8 digit bank identification numbers.
    """
    # Look for actual BIN list patterns:
    # - Multiple 6-digit numbers on separate lines
    # - Card number patterns (13-19 digits)
    # - CVV patterns (3-4 digits after card info)
    # - Expiry patterns (MM/YY or MM/YYYY)
    
    # Count lines that look like BIN entries (6+ digits at start)
    bin_line_pattern = re.compile(r'^\s*\d{6,8}\b', re.MULTILINE)
    bin_lines = len(bin_line_pattern.findall(text))
    
    # Count full card number patterns (13-19 digits, possibly with spaces/dashes)
    card_pattern = re.compile(r'\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{1,7}\b')
    card_count = len(card_pattern.findall(text))
    
    # Count CVV patterns (3-4 digits after pipe or tab, common in dumps)
    cvv_pattern = re.compile(r'[|\t]\s*\d{3,4}\s*$', re.MULTILINE)
    cvv_count = len(cvv_pattern.findall(text))
    
    # Count expiry date patterns
    expiry_pattern = re.compile(r'\b(0[1-9]|1[0-2])[/\-](2[4-9]|[3-9]\d|\d{4})\b')
    expiry_count = len(expiry_pattern.findall(text))
    
    # Require multiple actual BIN/card patterns, not just keywords
    return (bin_lines >= 5 or card_count >= 3 or 
            (cvv_count >= 2 and expiry_count >= 2))

def is_leak_content(text: str) -> bool:
    """
    Credential detection - requires actual credentials OR strong keyword signals.
    Excludes Stripe checkout links and payment-related content.
    """
    if not text or len(text) < 50:
        return False
    
    lower = text.lower()
    
    # SKIP if contains payment/checkout keywords
    if any(skip_kw in lower for skip_kw in SKIP_KEYWORDS):
        return False
    
    # SKIP if matches exclude patterns (Stripe links, etc.)
    if should_exclude(text):
        return False
    
    # Count email:password combinations
    email_pass_pattern = re.compile(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,}')
    email_pass_count = len(email_pass_pattern.findall(text))
    
    # Count username:password combos (non-email)
    user_pass_pattern = re.compile(r'\b[a-zA-Z0-9_.-]{3,}:[^\s@:]{6,}(?:\s|$)')
    user_pass_count = len(user_pass_pattern.findall(text))
    
    # Count URL:login:pass format (stealer logs)
    ulp_pattern = re.compile(r'https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}')
    ulp_count = len(ulp_pattern.findall(text))
    
    # Count actual credential patterns (API keys, tokens, etc)
    cred_pattern_count = sum(1 for pattern in CREDENTIAL_PATTERNS if pattern.search(text))
    
    # Check for private keys
    has_private_key = '-----BEGIN' in text and 'PRIVATE KEY-----' in text
    
    # Check for leak keywords
    keyword_count = sum(1 for kw in LEAK_KEYWORDS if kw in lower)
    
    # Check for ACTUAL BIN/card data patterns (not just keywords)
    has_bin_data = has_actual_bin_data(text)
    
    # STRICT: Accept ONLY if actual credentials are present
    # NO keyword-only detection - removed as it was too loose
    return (
        email_pass_count >= MIN_EMAIL_PASS_COMBOS or
        user_pass_count >= 1 or
        ulp_count >= 1 or
        cred_pattern_count >= MIN_CREDENTIAL_PATTERNS or
        has_private_key or
        has_bin_data
    )


def is_password_file(filename: str) -> bool:
    """
    Check if a filename matches password file patterns.
    Used to filter which files to extract from archives.
    """
    lower = filename.lower()
    base = os.path.basename(lower)
    
    for pattern in PASSWORD_FILE_PATTERNS:
        if pattern in base:
            return True
    
    return False


def extract_file_host_links(text: str) -> list:
    """
    Extract download links from file hosting services.
    Returns list of (url, host_name) tuples.
    """
    if not text:
        return []
    
    links = []
    for pattern in FILE_HOST_PATTERNS:
        for match in pattern.finditer(text):
            url = match.group(0)
            # Identify the host
            if 'gofile' in url:
                host = 'gofile'
            elif 'pixeldrain' in url:
                host = 'pixeldrain'
            elif 'mega.nz' in url:
                host = 'mega'
            elif 'mediafire' in url:
                host = 'mediafire'
            elif 'catbox' in url:
                host = 'catbox'
            elif 'krakenfiles' in url:
                host = 'krakenfiles'
            elif 'workupload' in url:
                host = 'workupload'
            elif 'buzzheavier' in url:
                host = 'buzzheavier'
            elif '1fichier' in url:
                host = '1fichier'
            elif 'uploadhaven' in url:
                host = 'uploadhaven'
            elif 'sendspace' in url:
                host = 'sendspace'
            elif 'send.cm' in url or 'send-cm' in url:
                host = 'sendcm'
            else:
                host = 'direct'
            
            links.append((url, host))
    
    return links

def extract_credential_summary(content: str, max_samples: int = 10) -> tuple:
    """
    Extract and summarize critical credential information from content.
    Now uses the new credential_extractor module for categorization, deduplication,
    and automatic output to category files (AWS_Keys.txt, Discord_Tokens.txt, etc.).
    
    Returns (summary_title, summary_header) tuple.
    - summary_title: Short title like "2x API Key, 3x Email:Pass"
    - summary_header: Full formatted header to prepend to paste
    """
    # Use new extractor if available (handles deduplication and file output)
    if HAS_CREDENTIAL_EXTRACTOR:
        try:
            return new_extract_credential_summary(content, max_samples)
        except Exception as e:
            logger.warning(f"New extractor failed, falling back to legacy: {e}")
    
    # Legacy extraction (fallback)
    summary_parts = []
    title_parts = []
    
    # Extract email:password combos (limit samples)
    email_pass_pattern = re.compile(r'([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,})')
    email_passes = email_pass_pattern.findall(content)
    if email_passes:
        unique_emails = list(set(email_passes))[:max_samples]
        summary_parts.append(f"EMAIL:PASS COMBOS ({len(email_passes)} total, showing {len(unique_emails)}):")
        for ep in unique_emails:
            summary_parts.append(f"  - {ep}")
        title_parts.append(f"{len(email_passes)}x Email:Pass")
    
    # Extract URL:login:pass (stealer logs)
    ulp_pattern = re.compile(r'(https?://[^\s]+)[\s\t|:]+([^\s@]+)[\s\t|:]+([^\s]{4,})')
    ulps = ulp_pattern.findall(content)
    if ulps:
        unique_ulps = list(set(ulps))[:max_samples]
        summary_parts.append(f"\nURL:LOGIN:PASS ({len(ulps)} total, showing {len(unique_ulps)}):")
        for url, login, pwd in unique_ulps:
            display_url = url[:50] + "..." if len(url) > 50 else url
            summary_parts.append(f"  - {display_url} | {login} | {pwd}")
        title_parts.append(f"{len(ulps)}x URL:Login:Pass")
    
    # Extract API keys
    api_patterns = [
        (r'(ghp_[a-zA-Z0-9]{36})', 'GitHub PAT'),
        (r'(gho_[a-zA-Z0-9]{36})', 'GitHub OAuth'),
        (r'(github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59})', 'GitHub Fine-grained'),
        (r'(sk-[a-zA-Z0-9]{48})', 'OpenAI'),
        (r'(sk-proj-[a-zA-Z0-9-_]{80,})', 'OpenAI Project'),
        (r'(AKIA[0-9A-Z]{16})', 'AWS Access Key'),
        (r'(AIza[0-9A-Za-z_-]{35})', 'Firebase/Google'),
        (r'(SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43})', 'SendGrid'),
        (r'(xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+)', 'Slack'),
    ]
    
    api_keys_found = []
    for pattern, key_type in api_patterns:
        matches = re.findall(pattern, content)
        for match in matches[:3]:
            api_keys_found.append((key_type, match))
    
    if api_keys_found:
        summary_parts.append(f"\nAPI KEYS/TOKENS ({len(api_keys_found)} total):")
        for key_type, key in api_keys_found[:max_samples]:
            if len(key) > 20:
                masked = key[:8] + "..." + key[-8:]
            else:
                masked = key[:4] + "..." + key[-4:]
            summary_parts.append(f"  - {key_type}: {masked}")
        title_parts.append(f"{len(api_keys_found)}x API Key")
    
    # Extract Discord tokens
    discord_pattern = re.compile(r'([MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27})')
    discord_tokens = discord_pattern.findall(content)
    if discord_tokens:
        summary_parts.append(f"\nDISCORD TOKENS ({len(discord_tokens)} total):")
        for token in discord_tokens[:max_samples]:
            masked = token[:10] + "..." + token[-10:]
            summary_parts.append(f"  - {masked}")
        title_parts.append(f"{len(discord_tokens)}x Discord Token")
    
    # Extract Telegram bot tokens
    tg_pattern = re.compile(r'([0-9]{8,10}:[A-Za-z0-9_-]{35})')
    tg_tokens = tg_pattern.findall(content)
    if tg_tokens:
        summary_parts.append(f"\nTELEGRAM BOT TOKENS ({len(tg_tokens)} total):")
        for token in tg_tokens[:max_samples]:
            masked = token[:8] + "..." + token[-8:]
            summary_parts.append(f"  - {masked}")
        title_parts.append(f"{len(tg_tokens)}x TG Bot Token")
    
    # Extract database connection strings
    db_patterns = [
        (r'(mongodb(?:\+srv)?://[^\s]+)', 'MongoDB'),
        (r'(postgres(?:ql)?://[^\s]+)', 'PostgreSQL'),
        (r'(mysql://[^\s]+)', 'MySQL'),
        (r'(redis://[^\s]+)', 'Redis'),
    ]
    
    db_strings = []
    for pattern, db_type in db_patterns:
        matches = re.findall(pattern, content, re.IGNORECASE)
        for match in matches[:2]:
            db_strings.append((db_type, match))
    
    if db_strings:
        summary_parts.append(f"\nDATABASE CONNECTIONS ({len(db_strings)} total):")
        for db_type, conn in db_strings[:max_samples]:
            if len(conn) > 60:
                display = conn[:30] + "..." + conn[-15:]
            else:
                display = conn
            summary_parts.append(f"  - {db_type}: {display}")
        title_parts.append(f"{len(db_strings)}x DB Conn")
    
    # Extract private keys indicator
    if '-----BEGIN' in content and 'PRIVATE KEY-----' in content:
        key_types = []
        if 'RSA PRIVATE KEY' in content:
            key_types.append('RSA')
        if 'DSA PRIVATE KEY' in content:
            key_types.append('DSA')
        if 'EC PRIVATE KEY' in content:
            key_types.append('EC')
        if 'OPENSSH PRIVATE KEY' in content:
            key_types.append('OpenSSH')
        if 'PGP PRIVATE KEY' in content:
            key_types.append('PGP')
        if not key_types:
            key_types.append('Unknown')
        
        summary_parts.append(f"\nPRIVATE KEYS: {', '.join(key_types)}")
        title_parts.append(f"{len(key_types)}x Private Key")
    
    if not summary_parts:
        return "", ""
    
    # Build the summary title
    summary_title = ", ".join(title_parts[:4])  # Max 4 items in title
    
    # Build the summary header
    header = "="*60 + "\n"
    header += "CREDENTIAL SUMMARY\n"
    header += "="*60 + "\n"
    header += "\n".join(summary_parts)
    header += "\n" + "="*60 + "\n"
    header += "\n" + " "*20 + "FULL CONTENT BELOW\n"
    header += "-"*60 + "\n\n"
    
    return summary_title, header


def generate_auto_title(content: str, channel_name: str = "") -> str:
    """
    Generate a credential summary title based on content analysis.
    Returns a title like "3x Netflix, 5x API Key, 12x Email:Pass" - NO channel name.
    """
    lower = content.lower()
    counts = {}  # service/type -> count
    
    # Count services found
    service_keywords = {
        'netflix': 'Netflix', 'spotify': 'Spotify', 'disney': 'Disney+',
        'hbo': 'HBO', 'amazon': 'Amazon', 'prime': 'Prime', 'steam': 'Steam',
        'hulu': 'Hulu', 'apple': 'Apple', 'icloud': 'iCloud',
        'fortnite': 'Fortnite', 'minecraft': 'Minecraft', 'roblox': 'Roblox',
        'gmail': 'Gmail', 'yahoo': 'Yahoo', 'outlook': 'Outlook',
        'paypal': 'PayPal', 'crypto': 'Crypto', 'nordvpn': 'NordVPN',
        'expressvpn': 'ExpressVPN', 'cpanel': 'cPanel', 'rdp': 'RDP',
        'github': 'GitHub', 'aws': 'AWS', 'mongodb': 'MongoDB',
        'google': 'Google', 'twitch': 'Twitch', 'tiktok': 'TikTok',
    }
    
    for keyword, name in service_keywords.items():
        # Count actual credential lines mentioning this service
        service_pattern = re.compile(rf'{keyword}[^\n]*:[^\s]{{4,}}', re.IGNORECASE)
        matches = service_pattern.findall(content)
        if matches:
            counts[name] = len(matches)
    
    # Count API keys by type
    api_patterns = {
        'GitHub PAT': r'ghp_[a-zA-Z0-9]{36}',
        'OpenAI Key': r'sk-[a-zA-Z0-9]{48}',
        'AWS Key': r'AKIA[0-9A-Z]{16}',
        'Google API': r'AIza[0-9A-Za-z_-]{35}',
        'Discord Token': r'[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27}',
        'Slack Token': r'xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+',
        'Private Key': r'-----BEGIN.*PRIVATE KEY-----',
    }
    
    for name, pattern in api_patterns.items():
        matches = re.findall(pattern, content)
        if matches:
            counts[name] = len(matches)
    
    # Count credential formats
    email_pass_count = len(re.findall(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,}', content))
    if email_pass_count > 0:
        counts['Email:Pass'] = email_pass_count
    
    url_login_count = len(re.findall(r'https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}', content))
    if url_login_count > 0:
        counts['URL:Login:Pass'] = url_login_count
    
    # Check for BIN/card data
    if has_actual_bin_data(content):
        counts['BINs/Cards'] = 1
    
    # Build title from counts - sorted by count descending
    if not counts:
        # Fallback: count lines as generic combos
        line_count = len([l for l in content.split('\n') if ':' in l and len(l.strip()) > 10])
        if line_count > 0:
            return f"{line_count}x Combos"
        return "Credential Leak"
    
    sorted_items = sorted(counts.items(), key=lambda x: x[1], reverse=True)
    title_parts = [f"{count}x {name}" for name, count in sorted_items[:4]]  # Max 4 items
    
    return ", ".join(title_parts)[:100]

async def post_to_skybin(content: str, base_title: str, source: str = "telegram"):
    """
    Post discovered content to SkyBin API with credential summary header.
    ONLY posts if actual credentials are found - skips empty/non-credential content.
    """
    if len(content) < 50:  # Lowered threshold
        return False
    
    # STRICT: Only post if content has actual credentials
    if not is_leak_content(content):
        logger.debug(f"Skipping post - no credentials found in content")
        return False
    
    # Generate credential summary - returns (summary_title, summary_header)
    summary_title, summary_header = extract_credential_summary(content)
    
    # STRICT: Skip if no credentials detected in summary (double-check)
    if not summary_title and not summary_header:
        logger.debug(f"Skipping post - credential extraction returned empty")
        return False
    
    # Build final title: Use summary title ("3x Netflix, 5x Email:Pass") or fallback
    if summary_title:
        final_title = summary_title
    else:
        # Generate title from content analysis (no channel name)
        final_title = generate_auto_title(content)
    
    # Prepend header to content
    if summary_header:
        final_content = summary_header + content
    else:
        final_content = content
        
    try:
        async with aiohttp.ClientSession() as session:
            payload = {
                'content': final_content[:100000],  # Increased limit
                'title': final_title[:100].strip(),
            }
            async with session.post(
                f'{SKYBIN_API}/api/paste',
                json=payload,
                timeout=aiohttp.ClientTimeout(total=30)
            ) as resp:
                if resp.status in (200, 201):
                    data = await resp.json()
                    paste_id = data.get('data', {}).get('id', 'unknown')
                    logger.info(f"✓ Posted to SkyBin: {paste_id}")
                    return True
                elif resp.status == 500 and 'UNIQUE' in await resp.text():
                    # Duplicate - already posted
                    return False
                else:
                    text = await resp.text()
                    logger.warning(f"Failed to post to SkyBin: {resp.status} - {text[:100]}")
                    return False
    except Exception as e:
        logger.error(f"Error posting to SkyBin: {e}")
        return False

def cleanup_orphaned_temp_files():
    """
    Clean up any orphaned temp files from previous runs.
    Called on startup to prevent disk bloat.
    """
    import glob
    
    temp_dir = tempfile.gettempdir()
    patterns = [
        os.path.join(temp_dir, 'tmp*.rar'),
        os.path.join(temp_dir, 'tmp*.zip'),
        os.path.join(temp_dir, f'{TEMP_FILE_PREFIX}*'),
    ]
    
    cleaned = 0
    for pattern in patterns:
        for filepath in glob.glob(pattern):
            try:
                # Only clean files older than 1 hour (to avoid cleaning active downloads)
                if os.path.isfile(filepath):
                    age = datetime.now().timestamp() - os.path.getmtime(filepath)
                    if age > 3600:  # 1 hour
                        os.unlink(filepath)
                        cleaned += 1
            except Exception as e:
                logger.debug(f"Could not clean {filepath}: {e}")
    
    if cleaned > 0:
        logger.info(f"🧹 Cleaned up {cleaned} orphaned temp files")


class TelegramScraper:
    def __init__(self):
        self.client = None
        self.processed_messages = set()
        self.processed_files = set()  # Track downloaded files by unique ID
        self.processed_links = set()  # Track processed external links
        self.active_channels = []
        self.post_queue = asyncio.Queue()
        self.file_queue = asyncio.Queue()  # Queue for file downloads
        self.link_queue = asyncio.Queue()  # Queue for external link downloads
        self.posts_made = 0
        self.files_downloaded = 0
        self.links_downloaded = 0
        self.channels_joined = 0
        self.download_semaphore = asyncio.Semaphore(MAX_CONCURRENT_DOWNLOADS)
        
    async def start(self):
        """Initialize and start the Telegram client"""
        if not API_ID or not API_HASH:
            logger.error("TELEGRAM_API_ID and TELEGRAM_API_HASH are required")
            return False
            
        self.client = TelegramClient(SESSION_NAME, int(API_ID), API_HASH)
        
        logger.info("Connecting to Telegram...")
        await self.client.connect()
        
        if not await self.client.is_user_authorized():
            logger.info("User not authorized. Starting authentication...")
            phone = input("Enter your phone number (with country code, e.g. +1234567890): ")
            await self.client.send_code_request(phone)
            code = input("Enter the code you received: ")
            try:
                await self.client.sign_in(phone, code)
            except Exception as e:
                if "Two-step verification" in str(e) or "password" in str(e).lower():
                    password = input("Enter your 2FA password: ")
                    await self.client.sign_in(password=password)
                else:
                    raise e
        
        me = await self.client.get_me()
        logger.info(f"Logged in as: {me.first_name} (@{me.username})")
        return True
    
    async def join_channel_by_username(self, username: str) -> bool:
        """Join a public channel by username"""
        try:
            await self.client(JoinChannelRequest(username))
            logger.info(f"  ✓ Joined @{username}")
            self.channels_joined += 1
            return True
        except UserAlreadyParticipantError:
            logger.debug(f"  Already in @{username}")
            return True
        except UsernameNotOccupiedError:
            logger.warning(f"  ✗ Channel @{username} not found")
            return False
        except UsernameInvalidError:
            logger.warning(f"  ✗ Invalid username: {username}")
            return False
        except ChannelPrivateError:
            logger.warning(f"  ✗ Channel @{username} is private")
            return False
        except FloodWaitError as e:
            logger.warning(f"  Rate limited, waiting {e.seconds}s")
            await asyncio.sleep(e.seconds)
            return await self.join_channel_by_username(username)
        except Exception as e:
            logger.error(f"  ✗ Error joining @{username}: {e}")
            return False
    
    async def join_channel_by_invite(self, invite_hash: str) -> bool:
        """Join a private channel by invite hash"""
        try:
            await self.client(ImportChatInviteRequest(invite_hash))
            logger.info(f"  ✓ Joined via invite: {invite_hash[:8]}...")
            self.channels_joined += 1
            return True
        except UserAlreadyParticipantError:
            logger.debug(f"  Already in channel (invite: {invite_hash[:8]}...)")
            return True
        except InviteHashExpiredError:
            logger.warning(f"  ✗ Invite expired: {invite_hash[:8]}...")
            return False
        except InviteHashInvalidError:
            logger.warning(f"  ✗ Invalid invite: {invite_hash[:8]}...")
            return False
        except FloodWaitError as e:
            logger.warning(f"  Rate limited, waiting {e.seconds}s")
            await asyncio.sleep(e.seconds)
            return await self.join_channel_by_invite(invite_hash)
        except Exception as e:
            logger.error(f"  ✗ Error with invite {invite_hash[:8]}...: {e}")
            return False

    async def join_known_channels(self):
        """Try to join all known leak channels"""
        logger.info(f"Attempting to join {len(KNOWN_CHANNELS)} known channels...")
        
        for channel in KNOWN_CHANNELS:
            # Check if it looks like an invite hash (long alphanumeric string)
            if len(channel) > 15 and not channel.startswith('@') and '/' not in channel:
                await self.join_channel_by_invite(channel)
            else:
                # It's a username
                username = channel.lstrip('@')
                await self.join_channel_by_username(username)
            
            # Rate limit between join attempts
            await asyncio.sleep(2)
        
        logger.info(f"Channel joining complete. Joined {self.channels_joined} new channels.")
    
    async def find_leak_channels(self):
        """Find ALL channels/groups from dialogs"""
        logger.info("Scanning all your channels and groups...")
        
        async for dialog in self.client.iter_dialogs():
            entity = dialog.entity
            
            # Process channels and groups (supergroups are also Channel type)
            if isinstance(entity, (Channel, Chat)):
                name = getattr(entity, 'title', '') or ''
                username = getattr(entity, 'username', '') or ''
                
                self.active_channels.append(entity)
                logger.info(f"  Found: {name} (@{username})" if username else f"  Found: {name}")
        
        logger.info(f"Total channels/groups to monitor: {len(self.active_channels)}")
        return self.active_channels
    
    async def stream_download_to_file(self, session: aiohttp.ClientSession, url: str, 
                                        headers: dict = None, fname: str = "unknown") -> str:
        """
        Stream download a large file to a temp file with progress logging.
        Returns the temp file path, or None on failure.
        Caller is responsible for cleanup.
        """
        tmp_path = None
        try:
            ext = os.path.splitext(fname)[1] or '.tmp'
            with tempfile.NamedTemporaryFile(suffix=ext, prefix=TEMP_FILE_PREFIX, delete=False, dir=TEMP_DIR) as tmp:
                tmp_path = tmp.name
            
            async with session.get(url, headers=headers, timeout=aiohttp.ClientTimeout(total=DOWNLOAD_TIMEOUT)) as resp:
                if resp.status != 200:
                    if tmp_path and os.path.exists(tmp_path):
                        os.unlink(tmp_path)
                    return None
                
                total_size = int(resp.headers.get('Content-Length', 0))
                downloaded = 0
                last_log = 0
                
                with open(tmp_path, 'wb') as f:
                    async for chunk in resp.content.iter_chunked(1024 * 1024):  # 1MB chunks
                        f.write(chunk)
                        downloaded += len(chunk)
                        
                        # Log progress every 100MB
                        if total_size > 0 and downloaded - last_log >= 100 * 1024 * 1024:
                            pct = (downloaded / total_size) * 100
                            logger.info(f"    📥 {fname}: {downloaded / 1024 / 1024:.0f}MB / {total_size / 1024 / 1024:.0f}MB ({pct:.1f}%)")
                            last_log = downloaded
                
                return tmp_path
                
        except Exception as e:
            logger.error(f"  Stream download error: {e}")
            if tmp_path and os.path.exists(tmp_path):
                os.unlink(tmp_path)
            return None

    async def extract_text_from_archive_file(self, file_path: str, filename: str, channel_name: str) -> int:
        """
        Extract ONLY password files from an archive file on disk and post them.
        Used for large archives that were streamed to disk.
        Returns number of files processed.
        """
        processed = 0
        lower_filename = filename.lower()
        
        try:
            if lower_filename.endswith('.zip'):
                with zipfile.ZipFile(file_path, 'r') as zf:
                    for info in zf.infolist():
                        if info.is_dir() or info.file_size > MAX_FILE_SIZE:
                            continue
                        if not is_password_file(info.filename):
                            continue
                        try:
                            data = zf.read(info.filename)
                            try:
                                content = data.decode('utf-8', errors='ignore')
                            except:
                                content = data.decode('latin-1', errors='ignore')
                            
                            if len(content) > 50 and is_leak_content(content):
                                title = generate_auto_title(content)
                                await self.post_queue.put((content, title))
                                processed += 1
                                logger.info(f"    🔑 Extracted password file: {info.filename}")
                        except Exception as e:
                            logger.debug(f"    Error extracting {info.filename}: {e}")
            
            elif lower_filename.endswith('.rar') and HAS_RARFILE:
                with rarfile.RarFile(file_path, 'r') as rf:
                    for info in rf.infolist():
                        if info.is_dir() or info.file_size > MAX_FILE_SIZE:
                            continue
                        if not is_password_file(info.filename):
                            continue
                        try:
                            data = rf.read(info.filename)
                            try:
                                content = data.decode('utf-8', errors='ignore')
                            except:
                                content = data.decode('latin-1', errors='ignore')
                            
                            if len(content) > 50 and is_leak_content(content):
                                title = generate_auto_title(content)
                                await self.post_queue.put((content, title))
                                processed += 1
                                logger.info(f"    🔑 Extracted password file: {info.filename}")
                        except Exception as e:
                            logger.debug(f"    Error extracting {info.filename}: {e}")
            
            elif lower_filename.endswith('.rar') and not HAS_RARFILE:
                logger.warning(f"  Cannot extract .rar - rarfile not installed")
                
        except Exception as e:
            logger.error(f"  Error extracting archive {filename}: {e}")
        
        return processed

    async def extract_text_from_archive(self, buffer: io.BytesIO, filename: str, channel_name: str) -> int:
        """
        Extract ONLY password files from a zip/rar archive (in memory) and post them.
        Returns number of files processed.
        """
        processed = 0
        lower_filename = filename.lower()
        
        try:
            if lower_filename.endswith('.zip'):
                buffer.seek(0)
                with zipfile.ZipFile(buffer, 'r') as zf:
                    for info in zf.infolist():
                        # Skip directories and large files
                        if info.is_dir() or info.file_size > MAX_FILE_SIZE:
                            continue
                        
                        # ONLY extract password files
                        if not is_password_file(info.filename):
                            continue
                        
                        try:
                            data = zf.read(info.filename)
                            try:
                                content = data.decode('utf-8', errors='ignore')
                            except:
                                content = data.decode('latin-1', errors='ignore')
                            
                            if len(content) > 50:  # Reasonable minimum
                                title = generate_auto_title(content, f"{channel_name}/{filename}")
                                await self.post_queue.put((content, title))
                                processed += 1
                                logger.info(f"    🔑 Extracted password file: {info.filename}")
                        except Exception as e:
                            logger.debug(f"    Error extracting {info.filename}: {e}")
            
            elif lower_filename.endswith('.rar') and HAS_RARFILE:
                # Write to temp file for rarfile (it needs a file path)
                buffer.seek(0)
                tmp_path = None
                try:
                    with tempfile.NamedTemporaryFile(suffix='.rar', delete=False, dir=TEMP_DIR) as tmp:
                        tmp.write(buffer.read())
                        tmp_path = tmp.name
                    
                    # Clear buffer to free memory
                    buffer.seek(0)
                    buffer.truncate(0)
                    
                    with rarfile.RarFile(tmp_path, 'r') as rf:
                        for info in rf.infolist():
                            # Skip directories and large files
                            if info.is_dir() or info.file_size > MAX_FILE_SIZE:
                                continue
                            
                            # ONLY extract password files
                            if not is_password_file(info.filename):
                                continue
                            
                            try:
                                data = rf.read(info.filename)
                                try:
                                    content = data.decode('utf-8', errors='ignore')
                                except:
                                    content = data.decode('latin-1', errors='ignore')
                                
                                if len(content) > 50:  # Reasonable minimum
                                    title = generate_auto_title(content, f"{channel_name}/{filename}")
                                    await self.post_queue.put((content, title))
                                    processed += 1
                                    logger.info(f"    🔑 Extracted password file: {info.filename}")
                            except Exception as e:
                                logger.debug(f"    Error extracting {info.filename}: {e}")
                finally:
                    # Always clean up temp file
                    if tmp_path and os.path.exists(tmp_path):
                        os.unlink(tmp_path)
                        logger.debug(f"    Cleaned up temp file: {tmp_path}")
            
            elif lower_filename.endswith('.rar') and not HAS_RARFILE:
                logger.warning(f"  Cannot extract .rar - rarfile not installed")
                
        except Exception as e:
            logger.error(f"  Error extracting archive {filename}: {e}")
        
        return processed

    async def download_from_gofile(self, url: str) -> tuple:
        """
        Download file from gofile.io
        Returns (bytes, filename) or (None, None) on failure
        """
        try:
            match = re.search(r'gofile\.io/d/([a-zA-Z0-9]+)', url)
            if not match:
                return None, None
            
            file_id = match.group(1)
            
            async with aiohttp.ClientSession() as session:
                # Get account token first
                async with session.post('https://api.gofile.io/accounts') as resp:
                    if resp.status != 200:
                        return None, None
                    data = await resp.json()
                    token = data.get('data', {}).get('token')
                
                if not token:
                    return None, None
                
                # Get file info
                headers = {'Authorization': f'Bearer {token}'}
                async with session.get(f'https://api.gofile.io/contents/{file_id}', headers=headers) as resp:
                    if resp.status != 200:
                        return None, None
                    data = await resp.json()
                
                contents = data.get('data', {}).get('children', {})
                if not contents:
                    return None, None
                
                # Get first file
                for file_info in contents.values():
                    if file_info.get('type') == 'file':
                        download_url = file_info.get('link')
                        fname = file_info.get('name', 'unknown')
                        file_size = file_info.get('size', 0)
                        
                        if file_size > MAX_ARCHIVE_SIZE:
                            logger.info(f"  Skipping large gofile: {fname} ({file_size / 1024 / 1024:.1f}MB)")
                            return None, None
                        
                        # Download the file
                        async with session.get(download_url, headers=headers) as resp:
                            if resp.status == 200:
                                return await resp.read(), fname
                
                return None, None
                
        except Exception as e:
            logger.error(f"  Gofile download error: {e}")
            return None, None

    async def download_from_pixeldrain(self, url: str) -> tuple:
        """
        Download file from pixeldrain.com
        Returns (bytes, filename) or (None, None) on failure
        """
        try:
            match = re.search(r'pixeldrain\.com/(?:u|l)/([a-zA-Z0-9]+)', url)
            if not match:
                return None, None
            
            file_id = match.group(1)
            api_url = f'https://pixeldrain.com/api/file/{file_id}'
            info_url = f'https://pixeldrain.com/api/file/{file_id}/info'
            
            async with aiohttp.ClientSession() as session:
                # Get file info first
                async with session.get(info_url) as resp:
                    if resp.status == 200:
                        info = await resp.json()
                        fname = info.get('name', 'unknown')
                        file_size = info.get('size', 0)
                        
                        if file_size > MAX_ARCHIVE_SIZE:
                            logger.info(f"  Skipping large pixeldrain: {fname} ({file_size / 1024 / 1024:.1f}MB)")
                            return None, None
                    else:
                        fname = 'unknown'
                
                # Download
                async with session.get(api_url) as resp:
                    if resp.status == 200:
                        return await resp.read(), fname
            
            return None, None
            
        except Exception as e:
            logger.error(f"  Pixeldrain download error: {e}")
            return None, None

    async def download_from_catbox(self, url: str) -> tuple:
        """
        Download file from catbox.moe / litterbox
        Returns (bytes, filename) or (None, None) on failure
        """
        try:
            match = re.search(r'/([a-zA-Z0-9]+\.[a-z]+)$', url)
            fname = match.group(1) if match else 'unknown'
            
            async with aiohttp.ClientSession() as session:
                async with session.get(url) as resp:
                    if resp.status == 200:
                        content_length = resp.headers.get('Content-Length', 0)
                        if int(content_length) > MAX_ARCHIVE_SIZE:
                            logger.info(f"  Skipping large catbox file: {fname}")
                            return None, None
                        return await resp.read(), fname
            
            return None, None
            
        except Exception as e:
            logger.error(f"  Catbox download error: {e}")
            return None, None

    async def download_from_direct_link(self, url: str) -> tuple:
        """
        Download from direct link (generic)
        Returns (bytes, filename) or (None, None) on failure
        """
        try:
            fname = url.split('/')[-1].split('?')[0]
            if not fname:
                fname = 'unknown.zip'
            
            async with aiohttp.ClientSession() as session:
                async with session.get(url, timeout=aiohttp.ClientTimeout(total=DOWNLOAD_TIMEOUT)) as resp:
                    if resp.status == 200:
                        content_length = resp.headers.get('Content-Length', 0)
                        if int(content_length) > MAX_ARCHIVE_SIZE:
                            logger.info(f"  Skipping large file: {fname}")
                            return None, None
                        return await resp.read(), fname
            
            return None, None
            
        except Exception as e:
            logger.error(f"  Direct download error: {e}")
            return None, None

    async def download_from_mediafire(self, url: str) -> tuple:
        """
        Download file from mediafire.com
        Returns (bytes, filename) or (None, None) on failure
        """
        try:
            async with aiohttp.ClientSession() as session:
                # Get the download page
                async with session.get(url) as resp:
                    if resp.status != 200:
                        return None, None
                    html = await resp.text()
                
                # Extract direct download link from page
                # MediaFire puts it in a JS variable or direct link
                download_match = re.search(r'href="(https://download\d*\.mediafire\.com/[^"]+)"', html)
                if not download_match:
                    # Try alternate pattern
                    download_match = re.search(r"aria-label=\"Download file\"[^>]*href=\"([^\"]+)\"", html)
                
                if not download_match:
                    logger.warning("  Could not find MediaFire download link")
                    return None, None
                
                download_url = download_match.group(1)
                
                # Extract filename from URL or page
                fname_match = re.search(r'<div class="filename">([^<]+)</div>', html)
                fname = fname_match.group(1) if fname_match else url.split('/')[-1]
                
                # Download the file
                async with session.get(download_url, timeout=aiohttp.ClientTimeout(total=DOWNLOAD_TIMEOUT)) as resp:
                    if resp.status == 200:
                        content_length = resp.headers.get('Content-Length', 0)
                        if int(content_length) > MAX_ARCHIVE_SIZE:
                            logger.info(f"  Skipping large mediafire: {fname} ({int(content_length) / 1024 / 1024:.1f}MB)")
                            return None, None
                        return await resp.read(), fname
            
            return None, None
            
        except Exception as e:
            logger.error(f"  MediaFire download error: {e}")
            return None, None

    async def download_from_mega(self, url: str) -> tuple:
        """
        Download file from mega.nz
        Note: Mega requires special handling due to encryption.
        This uses the megadl CLI if available, otherwise skips.
        Returns (bytes, filename) or (None, None) on failure
        """
        try:
            import subprocess
            import shutil
            
            # Check if megatools is installed
            if not shutil.which('megadl'):
                logger.warning("  megadl not installed - skipping Mega link (install: apt install megatools)")
                return None, None
            
            # Create temp directory for download
            with tempfile.TemporaryDirectory(prefix='skybin_mega_') as tmpdir:
                # Download using megadl
                result = subprocess.run(
                    ['megadl', '--path', tmpdir, url],
                    capture_output=True,
                    text=True,
                    timeout=DOWNLOAD_TIMEOUT
                )
                
                if result.returncode != 0:
                    logger.warning(f"  Mega download failed: {result.stderr[:100]}")
                    return None, None
                
                # Find downloaded file
                files = os.listdir(tmpdir)
                if not files:
                    return None, None
                
                fpath = os.path.join(tmpdir, files[0])
                fname = files[0]
                
                # Check size
                fsize = os.path.getsize(fpath)
                if fsize > MAX_ARCHIVE_SIZE:
                    logger.info(f"  Skipping large mega file: {fname} ({fsize / 1024 / 1024:.1f}MB)")
                    return None, None
                
                # Read file
                with open(fpath, 'rb') as f:
                    data = f.read()
                
                return data, fname
                
        except subprocess.TimeoutExpired:
            logger.warning("  Mega download timed out")
            return None, None
        except Exception as e:
            logger.error(f"  Mega download error: {e}")
            return None, None

    async def download_from_krakenfiles(self, url: str) -> tuple:
        """
        Download file from krakenfiles.com
        Returns (bytes, filename) or (None, None) on failure
        """
        try:
            async with aiohttp.ClientSession() as session:
                # Get the download page
                async with session.get(url) as resp:
                    if resp.status != 200:
                        return None, None
                    html = await resp.text()
                
                # Extract hash and filename
                hash_match = re.search(r'data-file-hash="([^"]+)"', html)
                fname_match = re.search(r'<span class="coin-name">([^<]+)</span>', html)
                
                if not hash_match:
                    return None, None
                
                file_hash = hash_match.group(1)
                fname = fname_match.group(1) if fname_match else 'unknown'
                
                # Get download token
                async with session.post(
                    'https://krakenfiles.com/download/' + file_hash,
                    data={'hash': file_hash}
                ) as resp:
                    if resp.status != 200:
                        return None, None
                    data = await resp.json()
                
                download_url = data.get('url')
                if not download_url:
                    return None, None
                
                # Download
                async with session.get(download_url, timeout=aiohttp.ClientTimeout(total=DOWNLOAD_TIMEOUT)) as resp:
                    if resp.status == 200:
                        content_length = resp.headers.get('Content-Length', 0)
                        if int(content_length) > MAX_ARCHIVE_SIZE:
                            logger.info(f"  Skipping large krakenfiles: {fname}")
                            return None, None
                        return await resp.read(), fname
            
            return None, None
            
        except Exception as e:
            logger.error(f"  Krakenfiles download error: {e}")
            return None, None

    async def process_external_link(self, url: str, host: str, channel_name: str) -> bool:
        """
        Download and process a file from an external host.
        For large files (>500MB), streams to disk instead of memory.
        Extracts password files and posts to SkyBin, then cleans up.
        Returns True if successful.
        """
        if url in self.processed_links:
            return False
        
        self.processed_links.add(url)
        logger.info(f"  🔗 Downloading from {host}: {url[:60]}...")
        
        # Threshold for streaming to disk instead of memory
        STREAM_THRESHOLD = 500 * 1024 * 1024  # 500MB
        
        try:
            # Download based on host
            if host == 'gofile':
                data, fname = await self.download_from_gofile(url)
            elif host == 'pixeldrain':
                data, fname = await self.download_from_pixeldrain(url)
            elif host == 'mega':
                data, fname = await self.download_from_mega(url)
            elif host == 'mediafire':
                data, fname = await self.download_from_mediafire(url)
            elif host == 'catbox':
                data, fname = await self.download_from_catbox(url)
            elif host == 'krakenfiles':
                data, fname = await self.download_from_krakenfiles(url)
            elif host == 'direct':
                data, fname = await self.download_from_direct_link(url)
            else:
                # Unsupported hosts - log but don't fail
                logger.debug(f"  Skipping unsupported host: {host}")
                return False
            
            if not data:
                logger.warning(f"  Failed to download from {host}")
                return False
            
            self.links_downloaded += 1
            file_size = len(data)
            logger.info(f"  📥 Downloaded: {fname} ({file_size / 1024 / 1024:.1f}MB)")
            
            # Check if it's an archive
            lower = fname.lower()
            if any(lower.endswith(ext) for ext in ARCHIVE_EXTENSIONS):
                # For very large files, write to disk first
                if file_size > STREAM_THRESHOLD:
                    tmp_path = None
                    try:
                        ext = os.path.splitext(fname)[1]
                        with tempfile.NamedTemporaryFile(suffix=ext, prefix=TEMP_FILE_PREFIX, delete=False, dir=TEMP_DIR) as tmp:
                            tmp.write(data)
                            tmp_path = tmp.name
                        del data  # Free memory immediately
                        
                        logger.info(f"  💾 Large archive saved to temp, extracting...")
                        extracted = await self.extract_text_from_archive_file(tmp_path, fname, channel_name)
                    finally:
                        if tmp_path and os.path.exists(tmp_path):
                            os.unlink(tmp_path)
                            logger.debug(f"  🗑️ Cleaned up temp file")
                else:
                    buffer = io.BytesIO(data)
                    extracted = await self.extract_text_from_archive(buffer, fname, f"{channel_name}/{host}")
                    buffer.close()
                    del data
                
                if extracted > 0:
                    logger.info(f"  ✓ Extracted {extracted} password files from {fname}")
                    return True
                else:
                    logger.info(f"  No password files in: {fname}")
                    return False
            
            # Plain text file - check if it's a password file
            elif any(lower.endswith(ext) for ext in CRED_FILE_EXTENSIONS):
                try:
                    content = data.decode('utf-8', errors='ignore')
                except:
                    content = data.decode('latin-1', errors='ignore')
                
                del data
                
                if is_password_file(fname) or is_leak_content(content):
                    title = generate_auto_title(content)
                    await self.post_queue.put((content, title))
                    logger.info(f"  ✓ Posted content from: {fname}")
                    return True
            
            return False
            
        except Exception as e:
            logger.error(f"  Error processing {host} link: {e}")
            return False

    async def link_worker(self, worker_id: int = 0):
        """Worker to process external link downloads from queue"""
        while True:
            try:
                url, host, channel_name = await asyncio.wait_for(self.link_queue.get(), timeout=5.0)
                async with self.download_semaphore:
                    await self.process_external_link(url, host, channel_name)
                # Rate limit per host - external services need spacing
                await asyncio.sleep(2)
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"Link worker {worker_id} error: {e}")
                await asyncio.sleep(2)

    async def download_and_process_file(self, message, channel_name: str) -> bool:
        """
        Download a file from Telegram and post its contents.
        Supports both plain text files and archives (.zip, .rar).
        Returns True if successfully processed.
        """
        if not message.document:
            return False
        
        # Get filename
        filename = None
        for attr in message.document.attributes:
            if isinstance(attr, DocumentAttributeFilename):
                filename = attr.file_name
                break
        
        if not filename:
            return False
        
        lower_filename = filename.lower()
        is_archive = any(lower_filename.endswith(ext) for ext in ARCHIVE_EXTENSIONS)
        is_text_file = any(lower_filename.endswith(ext) for ext in CRED_FILE_EXTENSIONS)
        
        if not is_archive and not is_text_file:
            return False
        
        # Check file size (different limits for archives vs text)
        file_size = message.document.size
        max_size = MAX_ARCHIVE_SIZE if is_archive else MAX_FILE_SIZE
        
        if file_size > max_size:
            logger.info(f"  Skipping large file: {filename} ({file_size / 1024 / 1024:.1f}MB > {max_size / 1024 / 1024:.0f}MB limit)")
            return False
        
        # Create unique file ID
        file_id = f"{message.chat_id}_{message.id}_{filename}"
        if file_id in self.processed_files:
            return False
        
        try:
            async with self.download_semaphore:
                size_str = f"{file_size / 1024 / 1024:.1f}MB" if file_size > 1024*1024 else f"{file_size / 1024:.1f}KB"
                logger.info(f"  📥 Downloading: {filename} ({size_str})")
                
                self.processed_files.add(file_id)
                
                # Handle archives - download to temp file for large ones
                if is_archive:
                    logger.info(f"  📦 Processing archive: {filename}")
                    
                    # For large archives (>100MB), download to disk and extract directly
                    if file_size > 100 * 1024 * 1024:  # > 100MB
                        tmp_path = None
                        try:
                            ext = os.path.splitext(filename)[1]
                            with tempfile.NamedTemporaryFile(suffix=ext, prefix=TEMP_FILE_PREFIX, delete=False, dir=TEMP_DIR) as tmp:
                                tmp_path = tmp.name
                            
                            logger.info(f"  📥 Downloading large archive ({file_size / 1024 / 1024 / 1024:.2f}GB) to temp...")
                            await self.client.download_media(message, tmp_path)
                            logger.info(f"  💾 Download complete, extracting from disk...")
                            
                            # Extract directly from disk file (no memory copy)
                            extracted = await self.extract_text_from_archive_file(tmp_path, filename, channel_name)
                            
                        except Exception as e:
                            logger.error(f"  Error with large archive: {e}")
                            extracted = 0
                        finally:
                            # Always clean up temp file
                            if tmp_path and os.path.exists(tmp_path):
                                os.unlink(tmp_path)
                                logger.debug(f"  🗑️ Cleaned up temp file")
                    else:
                        # Smaller archives - download to memory
                        buffer = io.BytesIO()
                        await self.client.download_media(message, buffer)
                        buffer.seek(0)
                        
                        extracted = await self.extract_text_from_archive(buffer, filename, channel_name)
                        
                        # Clear buffer to free memory
                        buffer.close()
                        del buffer
                    
                    if extracted > 0:
                        self.files_downloaded += extracted
                        logger.info(f"  ✓ Extracted {extracted} files from: {filename}")
                        return True
                    else:
                        logger.info(f"  No text files found in archive: {filename}")
                        return False
                
                # For regular files, download to memory
                buffer = io.BytesIO()
                await self.client.download_media(message, buffer)
                buffer.seek(0)
                
                # Handle plain text files
                try:
                    content = buffer.read().decode('utf-8', errors='ignore')
                except:
                    try:
                        buffer.seek(0)
                        content = buffer.read().decode('latin-1', errors='ignore')
                    except:
                        logger.warning(f"  Could not decode file: {filename}")
                        return False
                
                # Check if content is leak material
                if not is_leak_content(content) and len(content) < 100:
                    logger.debug(f"  File doesn't contain leak content: {filename}")
                    return False
                
                # Generate auto-title based on content
                title = generate_auto_title(content, channel_name)
                
                # Add to post queue
                await self.post_queue.put((content, title))
                self.files_downloaded += 1
                logger.info(f"  ✓ Processed file: {filename}")
                return True
                
        except Exception as e:
            logger.error(f"  Error downloading {filename}: {e}")
            return False

    async def file_worker(self, worker_id: int = 0):
        """Worker to process file downloads from queue"""
        while True:
            try:
                message, channel_name = await asyncio.wait_for(self.file_queue.get(), timeout=5.0)
                await self.download_and_process_file(message, channel_name)
                # Minimal delay - semaphore handles concurrency
                await asyncio.sleep(0.5)
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"File worker {worker_id} error: {e}")
                await asyncio.sleep(2)

    async def scrape_channel(self, channel, limit=100):
        """Scrape recent messages from a channel"""
        name = getattr(channel, 'title', str(channel.id))
        
        try:
            count = 0
            file_count = 0
            link_count = 0
            async for message in self.client.iter_messages(channel, limit=limit):
                if message.id in self.processed_messages:
                    continue
                
                # Check for credential file attachments - queue for download
                if message.document:
                    try:
                        filename = None
                        for attr in message.document.attributes:
                            if isinstance(attr, DocumentAttributeFilename):
                                filename = attr.file_name
                                break
                        
                        all_extensions = CRED_FILE_EXTENSIONS + ARCHIVE_EXTENSIONS
                        if filename and any(filename.lower().endswith(ext) for ext in all_extensions):
                            # Queue file for download
                            file_id = f"{message.chat_id}_{message.id}_{filename}"
                            if file_id not in self.processed_files:
                                await self.file_queue.put((message, name))
                                file_count += 1
                                self.processed_messages.add(message.id)
                                continue  # Don't also process as text
                    except Exception as e:
                        logger.debug(f"Error checking file: {e}")
                
                # Check for external file host links
                text = message.text or ''
                external_links = extract_file_host_links(text)
                for url, host in external_links:
                    if url not in self.processed_links:
                        await self.link_queue.put((url, host, name))
                        link_count += 1
                        self.processed_messages.add(message.id)
                
                # Process text content (only if has actual credentials)
                if is_leak_content(text):
                    title = generate_auto_title(text, name)
                    await self.post_queue.put((text, title))
                    self.processed_messages.add(message.id)
                    count += 1
                    
            if count > 0 or file_count > 0 or link_count > 0:
                logger.info(f"  Found {count} leak messages, {file_count} files, {link_count} external links in {name}")
                    
        except ChannelPrivateError:
            logger.warning(f"  Cannot access {name} (private)")
        except FloodWaitError as e:
            logger.warning(f"  Rate limited, waiting {e.seconds}s")
            await asyncio.sleep(e.seconds)
        except Exception as e:
            logger.error(f"  Error scraping {name}: {e}")
    
    async def post_worker(self, worker_id: int = 0):
        """Worker to post items from queue with rate limiting"""
        while True:
            try:
                content, title = await asyncio.wait_for(self.post_queue.get(), timeout=5.0)
                
                success = await post_to_skybin(content, title)
                if success:
                    self.posts_made += 1
                
                # Minimal rate limit - SkyBin can handle concurrent posts
                await asyncio.sleep(0.5)
                
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"Post worker {worker_id} error: {e}")
                await asyncio.sleep(2)
    
    async def monitor_realtime(self):
        """Monitor all channels for new messages in real-time"""
        if not self.active_channels:
            logger.warning("No channels to monitor")
            return
        
        # Register handler for new messages in monitored channels
        @self.client.on(events.NewMessage(chats=self.active_channels))
        async def handler(event):
            try:
                chat = await event.get_chat()
                chat_name = getattr(chat, 'title', None) or str(chat.id)
                
                # Check for credential file attachments - queue for download
                if event.document:
                    try:
                        filename = None
                        for attr in event.document.attributes:
                            if isinstance(attr, DocumentAttributeFilename):
                                filename = attr.file_name
                                break
                        
                        all_extensions = CRED_FILE_EXTENSIONS + ARCHIVE_EXTENSIONS
                        if filename and any(filename.lower().endswith(ext) for ext in all_extensions):
                            file_id = f"{event.chat_id}_{event.id}_{filename}"
                            if file_id not in self.processed_files:
                                await self.file_queue.put((event.message, chat_name))
                                logger.info(f"📁 New file from {chat_name}: {filename}")
                                return  # Don't also process as text
                    except Exception as e:
                        logger.debug(f"Error checking file in handler: {e}")
                
                # Check for external file host links
                text = event.text or ''
                external_links = extract_file_host_links(text)
                for url, host in external_links:
                    if url not in self.processed_links:
                        await self.link_queue.put((url, host, chat_name))
                        logger.info(f"🔗 New external link from {chat_name}: {host}")
                
                # Process text content (only if has actual credentials)
                if is_leak_content(text):
                    title = generate_auto_title(text, chat_name)
                    await self.post_queue.put((text, title))
                    logger.info(f"New leak from {chat_name}")
                    
            except Exception as e:
                logger.error(f"Handler error: {e}")
        
        # Also monitor for when we join new channels
        @self.client.on(events.ChatAction())
        async def join_handler(event):
            if event.user_joined or event.user_added:
                me = await self.client.get_me()
                if event.user_id == me.id:
                    chat = await event.get_chat()
                    if isinstance(chat, (Channel, Chat)) and chat not in self.active_channels:
                        self.active_channels.append(chat)
                        logger.info(f"Joined new channel: {getattr(chat, 'title', chat.id)}")
                        # Scrape the new channel
                        await self.scrape_channel(chat, limit=50)
                
        logger.info(f"Real-time monitoring started for {len(self.active_channels)} channels")
        logger.info("Press Ctrl+C to stop")
        
        await self.client.run_until_disconnected()
        
    async def run(self):
        """Main run loop"""
        if not await self.start():
            return
        
        # Start multiple worker tasks for parallel processing
        worker_tasks = []
        
        # Post workers
        for i in range(NUM_POST_WORKERS):
            worker_tasks.append(asyncio.create_task(self.post_worker(i)))
        
        # File download workers
        for i in range(NUM_FILE_WORKERS):
            worker_tasks.append(asyncio.create_task(self.file_worker(i)))
        
        # Link download workers
        for i in range(NUM_LINK_WORKERS):
            worker_tasks.append(asyncio.create_task(self.link_worker(i)))
        
        logger.info(f"Started {NUM_POST_WORKERS} post workers, {NUM_FILE_WORKERS} file workers, {NUM_LINK_WORKERS} link workers")
        
        try:
            # First, try to join known leak channels
            await self.join_known_channels()
            
            # Then find all channels we're now in
            await self.find_leak_channels()
            
            if not self.active_channels:
                logger.warning("No channels found. The known channels may have been taken down.")
                logger.info("Join some leak channels manually on Telegram and restart.")
                return
            
            # Scrape history from each channel
            logger.info("Scraping recent history...")
            for channel in self.active_channels:
                await self.scrape_channel(channel, limit=50)
                await asyncio.sleep(1)  # Rate limit between channels
            
            # Wait for queues to empty
            while not self.post_queue.empty() or not self.file_queue.empty() or not self.link_queue.empty():
                await asyncio.sleep(1)
            
            logger.info(f"Initial scrape complete. Posted {self.posts_made} items, downloaded {self.files_downloaded} files, {self.links_downloaded} external links.")
            
            # Then monitor in real-time
            await self.monitor_realtime()
            
        finally:
            for task in worker_tasks:
                task.cancel()
        
    async def stop(self):
        """Stop the client"""
        if self.client:
            await self.client.disconnect()

async def main():
    # Cleanup orphaned temp files from previous crashes
    cleanup_orphaned_temp_files()
    
    scraper = TelegramScraper()
    try:
        await scraper.run()
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    finally:
        await scraper.stop()

if __name__ == '__main__':
    asyncio.run(main())
