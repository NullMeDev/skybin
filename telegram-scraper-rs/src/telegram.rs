use grammers_client::{Client, Config, InitParams, Update};
use grammers_session::Session;
use grammers_tl_types as tl;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{info, warn, error, debug};

use crate::archive::{extract_password_files, is_archive};
use crate::config::Config as AppConfig;
use crate::skybin::SkybinClient;
use crate::stats::SharedStats;

/// File processing message
pub struct FileMessage {
    pub data: Vec<u8>,
    pub filename: String,
    pub channel_name: String,
    pub message_id: i32,
}

pub struct TelegramScraper {
    client: Client,
    config: AppConfig,
    stats: SharedStats,
    skybin: SkybinClient,
    file_tx: mpsc::Sender<FileMessage>,
}

impl TelegramScraper {
    /// Connect to Telegram and authenticate
    pub async fn connect(config: AppConfig, stats: SharedStats) -> anyhow::Result<(Self, mpsc::Receiver<FileMessage>)> {
        info!("🔌 Connecting to Telegram...");
        
        let session = if Path::new(&config.telegram.session_file).exists() {
            Session::load_file(&config.telegram.session_file)?
        } else {
            Session::new()
        };
        
        let client = Client::connect(Config {
            session,
            api_id: config.telegram.api_id,
            api_hash: config.telegram.api_hash.clone(),
            params: InitParams {
                app_version: "2.0.0".into(),
                device_model: "SkyBin Scraper".into(),
                system_version: "Linux".into(),
                ..Default::default()
            },
        }).await?;
        
        // Handle authentication
        if !client.is_authorized().await? {
            info!("📱 Authentication required...");
            
            let token = client.request_login_code(&config.telegram.phone).await?;
            
            print!("Enter the code you received: ");
            io::stdout().flush()?;
            let mut code = String::new();
            io::stdin().lock().read_line(&mut code)?;
            let code = code.trim();
            
            match client.sign_in(&token, code).await {
                Ok(_user) => {
                    info!("✅ Signed in successfully");
                }
                Err(grammers_client::SignInError::PasswordRequired(password_token)) => {
                    print!("Enter your 2FA password: ");
                    io::stdout().flush()?;
                    let mut password = String::new();
                    io::stdin().lock().read_line(&mut password)?;
                    let password = password.trim();
                    
                    client.check_password(password_token, password).await?;
                    info!("✅ Signed in with 2FA");
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Sign in failed: {}", e));
                }
            }
            
            // Save session
            client.session().save_to_file(&config.telegram.session_file)?;
        }
        
        info!("✅ Connected to Telegram");
        
        let skybin = SkybinClient::new(&config.skybin.api_url, config.skybin.check_duplicates);
        let (file_tx, file_rx) = mpsc::channel(100);
        
        Ok((Self {
            client,
            config,
            stats,
            skybin,
            file_tx,
        }, file_rx))
    }
    
    /// Join configured channels
    pub async fn join_channels(&self) -> anyhow::Result<()> {
        info!("📡 Joining {} configured channels...", self.config.channels.len());
        
        for channel in &self.config.channels {
            self.join_channel(channel).await;
            sleep(Duration::from_secs(2)).await; // Rate limit
        }
        
        Ok(())
    }
    
    /// Join a single channel by username or invite hash
    async fn join_channel(&self, channel: &str) {
        let channel = channel.trim().trim_start_matches('@');
        
        // Check if it's an invite hash (long alphanumeric) or username
        if channel.len() > 15 && !channel.contains('/') {
            // Invite hash
            match self.join_by_invite(channel).await {
                Ok(_) => {
                    info!("  ✓ Joined via invite: {}...", &channel[..8.min(channel.len())]);
                    self.stats.inc_channels_joined();
                }
                Err(e) => {
                    warn!("  ✗ Failed to join invite {}: {}", &channel[..8.min(channel.len())], e);
                }
            }
        } else {
            // Username
            match self.join_by_username(channel).await {
                Ok(_) => {
                    info!("  ✓ Joined @{}", channel);
                    self.stats.inc_channels_joined();
                }
                Err(e) => {
                    warn!("  ✗ Failed to join @{}: {}", channel, e);
                }
            }
        }
    }
    
    async fn join_by_username(&self, username: &str) -> anyhow::Result<()> {
        let resolved = self.client.resolve_username(username).await?;
        if let Some(chat) = resolved {
            // Try to join if it's a channel/group
            let input_channel = match chat.pack().to_input_peer() {
                tl::enums::InputPeer::Channel(c) => tl::types::InputChannel {
                    channel_id: c.channel_id,
                    access_hash: c.access_hash,
                },
                _ => return Err(anyhow::anyhow!("Not a channel")),
            };
            
            if let Err(e) = self.client.invoke(&tl::functions::channels::JoinChannel {
                channel: tl::enums::InputChannel::Channel(input_channel),
            }).await {
                // Ignore "already participant" errors
                let err_str = format!("{:?}", e);
                if !err_str.contains("USER_ALREADY_PARTICIPANT") {
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }
    
    async fn join_by_invite(&self, hash: &str) -> anyhow::Result<()> {
        if let Err(e) = self.client.invoke(&tl::functions::messages::ImportChatInvite {
            hash: hash.to_string(),
        }).await {
            let err_str = format!("{:?}", e);
            if !err_str.contains("USER_ALREADY_PARTICIPANT") {
                return Err(e.into());
            }
        }
        Ok(())
    }
    
    /// Process pending invite links from SkyBin
    pub async fn process_pending_invites(&self) {
        let invites = self.stats.take_pending_invites();
        
        for invite in invites {
            info!("🔗 Processing pending invite: {}", invite);
            self.join_channel(&invite).await;
            sleep(Duration::from_secs(3)).await;
        }
    }
    
    /// Start listening for new messages in real-time
    pub async fn start_realtime_listener(self: Arc<Self>) -> anyhow::Result<()> {
        info!("👂 Starting real-time message listener...");
        
        // Get list of dialogs to count channels
        let mut channel_count = 0u64;
        let mut dialogs = self.client.iter_dialogs();
        while let Some(dialog) = dialogs.next().await? {
            if dialog.chat().pack().is_channel() {
                channel_count += 1;
            }
        }
        self.stats.set_channels_monitored(channel_count);
        info!("📊 Monitoring {} channels/groups", channel_count);
        
        // Listen for updates
        loop {
            match self.client.next_update().await {
                Ok(update) => {
                    self.handle_update(update).await;
                }
                Err(e) => {
                    error!("Update error: {}", e);
                    self.stats.inc_errors();
                    
                    // Check for flood wait
                    let err_str = format!("{:?}", e);
                    if err_str.contains("FLOOD_WAIT") {
                        self.stats.inc_flood_waits();
                        // Extract wait time and sleep
                        if let Some(secs) = extract_flood_wait_seconds(&err_str) {
                            let wait = secs.min(self.config.scraper.backoff_max_seconds);
                            warn!("⏳ Flood wait: sleeping {}s", wait);
                            sleep(Duration::from_secs(wait)).await;
                        }
                    }
                }
            }
            
            // Process any pending invites periodically
            self.process_pending_invites().await;
        }
    }
    
    async fn handle_update(&self, update: Update) {
        match update {
            Update::NewMessage(message) if !message.outgoing() => {
                self.stats.inc_messages();
                
                // Check if message has a document (file)
                if let Some(media) = message.media() {
                    if let grammers_client::types::Media::Document(doc) = media {
                        let filename = doc.name().to_string();
                        
                        if !filename.is_empty() && is_archive(&filename) {
                            let size = doc.size() as u64;
                            let max_size = self.config.scraper.max_archive_size_mb * 1024 * 1024;
                            
                            if size <= max_size {
                                let channel_name = message.chat().name().to_string();
                                info!("📦 New archive in {}: {} ({:.1} MB)", 
                                    channel_name, filename, size as f64 / 1024.0 / 1024.0);
                                
                                // Download and process
                                self.download_and_process(&message, &filename, &channel_name).await;
                            } else {
                                debug!("Skipping large archive: {} ({:.1} MB)", filename, size as f64 / 1024.0 / 1024.0);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    async fn download_and_process(
        &self,
        message: &grammers_client::types::Message,
        filename: &str,
        channel_name: &str,
    ) {
        self.stats.inc_files_processed();
        
        // Download file
        let media = match message.media() {
            Some(m) => m,
            None => {
                warn!("  ❌ No media in message");
                return;
            }
        };
        
        let mut data = Vec::new();
        
        // Convert Media to Downloadable
        let downloadable = grammers_client::types::Downloadable::Media(media);
        let mut download = self.client.iter_download(&downloadable);
        
        loop {
            match download.next().await {
                Ok(Some(chunk)) => data.extend(chunk),
                Ok(None) => break,
                Err(e) => {
                    warn!("  ❌ Download error: {}", e);
                    self.stats.inc_errors();
                    return;
                }
            }
        }
        
        if data.is_empty() {
            warn!("  ❌ Download failed or empty file");
            self.stats.inc_errors();
            return;
        }
        
        info!("  📥 Downloaded {} bytes", data.len());
        self.stats.inc_archives_extracted();
        
        // Extract password files
        let max_file_size = self.config.scraper.max_file_size_mb;
        let password_files = extract_password_files(&data, filename, max_file_size, 0);
        
        if password_files.is_empty() {
            debug!("  No password files found in {}", filename);
            self.stats.inc_skipped_no_password();
            self.stats.update_channel(channel_name, false);
            return;
        }
        
        info!("  🔑 Found {} password file(s)", password_files.len());
        
        // Post each password file
        for pf in password_files {
            match self.skybin.post_paste(&pf, channel_name, &self.stats).await {
                Ok(_) => {
                    self.stats.update_channel(channel_name, true);
                }
                Err(e) => {
                    if !e.contains("Duplicate") {
                        self.stats.inc_errors();
                        self.stats.add_error(e, Some(filename.to_string()));
                    }
                    self.stats.update_channel(channel_name, false);
                }
            }
            
            // Rate limit between posts
            sleep(Duration::from_millis(self.config.scraper.rate_limit_delay_ms)).await;
        }
    }
}

/// Extract seconds from flood wait error message
fn extract_flood_wait_seconds(err: &str) -> Option<u64> {
    // Try to find a number in the error
    for word in err.split_whitespace() {
        if let Ok(n) = word.trim_matches(|c: char| !c.is_numeric()).parse::<u64>() {
            if n > 0 && n < 86400 {
                return Some(n);
            }
        }
    }
    Some(60) // Default to 60 seconds
}
