use crate::models::Paste;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

/// Maximum number of messages to buffer in the broadcast channel
const CHANNEL_CAPACITY: usize = 1000;

/// Real-time event types broadcasted to WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RealtimeEvent {
    /// New paste added to the system
    PasteAdded {
        id: String,
        title: Option<String>,
        source: String,
        syntax: String,
        is_sensitive: bool,
        high_value: bool,
        created_at: i64,
        preview: String, // First 200 chars
    },
    /// Paste view count updated
    PasteViewed {
        id: String,
        view_count: i64,
    },
    /// System statistics updated
    StatsUpdate {
        total_pastes: i64,
        sensitive_pastes: i64,
        recent_24h: i64,
    },
    /// Heartbeat/keepalive
    Ping {
        timestamp: i64,
    },
}

impl RealtimeEvent {
    /// Create a paste added event from a Paste struct
    pub fn paste_added(paste: &Paste) -> Self {
        let preview = paste
            .content
            .chars()
            .take(200)
            .collect::<String>()
            .replace('\n', " ");

        Self::PasteAdded {
            id: paste.id.clone(),
            title: paste.title.clone(),
            source: paste.source.clone(),
            syntax: paste.syntax.clone(),
            is_sensitive: paste.is_sensitive,
            high_value: paste.high_value,
            created_at: paste.created_at,
            preview,
        }
    }

    /// Create a stats update event
    pub fn stats_update(total: i64, sensitive: i64, recent: i64) -> Self {
        Self::StatsUpdate {
            total_pastes: total,
            sensitive_pastes: sensitive,
            recent_24h: recent,
        }
    }

    /// Create a heartbeat event
    pub fn ping() -> Self {
        Self::Ping {
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Broadcast hub for real-time events
#[derive(Clone)]
pub struct RealtimeBroadcast {
    sender: broadcast::Sender<RealtimeEvent>,
    /// Track active connection count
    connection_count: Arc<RwLock<usize>>,
}

impl RealtimeBroadcast {
    /// Create a new broadcast hub
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender,
            connection_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Subscribe to real-time events
    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeEvent> {
        self.sender.subscribe()
    }

    /// Broadcast an event to all subscribers
    pub fn broadcast(&self, event: RealtimeEvent) {
        // Ignore send errors (no active subscribers)
        let _ = self.sender.send(event);
    }

    /// Increment active connection count
    pub async fn connect(&self) {
        let mut count = self.connection_count.write().await;
        *count += 1;
    }

    /// Decrement active connection count
    pub async fn disconnect(&self) {
        let mut count = self.connection_count.write().await;
        *count = count.saturating_sub(1);
    }

    /// Get active connection count
    pub async fn connection_count(&self) -> usize {
        *self.connection_count.read().await
    }
}

impl Default for RealtimeBroadcast {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket message filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketFilter {
    /// Only send sensitive pastes
    pub sensitive_only: Option<bool>,
    /// Only send high-value alerts
    pub high_value_only: Option<bool>,
    /// Filter by source
    pub source: Option<String>,
    /// Minimum quality score (0-100)
    pub min_quality: Option<u32>,
}

impl WebSocketFilter {
    /// Check if an event passes the filter
    pub fn matches(&self, event: &RealtimeEvent) -> bool {
        match event {
            RealtimeEvent::PasteAdded {
                is_sensitive,
                high_value,
                source,
                ..
            } => {
                // Check sensitive filter
                if let Some(sensitive_only) = self.sensitive_only {
                    if sensitive_only && !is_sensitive {
                        return false;
                    }
                }

                // Check high-value filter
                if let Some(high_value_only) = self.high_value_only {
                    if high_value_only && !high_value {
                        return false;
                    }
                }

                // Check source filter
                if let Some(ref filter_source) = self.source {
                    if source != filter_source {
                        return false;
                    }
                }

                true
            }
            // Always allow non-paste events (stats, ping)
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast_basic() {
        let broadcast = RealtimeBroadcast::new();
        let mut rx = broadcast.subscribe();

        broadcast.broadcast(RealtimeEvent::Ping { timestamp: 12345 });

        let event = rx.recv().await.unwrap();
        match event {
            RealtimeEvent::Ping { timestamp } => assert_eq!(timestamp, 12345),
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_connection_tracking() {
        let broadcast = RealtimeBroadcast::new();
        assert_eq!(broadcast.connection_count().await, 0);

        broadcast.connect().await;
        assert_eq!(broadcast.connection_count().await, 1);

        broadcast.connect().await;
        assert_eq!(broadcast.connection_count().await, 2);

        broadcast.disconnect().await;
        assert_eq!(broadcast.connection_count().await, 1);
    }

    #[test]
    fn test_filter_sensitive() {
        let filter = WebSocketFilter {
            sensitive_only: Some(true),
            high_value_only: None,
            source: None,
            min_quality: None,
        };

        let event = RealtimeEvent::PasteAdded {
            id: "test".into(),
            title: None,
            source: "pastebin".into(),
            syntax: "text".into(),
            is_sensitive: true,
            high_value: false,
            created_at: 0,
            preview: "test".into(),
        };

        assert!(filter.matches(&event));

        let non_sensitive = RealtimeEvent::PasteAdded {
            id: "test".into(),
            title: None,
            source: "pastebin".into(),
            syntax: "text".into(),
            is_sensitive: false,
            high_value: false,
            created_at: 0,
            preview: "test".into(),
        };

        assert!(!filter.matches(&non_sensitive));
    }
}
