#!/usr/bin/env python3
"""
SkyBin Telegram Scraper Service v2.0
- Auto-discovers leak channels from your dialogs
- Joins known active channels
- Monitors all channels/groups you're in
- Posts found leaks incrementally to SkyBin API
"""

import asyncio
import re
import os
import json
import logging
import aiohttp
from datetime import datetime, timedelta
from dotenv import load_dotenv
from telethon import TelegramClient, events, functions
from telethon.tl.types import Channel, Chat, User, InputPeerChannel
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

# Credential detection patterns - EXPANDED
CREDENTIAL_PATTERNS = [
    # API keys and tokens
    re.compile(r'(?i)(api[_-]?key|apikey)\s*[=:]\s*[\'"]?[a-zA-Z0-9_-]{16,}'),
    re.compile(r'(?i)(secret[_-]?key|secretkey)\s*[=:]\s*[\'"]?[a-zA-Z0-9_-]{16,}'),
    re.compile(r'(?i)(access[_-]?token|accesstoken)\s*[=:]\s*[\'"]?[a-zA-Z0-9_-]{16,}'),
    re.compile(r'(?i)(auth[_-]?token|bearer)\s*[=:]\s*[\'"]?[a-zA-Z0-9_-]{16,}'),
    re.compile(r'(?i)(password|passwd|pwd)\s*[=:]\s*[\'"]?[^\s\'"]{6,}'),
    
    # Platform-specific tokens
    re.compile(r'ghp_[a-zA-Z0-9]{36}'),  # GitHub PAT
    re.compile(r'gho_[a-zA-Z0-9]{36}'),  # GitHub OAuth
    re.compile(r'github_pat_[a-zA-Z0-9]{22}_[a-zA-Z0-9]{59}'),  # GitHub Fine-grained
    re.compile(r'sk-[a-zA-Z0-9]{48}'),  # OpenAI
    re.compile(r'sk-proj-[a-zA-Z0-9-_]{80,}'),  # OpenAI Project
    re.compile(r'sk_live_[a-zA-Z0-9]{24,}'),  # Stripe Live
    re.compile(r'sk_test_[a-zA-Z0-9]{24,}'),  # Stripe Test
    re.compile(r'AKIA[0-9A-Z]{16}'),  # AWS Access Key
    
    # Database connection strings
    re.compile(r'(?i)mongodb(\+srv)?://[^\s]+'),
    re.compile(r'(?i)postgres(ql)?://[^\s]+'),
    re.compile(r'(?i)mysql://[^\s]+'),
    re.compile(r'(?i)redis://[^\s]+'),
    
    # Private keys
    re.compile(r'-----BEGIN (RSA |DSA |EC |OPENSSH |PGP )?PRIVATE KEY-----'),
    
    # Email:password combos
    re.compile(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@]{4,}'),
    
    # Communication platform tokens
    re.compile(r'xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+'),  # Slack
    re.compile(r'[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27}'),  # Discord bot
    re.compile(r'[0-9]{17,19}:[A-Za-z0-9_-]{35}'),  # Telegram bot
    
    # Other services
    re.compile(r'SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}'),  # SendGrid
    re.compile(r'AIza[0-9A-Za-z_-]{35}'),  # Firebase/Google
    re.compile(r'eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*'),  # JWT
]

# Minimum counts for credential detection
MIN_EMAIL_PASS_COMBOS = 1  # Accept even single email:pass
MIN_CREDENTIAL_PATTERNS = 1  # Accept single API key/token

# File extensions that indicate credential files
CRED_FILE_EXTENSIONS = ['.txt', '.csv', '.json', '.sql', '.db', '.log', '.env']

def contains_credentials(text: str) -> bool:
    """Check if text contains potential credentials"""
    if not text:
        return False
    return any(pattern.search(text) for pattern in CREDENTIAL_PATTERNS)

# Leak-related keywords for additional filtering
LEAK_KEYWORDS = [
    'leak', 'leaked', 'dump', 'dumped', 'combo', 'combolist', 'breach', 'breached',
    'crack', 'cracked', 'hacked', 'stolen', 'exposed', 'database', 'db dump',
    'credential', 'password', 'passwd', 'login', 'account', 'config', 'log',
    'stealer', 'infostealer', 'redline', 'raccoon', 'lumma', 'vidar', 'stealc',
    'netflix', 'spotify', 'disney', 'hbo', 'amazon', 'prime', 'vpn', 'nord',
    'express', 'steam', 'fortnite', 'minecraft', 'roblox', 'epic', 'origin',
    'paypal', 'stripe', 'venmo', 'cashapp', 'crypto', 'bitcoin', 'wallet',
    'gmail', 'yahoo', 'outlook', 'hotmail', 'proton', 'mail.ru',
    'api key', 'apikey', 'token', 'secret', 'private key', 'ssh', 'ftp', 'smtp',
    'cpanel', 'rdp', 'shell', 'root', 'admin', 'panel', 'backdoor',
    'fresh', 'valid', 'checked', 'hits', 'capture', 'working', 'premium',
    'email:pass', 'user:pass', 'mail:pass', 'url:login:pass', 'ulp', 'combo',
    'bin', 'fullz', 'cc', 'cvv', 'ssn', 'dob', 'credit card',
]

def is_leak_content(text: str) -> bool:
    """
    Credential detection - requires actual credentials OR strong keyword signals.
    Lowered thresholds to catch single leaks.
    """
    if not text or len(text) < 50:  # Lower threshold
        return False
    
    lower = text.lower()
    
    # Count email:password combinations
    email_pass_pattern = re.compile(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s]{4,}')
    email_pass_count = len(email_pass_pattern.findall(text))
    
    # Count URL:login:pass format (stealer logs)
    ulp_pattern = re.compile(r'https?://[^\s]+[\s\t|:]+[^\s@]+[\s\t|:]+[^\s]{4,}')
    ulp_count = len(ulp_pattern.findall(text))
    
    # Count actual credential patterns (API keys, tokens, etc)
    cred_pattern_count = sum(1 for pattern in CREDENTIAL_PATTERNS if pattern.search(text))
    
    # Check for private keys
    has_private_key = '-----BEGIN' in text and 'PRIVATE KEY-----' in text
    
    # Check for keywords
    keyword_count = sum(1 for kw in LEAK_KEYWORDS if kw in lower)
    
    # Accept if:
    # 1. Any email:pass combo
    # 2. Any ULP format entry
    # 3. Any credential pattern (API key, token, etc)
    # 4. Private key
    # 5. 3+ leak keywords (suggests leak content even without parsed creds)
    return (
        email_pass_count >= MIN_EMAIL_PASS_COMBOS or
        ulp_count >= 1 or
        cred_pattern_count >= MIN_CREDENTIAL_PATTERNS or
        has_private_key or
        keyword_count >= 3
    )

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
        self.active_channels = []
        self.post_queue = asyncio.Queue()
        self.posts_made = 0
        self.channels_joined = 0
        
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
    
    async def scrape_channel(self, channel, limit=100):
        """Scrape recent messages from a channel"""
        name = getattr(channel, 'title', str(channel.id))
        
        try:
            count = 0
            async for message in self.client.iter_messages(channel, limit=limit):
                if message.id in self.processed_messages:
                    continue
                    
                text = message.text or ''
                
                # Also check for credential file attachments
                has_cred_file = False
                if message.document:
                    try:
                        filename = getattr(message.document.attributes[0], 'file_name', '') if message.document.attributes else ''
                        if any(filename.lower().endswith(ext) for ext in CRED_FILE_EXTENSIONS):
                            has_cred_file = True
                            text = f"[File: {filename}]\n{text}"
                    except:
                        pass
                
                if is_leak_content(text) or has_cred_file:
                    title = f"[TG] {name}: {text[:40]}..."
                    
                    # Add to queue for rate-limited posting
                    await self.post_queue.put((text, title))
                    self.processed_messages.add(message.id)
                    count += 1
                    
            if count > 0:
                logger.info(f"  Found {count} leak messages in {name}")
                    
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
            text = event.text or ''
            
            # Check for credential files
            has_cred_file = False
            if event.document:
                try:
                    filename = getattr(event.document.attributes[0], 'file_name', '') if event.document.attributes else ''
                    if any(filename.lower().endswith(ext) for ext in CRED_FILE_EXTENSIONS):
                        has_cred_file = True
                        text = f"[File: {filename}]\n{text}"
                except:
                    pass
            
            if is_leak_content(text) or has_cred_file:
                try:
                    chat = await event.get_chat()
                    chat_name = getattr(chat, 'title', None) or str(chat.id)
                    title = f"[TG] {chat_name}: {text[:40]}..."
                    
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
        
        # Start the post worker
        worker_task = asyncio.create_task(self.post_worker())
        
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
            
            # Wait for queue to empty
            while not self.post_queue.empty():
                await asyncio.sleep(1)
            
            logger.info(f"Initial scrape complete. Posted {self.posts_made} items.")
            
            # Then monitor in real-time
            await self.monitor_realtime()
            
        finally:
            worker_task.cancel()
        
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
