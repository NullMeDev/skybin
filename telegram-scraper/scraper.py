#!/usr/bin/env python3
"""
SkyBin Telegram Scraper Service v2.3
- Auto-discovers leak channels from your dialogs
- Joins known active channels
- Monitors all channels/groups you're in
- Downloads .txt/.csv/.json/.sql/.log/.env files
- Extracts text from .zip and .rar archives (up to 3GB)
- Downloads to temp, extracts, then deletes - no disk bloat
- Filters Stripe checkout URLs (allows API keys)
- Pattern-based BIN detection (not keyword-based)
- Auto-generates titles based on content analysis
- Posts found leaks incrementally to SkyBin API
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

# Max file size for regular files (5MB)
MAX_FILE_SIZE = 5 * 1024 * 1024

# Max file size for archives (3GB - downloaded to temp, extracted, then deleted)
MAX_ARCHIVE_SIZE = 3 * 1024 * 1024 * 1024

# Concurrent file downloads limit
MAX_CONCURRENT_DOWNLOADS = 2

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
    
    # Accept if:
    # 1. Any email:pass combo
    # 2. Any username:pass combo
    # 3. Any ULP format entry
    # 4. Any credential pattern (API key, token, etc)
    # 5. Private key
    # 6. Has actual BIN/card data patterns
    # 7. 3+ leak keywords (suggests leak content)
    return (
        email_pass_count >= MIN_EMAIL_PASS_COMBOS or
        user_pass_count >= 1 or
        ulp_count >= 1 or
        cred_pattern_count >= MIN_CREDENTIAL_PATTERNS or
        has_private_key or
        has_bin_data or
        keyword_count >= 3
    )

def generate_auto_title(content: str, channel_name: str = "Telegram") -> str:
    """
    Generate a title based on content analysis.
    Detects services, credential types, and formats.
    """
    lower = content.lower()
    detected = []
    
    # Detect services
    service_keywords = {
        'netflix': 'Netflix', 'spotify': 'Spotify', 'disney': 'Disney+',
        'hbo': 'HBO', 'amazon': 'Amazon', 'prime': 'Prime', 'steam': 'Steam',
        'fortnite': 'Fortnite', 'minecraft': 'Minecraft', 'roblox': 'Roblox',
        'gmail': 'Gmail', 'yahoo': 'Yahoo', 'outlook': 'Outlook',
        'paypal': 'PayPal', 'crypto': 'Crypto', 'nordvpn': 'NordVPN',
        'expressvpn': 'ExpressVPN', 'cpanel': 'cPanel', 'rdp': 'RDP',
        'github': 'GitHub', 'aws': 'AWS', 'mongodb': 'MongoDB',
    }
    
    for keyword, name in service_keywords.items():
        if keyword in lower:
            detected.append(name)
            if len(detected) >= 2:
                break
    
    # Detect format
    format_type = None
    if re.search(r'https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}', content):
        format_type = "URL:Login:Pass"
    elif re.search(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,}', content):
        format_type = "Email:Pass"
    elif re.search(r'\b[a-zA-Z0-9_.+-]+:[^\s@:]{6,}', content):
        format_type = "User:Pass"
    elif has_actual_bin_data(content):
        format_type = "BINs/Cards"
    
    # Count credentials
    email_pass_count = len(re.findall(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@:]{4,}', content))
    line_count = len(content.split('\n'))
    
    # Build title
    parts = [f"[TG] {channel_name}"]
    
    if detected:
        parts.append(" | ".join(detected[:2]))
    
    if format_type:
        parts.append(format_type)
    
    if email_pass_count > 10:
        parts.append(f"{email_pass_count} combos")
    elif line_count > 100:
        parts.append(f"{line_count} lines")
    
    return " - ".join(parts)[:100]

async def post_to_skybin(content: str, title: str, source: str = "telegram"):
    """Post discovered content to SkyBin API"""
    if len(content) < 50:  # Lowered threshold
        return False
        
    try:
        async with aiohttp.ClientSession() as session:
            payload = {
                'content': content[:100000],  # Increased limit
                'title': (title[:100] if title else 'Telegram Leak').strip(),
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

class TelegramScraper:
    def __init__(self):
        self.client = None
        self.processed_messages = set()
        self.processed_files = set()  # Track downloaded files by unique ID
        self.active_channels = []
        self.post_queue = asyncio.Queue()
        self.file_queue = asyncio.Queue()  # Queue for file downloads
        self.posts_made = 0
        self.files_downloaded = 0
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
    
    async def extract_text_from_archive(self, buffer: io.BytesIO, filename: str, channel_name: str) -> int:
        """
        Extract text files from a zip/rar archive and post them.
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
                        
                        # Only extract text-like files
                        inner_name = info.filename.lower()
                        if any(inner_name.endswith(ext) for ext in CRED_FILE_EXTENSIONS):
                            try:
                                data = zf.read(info.filename)
                                try:
                                    content = data.decode('utf-8', errors='ignore')
                                except:
                                    content = data.decode('latin-1', errors='ignore')
                                
                                if is_leak_content(content) or len(content) > 100:
                                    title = generate_auto_title(content, f"{channel_name}/{filename}")
                                    await self.post_queue.put((content, title))
                                    processed += 1
                                    logger.info(f"    📄 Extracted: {info.filename}")
                            except Exception as e:
                                logger.debug(f"    Error extracting {info.filename}: {e}")
            
            elif lower_filename.endswith('.rar') and HAS_RARFILE:
                # Write to temp file for rarfile (it needs a file path)
                buffer.seek(0)
                tmp_path = None
                try:
                    with tempfile.NamedTemporaryFile(suffix='.rar', delete=False) as tmp:
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
                            
                            inner_name = info.filename.lower()
                            if any(inner_name.endswith(ext) for ext in CRED_FILE_EXTENSIONS):
                                try:
                                    data = rf.read(info.filename)
                                    try:
                                        content = data.decode('utf-8', errors='ignore')
                                    except:
                                        content = data.decode('latin-1', errors='ignore')
                                    
                                    if is_leak_content(content) or len(content) > 100:
                                        title = generate_auto_title(content, f"{channel_name}/{filename}")
                                        await self.post_queue.put((content, title))
                                        processed += 1
                                        logger.info(f"    📄 Extracted: {info.filename}")
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
                    
                    # For large archives, download directly to temp file
                    if file_size > 100 * 1024 * 1024:  # > 100MB
                        tmp_path = None
                        try:
                            with tempfile.NamedTemporaryFile(suffix=os.path.splitext(filename)[1], delete=False) as tmp:
                                tmp_path = tmp.name
                            
                            logger.info(f"  📥 Downloading large archive to temp...")
                            await self.client.download_media(message, tmp_path)
                            
                            # Read into buffer for extraction
                            with open(tmp_path, 'rb') as f:
                                buffer = io.BytesIO(f.read())
                            
                            # Delete temp file immediately after reading
                            os.unlink(tmp_path)
                            tmp_path = None
                            logger.info(f"  🗑️ Temp file deleted, extracting from memory...")
                            
                        except Exception as e:
                            logger.error(f"  Error with large archive: {e}")
                            if tmp_path and os.path.exists(tmp_path):
                                os.unlink(tmp_path)
                            return False
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

    async def file_worker(self):
        """Worker to process file downloads from queue"""
        while True:
            try:
                message, channel_name = await asyncio.wait_for(self.file_queue.get(), timeout=5.0)
                await self.download_and_process_file(message, channel_name)
                # Rate limit file downloads
                await asyncio.sleep(3)
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"File worker error: {e}")
                await asyncio.sleep(5)

    async def scrape_channel(self, channel, limit=100):
        """Scrape recent messages from a channel"""
        name = getattr(channel, 'title', str(channel.id))
        
        try:
            count = 0
            file_count = 0
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
                
                # Process text content
                text = message.text or ''
                if is_leak_content(text):
                    title = generate_auto_title(text, name)
                    await self.post_queue.put((text, title))
                    self.processed_messages.add(message.id)
                    count += 1
                    
            if count > 0 or file_count > 0:
                logger.info(f"  Found {count} leak messages, {file_count} files in {name}")
                    
        except ChannelPrivateError:
            logger.warning(f"  Cannot access {name} (private)")
        except FloodWaitError as e:
            logger.warning(f"  Rate limited, waiting {e.seconds}s")
            await asyncio.sleep(e.seconds)
        except Exception as e:
            logger.error(f"  Error scraping {name}: {e}")
    
    async def post_worker(self):
        """Worker to post items from queue with rate limiting"""
        while True:
            try:
                content, title = await asyncio.wait_for(self.post_queue.get(), timeout=5.0)
                
                success = await post_to_skybin(content, title)
                if success:
                    self.posts_made += 1
                
                # Rate limit: 1 post every 2 seconds
                await asyncio.sleep(2)
                
            except asyncio.TimeoutError:
                continue
            except Exception as e:
                logger.error(f"Post worker error: {e}")
                await asyncio.sleep(5)
    
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
                
                # Process text content
                text = event.text or ''
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
        
        # Start worker tasks
        post_worker_task = asyncio.create_task(self.post_worker())
        file_worker_task = asyncio.create_task(self.file_worker())
        
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
            while not self.post_queue.empty() or not self.file_queue.empty():
                await asyncio.sleep(1)
            
            logger.info(f"Initial scrape complete. Posted {self.posts_made} items, downloaded {self.files_downloaded} files.")
            
            # Then monitor in real-time
            await self.monitor_realtime()
            
        finally:
            post_worker_task.cancel()
            file_worker_task.cancel()
        
    async def stop(self):
        """Stop the client"""
        if self.client:
            await self.client.disconnect()

async def main():
    scraper = TelegramScraper()
    try:
        await scraper.run()
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    finally:
        await scraper.stop()

if __name__ == '__main__':
    asyncio.run(main())
