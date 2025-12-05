use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use chrono::{DateTime, Utc};

/// Global statistics for the scraper
#[derive(Debug)]
pub struct Stats {
    // Counters
    pub files_processed: AtomicU64,
    pub files_posted: AtomicU64,
    pub files_skipped_duplicate: AtomicU64,
    pub files_skipped_no_password: AtomicU64,
    pub archives_extracted: AtomicU64,
    pub nested_archives: AtomicU64,
    pub errors: AtomicU64,
    pub flood_waits: AtomicU64,
    
    // Channel stats
    pub channels_monitored: AtomicU64,
    pub channels_joined: AtomicU64,
    pub messages_received: AtomicU64,
    
    // Queue
    pub queue_depth: AtomicU64,
    
    // Per-channel stats
    pub channel_stats: RwLock<HashMap<String, ChannelStats>>,
    
    // Start time
    pub started_at: DateTime<Utc>,
    
    // Recent errors
    pub recent_errors: RwLock<Vec<ErrorEntry>>,
    
    // Pending invite links to join
    pub pending_invites: RwLock<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    pub name: String,
    pub files_received: u64,
    pub files_posted: u64,
    pub last_message: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub context: Option<String>,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new()
    }
}

impl Stats {
    pub fn new() -> Self {
        Self {
            files_processed: AtomicU64::new(0),
            files_posted: AtomicU64::new(0),
            files_skipped_duplicate: AtomicU64::new(0),
            files_skipped_no_password: AtomicU64::new(0),
            archives_extracted: AtomicU64::new(0),
            nested_archives: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            flood_waits: AtomicU64::new(0),
            channels_monitored: AtomicU64::new(0),
            channels_joined: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            queue_depth: AtomicU64::new(0),
            channel_stats: RwLock::new(HashMap::new()),
            started_at: Utc::now(),
            recent_errors: RwLock::new(Vec::new()),
            pending_invites: RwLock::new(Vec::new()),
        }
    }
    
    pub fn inc_files_processed(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_files_posted(&self) {
        self.files_posted.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_skipped_duplicate(&self) {
        self.files_skipped_duplicate.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_skipped_no_password(&self) {
        self.files_skipped_no_password.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_archives_extracted(&self) {
        self.archives_extracted.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_nested_archives(&self) {
        self.nested_archives.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_flood_waits(&self) {
        self.flood_waits.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_messages(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn set_channels_monitored(&self, count: u64) {
        self.channels_monitored.store(count, Ordering::Relaxed);
    }
    
    pub fn inc_channels_joined(&self) {
        self.channels_joined.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn set_queue_depth(&self, depth: u64) {
        self.queue_depth.store(depth, Ordering::Relaxed);
    }
    
    pub fn add_error(&self, message: String, context: Option<String>) {
        let mut errors = self.recent_errors.write();
        errors.push(ErrorEntry {
            timestamp: Utc::now(),
            message,
            context,
        });
        // Keep only last 100 errors
        if errors.len() > 100 {
            errors.remove(0);
        }
    }
    
    pub fn update_channel(&self, channel_name: &str, posted: bool) {
        let mut stats = self.channel_stats.write();
        let entry = stats.entry(channel_name.to_string()).or_insert_with(|| ChannelStats {
            name: channel_name.to_string(),
            files_received: 0,
            files_posted: 0,
            last_message: None,
        });
        entry.files_received += 1;
        if posted {
            entry.files_posted += 1;
        }
        entry.last_message = Some(Utc::now());
    }
    
    pub fn add_pending_invite(&self, invite: String) {
        let mut invites = self.pending_invites.write();
        if !invites.contains(&invite) {
            invites.push(invite);
        }
    }
    
    pub fn take_pending_invites(&self) -> Vec<String> {
        let mut invites = self.pending_invites.write();
        std::mem::take(&mut *invites)
    }
    
    pub fn to_json(&self) -> StatsResponse {
        let channel_stats: Vec<ChannelStats> = self.channel_stats.read().values().cloned().collect();
        let recent_errors: Vec<ErrorEntry> = self.recent_errors.read().iter().rev().take(20).cloned().collect();
        let uptime_seconds = (Utc::now() - self.started_at).num_seconds() as u64;
        
        StatsResponse {
            uptime_seconds,
            started_at: self.started_at,
            files_processed: self.files_processed.load(Ordering::Relaxed),
            files_posted: self.files_posted.load(Ordering::Relaxed),
            files_skipped_duplicate: self.files_skipped_duplicate.load(Ordering::Relaxed),
            files_skipped_no_password: self.files_skipped_no_password.load(Ordering::Relaxed),
            archives_extracted: self.archives_extracted.load(Ordering::Relaxed),
            nested_archives: self.nested_archives.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            flood_waits: self.flood_waits.load(Ordering::Relaxed),
            channels_monitored: self.channels_monitored.load(Ordering::Relaxed),
            channels_joined: self.channels_joined.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            queue_depth: self.queue_depth.load(Ordering::Relaxed),
            channel_stats,
            recent_errors,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub uptime_seconds: u64,
    pub started_at: DateTime<Utc>,
    pub files_processed: u64,
    pub files_posted: u64,
    pub files_skipped_duplicate: u64,
    pub files_skipped_no_password: u64,
    pub archives_extracted: u64,
    pub nested_archives: u64,
    pub errors: u64,
    pub flood_waits: u64,
    pub channels_monitored: u64,
    pub channels_joined: u64,
    pub messages_received: u64,
    pub queue_depth: u64,
    pub channel_stats: Vec<ChannelStats>,
    pub recent_errors: Vec<ErrorEntry>,
}

/// Shared stats handle
pub type SharedStats = Arc<Stats>;

pub fn new_shared() -> SharedStats {
    Arc::new(Stats::new())
}
