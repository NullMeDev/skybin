use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

/// Proxy type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ProxyType {
    Http,
    Https,
    Socks4,
    Socks5,
}

impl std::str::FromStr for ProxyType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "http" => Ok(ProxyType::Http),
            "https" => Ok(ProxyType::Https),
            "socks4" => Ok(ProxyType::Socks4),
            "socks5" => Ok(ProxyType::Socks5),
            _ => Err(()),
        }
    }
}

/// Represents a proxy server
#[derive(Debug, Clone)]
pub struct Proxy {
    pub protocol: ProxyType,
    pub host: String,
    pub port: u16,
    pub failures: u32,
}

impl Proxy {
    pub fn parse(proxy_str: &str) -> Option<Self> {
        // Format: protocol://host:port or host:port (defaults to http)
        let (protocol, rest) = if proxy_str.contains("://") {
            let parts: Vec<&str> = proxy_str.splitn(2, "://").collect();
            if parts.len() != 2 {
                return None;
            }
            (parts[0].parse().ok()?, parts[1])
        } else {
            (ProxyType::Http, proxy_str)
        };

        let parts: Vec<&str> = rest.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        let port: u16 = parts[0].parse().ok()?;
        let host = parts[1].to_string();

        Some(Proxy {
            protocol,
            host,
            port,
            failures: 0,
        })
    }

    pub fn url(&self) -> String {
        let proto = match self.protocol {
            ProxyType::Http => "http",
            ProxyType::Https => "https",
            ProxyType::Socks4 => "socks4",
            ProxyType::Socks5 => "socks5",
        };
        format!("{}://{}:{}", proto, self.host, self.port)
    }
}

/// Proxy rotator for distributing requests across multiple proxies
pub struct ProxyRotator {
    proxies: RwLock<Vec<Proxy>>,
    index: AtomicUsize,
    max_failures: u32,
}

impl ProxyRotator {
    pub fn new() -> Self {
        ProxyRotator {
            proxies: RwLock::new(Vec::new()),
            index: AtomicUsize::new(0),
            max_failures: 3,
        }
    }

    /// Load proxies from a list of strings
    pub fn load(&self, proxy_list: &[String]) {
        let mut proxies = self.proxies.write().unwrap();
        proxies.clear();
        for proxy_str in proxy_list {
            if let Some(proxy) = Proxy::parse(proxy_str) {
                proxies.push(proxy);
            }
        }
        tracing::info!("Loaded {} proxies", proxies.len());
    }

    /// Load proxies from a file (one per line)
    pub fn load_from_file(&self, path: &str) -> std::io::Result<()> {
        // Security: Canonicalize path to prevent traversal attacks
        let path_buf = std::path::PathBuf::from(path);
        let canonical_path = std::fs::canonicalize(&path_buf).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Invalid proxy file path: {}", e),
            )
        })?;

        // Security: Ensure file is .txt or no extension
        let ext = canonical_path.extension().and_then(|s| s.to_str());
        if ext.is_some() && ext != Some("txt") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Proxy file must be .txt or have no extension",
            ));
        }

        let content = std::fs::read_to_string(canonical_path)?;
        let list: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        self.load(&list);
        Ok(())
    }

    /// Get next proxy using round-robin
    pub fn next(&self) -> Option<Proxy> {
        let proxies = self.proxies.read().ok()?;
        if proxies.is_empty() {
            return None;
        }
        let idx = self.index.fetch_add(1, Ordering::SeqCst) % proxies.len();
        Some(proxies[idx].clone())
    }

    /// Mark a proxy as failed
    pub fn mark_failed(&self, proxy_url: &str) {
        if let Ok(mut proxies) = self.proxies.write() {
            for proxy in proxies.iter_mut() {
                if proxy.url() == proxy_url {
                    proxy.failures += 1;
                    if proxy.failures >= self.max_failures {
                        tracing::warn!("Proxy {} exceeded max failures, removing", proxy_url);
                    }
                    break;
                }
            }
            // Remove proxies with too many failures
            proxies.retain(|p| p.failures < self.max_failures);
        }
    }

    /// Get count of available proxies
    pub fn count(&self) -> usize {
        self.proxies.read().map(|p| p.len()).unwrap_or(0)
    }

    /// Check if proxies are available
    pub fn has_proxies(&self) -> bool {
        self.count() > 0
    }
}

impl Default for ProxyRotator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proxy() {
        let p = Proxy::parse("http://192.168.1.1:8080").unwrap();
        assert_eq!(p.protocol, ProxyType::Http);
        assert_eq!(p.host, "192.168.1.1");
        assert_eq!(p.port, 8080);

        let p = Proxy::parse("socks5://proxy.example.com:1080").unwrap();
        assert_eq!(p.protocol, ProxyType::Socks5);
        assert_eq!(p.host, "proxy.example.com");
        assert_eq!(p.port, 1080);

        // Default to HTTP
        let p = Proxy::parse("192.168.1.1:3128").unwrap();
        assert_eq!(p.protocol, ProxyType::Http);
    }

    #[test]
    fn test_rotator() {
        let rotator = ProxyRotator::new();
        rotator.load(&[
            "http://proxy1:8080".to_string(),
            "http://proxy2:8080".to_string(),
        ]);

        assert_eq!(rotator.count(), 2);

        // Round-robin
        let p1 = rotator.next().unwrap();
        let p2 = rotator.next().unwrap();
        let p3 = rotator.next().unwrap();

        assert_eq!(p1.host, "proxy1");
        assert_eq!(p2.host, "proxy2");
        assert_eq!(p3.host, "proxy1"); // Wraps around
    }
}
