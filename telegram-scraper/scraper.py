#!/usr/bin/env python3
"""
SkyBin Telegram Scraper Service
Monitors Telegram channels for credential leaks and posts to SkyBin API
"""

import asyncio
import re
import os
import json
import logging
import aiohttp
from datetime import datetime, timedelta
from telethon import TelegramClient, events
from telethon.tl.types import Channel, Chat

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Configuration from environment
API_ID = os.getenv('TELEGRAM_API_ID')
API_HASH = os.getenv('TELEGRAM_API_HASH')
BOT_TOKEN = os.getenv('TELEGRAM_BOT_TOKEN')
SKYBIN_API = os.getenv('SKYBIN_API_URL', 'http://localhost:8082/api')
SESSION_NAME = os.getenv('SESSION_NAME', 'skybin_scraper')

# Channels to monitor (usernames or IDs)
LEAK_CHANNELS = [
    # Add channel usernames or IDs here
    # '@leaksdb',
    # '@combocloud', 
    # '@daboross',
]

# Credential detection patterns
CREDENTIAL_PATTERNS = [
    re.compile(r'(?i)(api[_-]?key|apikey)\s*[=:]\s*[\'"]?[a-zA-Z0-9_-]{16,}'),
    re.compile(r'(?i)(secret[_-]?key|secretkey)\s*[=:]\s*[\'"]?[a-zA-Z0-9_-]{16,}'),
    re.compile(r'(?i)(access[_-]?token|accesstoken)\s*[=:]\s*[\'"]?[a-zA-Z0-9_-]{16,}'),
    re.compile(r'(?i)(password|passwd|pwd)\s*[=:]\s*[\'"]?[^\s\'"]{6,}'),
    re.compile(r'ghp_[a-zA-Z0-9]{36}'),  # GitHub PAT
    re.compile(r'gho_[a-zA-Z0-9]{36}'),  # GitHub OAuth
    re.compile(r'sk-[a-zA-Z0-9]{48}'),  # OpenAI
    re.compile(r'sk_live_[a-zA-Z0-9]{24,}'),  # Stripe
    re.compile(r'AKIA[0-9A-Z]{16}'),  # AWS Access Key
    re.compile(r'(?i)mongodb(\+srv)?://[^\s]+'),  # MongoDB
    re.compile(r'(?i)postgres(ql)?://[^\s]+'),  # PostgreSQL
    re.compile(r'(?i)mysql://[^\s]+'),  # MySQL
    re.compile(r'-----BEGIN (RSA |DSA |EC |OPENSSH )?PRIVATE KEY-----'),
    re.compile(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+:[^\s@]{4,}'),  # email:password
    re.compile(r'xox[baprs]-[0-9]{10,}-[a-zA-Z0-9-]+'),  # Slack
    re.compile(r'[MN][A-Za-z0-9]{23,}\.[A-Za-z0-9_-]{6}\.[A-Za-z0-9_-]{27}'),  # Discord bot
    re.compile(r'SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}'),  # SendGrid
    re.compile(r'AIza[0-9A-Za-z_-]{35}'),  # Firebase
    re.compile(r'eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*'),  # JWT
]

def contains_credentials(text: str) -> bool:
    """Check if text contains potential credentials"""
    if not text:
        return False
    return any(pattern.search(text) for pattern in CREDENTIAL_PATTERNS)

def is_leak_content(text: str) -> bool:
    """Check if message appears to be leak/credential content"""
    if not text:
        return False
    lower = text.lower()
    leak_keywords = [
        'leak', 'dump', 'combo', 'credential', 'password', 'account',
        'database', 'breach', 'hacked', 'cracked', 'premium', 'free',
        'netflix', 'spotify', 'disney', 'hbo', 'vpn', 'steam',
        'api key', 'token', 'secret', 'login', 'email:pass'
    ]
    return any(kw in lower for kw in leak_keywords) or contains_credentials(text)

async def post_to_skybin(content: str, title: str, source_url: str):
    """Post discovered content to SkyBin API"""
    try:
        async with aiohttp.ClientSession() as session:
            payload = {
                'content': content,
                'title': title[:100] if title else 'Telegram Leak',
                'syntax': 'plaintext',
            }
            async with session.post(
                f'{SKYBIN_API}/paste',
                json=payload,
                timeout=aiohttp.ClientTimeout(total=30)
            ) as resp:
                if resp.status == 200 or resp.status == 201:
                    data = await resp.json()
                    logger.info(f"Posted to SkyBin: {data.get('data', {}).get('id', 'unknown')}")
                    return True
                else:
                    logger.warning(f"Failed to post to SkyBin: {resp.status}")
                    return False
    except Exception as e:
        logger.error(f"Error posting to SkyBin: {e}")
        return False

class TelegramScraper:
    def __init__(self):
        self.client = None
        self.processed_messages = set()
        
    async def start(self):
        """Initialize and start the Telegram client"""
        if not API_ID or not API_HASH:
            logger.error("TELEGRAM_API_ID and TELEGRAM_API_HASH are required")
            return False
            
        self.client = TelegramClient(SESSION_NAME, int(API_ID), API_HASH)
        
        if BOT_TOKEN:
            await self.client.start(bot_token=BOT_TOKEN)
            logger.info("Started as bot")
        else:
            await self.client.start()
            logger.info("Started as user")
            
        return True
        
    async def join_channels(self):
        """Join configured leak channels"""
        for channel in LEAK_CHANNELS:
            try:
                entity = await self.client.get_entity(channel)
                logger.info(f"Joined/Found channel: {channel}")
            except Exception as e:
                logger.warning(f"Could not join {channel}: {e}")
                
    async def scrape_channel_history(self, channel, limit=100):
        """Scrape recent messages from a channel"""
        try:
            entity = await self.client.get_entity(channel)
            logger.info(f"Scraping history from: {channel}")
            
            async for message in self.client.iter_messages(entity, limit=limit):
                if message.id in self.processed_messages:
                    continue
                    
                text = message.text or ''
                if message.document and hasattr(message.document, 'attributes'):
                    # Handle file attachments (potential combo lists)
                    pass
                    
                if is_leak_content(text):
                    title = text[:50].split('\n')[0] if text else 'Telegram Leak'
                    source_url = f"https://t.me/{channel}/{message.id}"
                    
                    await post_to_skybin(text, title, source_url)
                    self.processed_messages.add(message.id)
                    logger.info(f"Found leak in {channel}: {title[:30]}...")
                    
        except Exception as e:
            logger.error(f"Error scraping {channel}: {e}")
            
    async def monitor_realtime(self):
        """Monitor channels for new messages in real-time"""
        @self.client.on(events.NewMessage(chats=LEAK_CHANNELS))
        async def handler(event):
            text = event.text or ''
            if is_leak_content(text):
                chat = await event.get_chat()
                chat_name = getattr(chat, 'username', None) or str(chat.id)
                title = text[:50].split('\n')[0] if text else 'Telegram Leak'
                source_url = f"https://t.me/{chat_name}/{event.id}"
                
                await post_to_skybin(text, title, source_url)
                logger.info(f"Real-time leak from {chat_name}: {title[:30]}...")
                
        logger.info("Real-time monitoring started")
        await self.client.run_until_disconnected()
        
    async def run(self):
        """Main run loop"""
        if not await self.start():
            return
            
        await self.join_channels()
        
        # Scrape recent history first
        for channel in LEAK_CHANNELS:
            await self.scrape_channel_history(channel, limit=50)
            await asyncio.sleep(2)  # Rate limit
            
        # Then monitor in real-time
        await self.monitor_realtime()
        
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
