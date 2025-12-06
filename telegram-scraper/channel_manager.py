#!/usr/bin/env python3
"""
Channel Management for Telegram Scraper

Features:
- Channel prioritization with scoring (activity, quality, success rate)
- Auto-discovery of new leak channels
- Channel health tracking
- Dynamic priority adjustment
"""

import json
import os
import asyncio
import logging
from datetime import datetime, timedelta
from typing import List, Dict, Set, Optional
from dataclasses import dataclass, asdict
from telethon import TelegramClient
from telethon.tl.functions.contacts import SearchRequest
from telethon.tl.functions.channels import GetFullChannelRequest
from telethon.errors import FloodWaitError

logger = logging.getLogger(__name__)

# Discovery keywords for finding new leak channels
DISCOVERY_KEYWORDS = [
    "logs", "leak", "leaks", "cloud", "combo", "combolist",
    "dump", "database", "breach", "stolen", "credentials",
    "passwords", "stealer", "redline", "raccoon", "vidar",
    "fresh", "free logs", "fresh logs", "txt base"
]

@dataclass
class ChannelMetrics:
    """Metrics for a single channel"""
    channel_id: str
    username: Optional[str]
    title: str
    member_count: int
    messages_scraped: int
    credentials_found: int
    last_activity: float  # timestamp
    success_rate: float  # 0-1
    priority_score: float  # computed score
    failures: int
    discovered_at: float
    
    def to_dict(self) -> dict:
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: dict) -> 'ChannelMetrics':
        return cls(**data)


class ChannelManager:
    """
    Manages Telegram channels with prioritization and auto-discovery
    """
    
    def __init__(self, metrics_file: str = "channel_metrics.json"):
        self.metrics_file = metrics_file
        self.channels: Dict[str, ChannelMetrics] = {}
        self.discovered_channels: Set[str] = set()
        self.load_metrics()
    
    def load_metrics(self):
        """Load channel metrics from disk"""
        if os.path.exists(self.metrics_file):
            try:
                with open(self.metrics_file, 'r') as f:
                    data = json.load(f)
                    for channel_id, metrics in data.items():
                        self.channels[channel_id] = ChannelMetrics.from_dict(metrics)
                logger.info(f"Loaded metrics for {len(self.channels)} channels")
            except Exception as e:
                logger.error(f"Failed to load channel metrics: {e}")
    
    def save_metrics(self):
        """Save channel metrics to disk"""
        try:
            data = {cid: metrics.to_dict() for cid, metrics in self.channels.items()}
            with open(self.metrics_file, 'w') as f:
                json.dump(data, f, indent=2)
        except Exception as e:
            logger.error(f"Failed to save channel metrics: {e}")
    
    def update_metrics(self, channel_id: str, username: Optional[str], title: str,
                      member_count: int, found_credentials: bool, success: bool):
        """Update metrics for a channel after scraping"""
        now = datetime.now().timestamp()
        
        if channel_id not in self.channels:
            # New channel discovered
            self.channels[channel_id] = ChannelMetrics(
                channel_id=channel_id,
                username=username,
                title=title,
                member_count=member_count,
                messages_scraped=0,
                credentials_found=0,
                last_activity=now,
                success_rate=1.0 if success else 0.0,
                priority_score=0.0,
                failures=0 if success else 1,
                discovered_at=now
            )
        
        metrics = self.channels[channel_id]
        metrics.messages_scraped += 1
        metrics.last_activity = now
        metrics.member_count = member_count
        
        if found_credentials:
            metrics.credentials_found += 1
        
        if not success:
            metrics.failures += 1
        
        # Update success rate (exponential moving average)
        alpha = 0.1
        metrics.success_rate = (alpha * (1.0 if success else 0.0)) + ((1 - alpha) * metrics.success_rate)
        
        # Recompute priority score
        metrics.priority_score = self._compute_priority(metrics)
        
        self.save_metrics()
    
    def _compute_priority(self, metrics: ChannelMetrics) -> float:
        """
        Compute priority score for a channel
        
        Factors:
        - Credential yield rate (40%)
        - Success rate (30%)
        - Member count (15%)
        - Recent activity (15%)
        
        Returns: 0-100 score
        """
        # Credential yield rate
        if metrics.messages_scraped > 0:
            yield_rate = metrics.credentials_found / metrics.messages_scraped
        else:
            yield_rate = 0.0
        
        # Normalize yield (cap at 50% for scoring purposes)
        yield_score = min(yield_rate * 2, 1.0)
        
        # Success rate
        success_score = metrics.success_rate
        
        # Member count (logarithmic, capped at 1M)
        import math
        if metrics.member_count > 0:
            # log10 scale: 1K = 0.3, 10K = 0.4, 100K = 0.5, 1M = 0.6
            member_score = math.log10(metrics.member_count) / 6.0
        else:
            member_score = 0.0
        member_score = min(member_score, 1.0)
        
        # Recent activity (penalize stale channels)
        now = datetime.now().timestamp()
        hours_since = (now - metrics.last_activity) / 3600
        if hours_since < 24:
            recency_score = 1.0
        elif hours_since < 168:  # 1 week
            recency_score = 0.5
        else:
            recency_score = 0.2
        
        # Weighted score
        score = (
            yield_score * 0.40 +
            success_score * 0.30 +
            member_score * 0.15 +
            recency_score * 0.15
        )
        
        return score * 100  # 0-100 scale
    
    def get_prioritized_channels(self, min_score: float = 20.0) -> List[str]:
        """Get list of channels sorted by priority score"""
        sorted_channels = sorted(
            self.channels.items(),
            key=lambda x: x[1].priority_score,
            reverse=True
        )
        
        return [
            cid for cid, metrics in sorted_channels
            if metrics.priority_score >= min_score
        ]
    
    def get_top_channels(self, n: int = 10) -> List[ChannelMetrics]:
        """Get top N channels by priority"""
        sorted_channels = sorted(
            self.channels.values(),
            key=lambda x: x.priority_score,
            reverse=True
        )
        return sorted_channels[:n]
    
    def get_stale_channels(self, days: int = 7) -> List[ChannelMetrics]:
        """Get channels that haven't been active in N days"""
        cutoff = datetime.now().timestamp() - (days * 86400)
        return [
            metrics for metrics in self.channels.values()
            if metrics.last_activity < cutoff
        ]
    
    async def discover_channels(self, client: TelegramClient, 
                               max_results: int = 20) -> List[str]:
        """
        Auto-discover new leak channels using keyword search
        
        Returns: List of newly discovered channel usernames
        """
        new_channels = []
        
        for keyword in DISCOVERY_KEYWORDS:
            try:
                # Search for channels with this keyword
                result = await client(SearchRequest(
                    q=keyword,
                    limit=10
                ))
                
                for peer in result.results:
                    try:
                        # Get channel info
                        entity = await client.get_entity(peer)
                        
                        # Only process channels (not users or groups)
                        if not hasattr(entity, 'username') or not entity.username:
                            continue
                        
                        username = entity.username
                        channel_id = str(entity.id)
                        
                        # Skip if already known
                        if channel_id in self.channels or username in self.discovered_channels:
                            continue
                        
                        # Heuristic: check if channel title suggests leaks
                        title_lower = entity.title.lower() if hasattr(entity, 'title') else ""
                        leak_keywords = ['log', 'leak', 'cloud', 'combo', 'dump', 'breach']
                        
                        if any(kw in title_lower for kw in leak_keywords):
                            new_channels.append(username)
                            self.discovered_channels.add(username)
                            logger.info(f"🔍 Discovered new channel: {username} ({entity.title})")
                            
                            if len(new_channels) >= max_results:
                                break
                    
                    except Exception as e:
                        logger.debug(f"Error processing search result: {e}")
                        continue
                
                # Rate limit between searches
                await asyncio.sleep(2)
                
                if len(new_channels) >= max_results:
                    break
            
            except FloodWaitError as e:
                logger.warning(f"Flood wait during discovery: {e.seconds}s")
                await asyncio.sleep(e.seconds)
            except Exception as e:
                logger.error(f"Error during channel discovery: {e}")
        
        # Save discovered channels to file
        if new_channels:
            self._save_discovered_channels(new_channels)
        
        return new_channels
    
    def _save_discovered_channels(self, channels: List[str]):
        """Append newly discovered channels to a file"""
        try:
            with open("discovered_channels.txt", "a") as f:
                for channel in channels:
                    f.write(f"{channel}\n")
            logger.info(f"Saved {len(channels)} discovered channels to discovered_channels.txt")
        except Exception as e:
            logger.error(f"Failed to save discovered channels: {e}")
    
    def export_health_report(self, output_file: str = "channel_health.json"):
        """Export comprehensive health report for all channels"""
        report = {
            "generated_at": datetime.now().isoformat(),
            "total_channels": len(self.channels),
            "top_channels": [
                {
                    "username": m.username,
                    "title": m.title,
                    "priority_score": m.priority_score,
                    "credentials_found": m.credentials_found,
                    "success_rate": m.success_rate,
                    "member_count": m.member_count,
                }
                for m in self.get_top_channels(20)
            ],
            "stale_channels": [
                {
                    "username": m.username,
                    "title": m.title,
                    "last_activity": datetime.fromtimestamp(m.last_activity).isoformat(),
                }
                for m in self.get_stale_channels(7)
            ]
        }
        
        try:
            with open(output_file, 'w') as f:
                json.dump(report, f, indent=2)
            logger.info(f"Exported channel health report to {output_file}")
        except Exception as e:
            logger.error(f"Failed to export health report: {e}")
