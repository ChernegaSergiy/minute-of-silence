//! NTP service with caching and periodic synchronization.
//!
//! Provides a clean service interface for NTP time synchronization,
//! supporting cached offsets and configurable sync intervals.

use chrono::{DateTime, Local};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::{AppError, Result};

const DEFAULT_SYNC_INTERVAL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone)]
pub struct NtpService {
    server: Arc<Mutex<String>>,
    cached_offset: Arc<Mutex<Option<i64>>>,
    last_sync: Arc<Mutex<Option<DateTime<Local>>>>,
    generation: Arc<AtomicU64>,
    sync_interval: Duration,
}

impl NtpService {
    pub fn new(server: String) -> Self {
        Self {
            server: Arc::new(Mutex::new(server)),
            cached_offset: Arc::new(Mutex::new(None)),
            last_sync: Arc::new(Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            sync_interval: DEFAULT_SYNC_INTERVAL,
        }
    }

    pub async fn sync(&self) -> Result<i64> {
        let start_generation = self.generation.load(Ordering::SeqCst);
        let offset = self.sync_once().await?;
        if self.generation.load(Ordering::SeqCst) == start_generation {
            self.set_cached(offset);
            self.set_last_sync(Local::now());
            Ok(offset)
        } else {
            Err(AppError::Ntp(
                "Server configuration changed during sync".to_string(),
            ))
        }
    }

    pub fn get_offset(&self) -> Option<i64> {
        *self.cached_offset.lock().unwrap()
    }

    pub fn should_sync(&self) -> bool {
        if let Some(last) = self.last_sync_time() {
            let elapsed = Local::now() - last;
            elapsed.num_seconds() as u64 >= self.sync_interval.as_secs()
        } else {
            true
        }
    }

    pub fn last_sync_time(&self) -> Option<DateTime<Local>> {
        *self.last_sync.lock().unwrap()
    }

    pub fn is_synced(&self) -> bool {
        self.cached_offset.lock().unwrap().is_some()
    }

    pub fn update_cached(&self, offset: i64) {
        self.set_cached(offset);
        self.set_last_sync(Local::now());
    }

    pub fn clear_cache(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.cached_offset.lock().unwrap() = None;
        *self.last_sync.lock().unwrap() = None;
    }

    pub fn set_server(&self, server: String) {
        let mut current = self.server.lock().unwrap();
        if *current != server {
            *current = server;
            self.clear_cache();
        }
    }

    pub fn server(&self) -> String {
        self.server.lock().unwrap().clone()
    }

    async fn sync_once(&self) -> Result<i64> {
        let server = self.server();
        println!("NTP: Attempting to sync with server: {}", server);
        tokio::task::spawn_blocking(move || {
            let result = rsntp::SntpClient::new().synchronize(&server).map_err(|e| {
                println!("NTP: Sync error: {}", e);
                AppError::Ntp(e.to_string())
            })?;
            let offset_ms = result.clock_offset().as_secs_f64() * 1000.0;
            println!("NTP: Sync successful, offset: {}ms", offset_ms);
            Ok(offset_ms as i64)
        })
        .await
        .map_err(|e| AppError::Ntp(e.to_string()))?
    }

    fn set_cached(&self, offset: i64) {
        *self.cached_offset.lock().unwrap() = Some(offset);
    }

    fn set_last_sync(&self, time: DateTime<Local>) {
        *self.last_sync.lock().unwrap() = Some(time);
    }
}

impl Default for NtpService {
    fn default() -> Self {
        Self::new("pool.ntp.org".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntp_service_initialization() {
        let service = NtpService::new("pool.ntp.org".to_string());
        assert!(!service.is_synced());
        assert!(service.last_sync_time().is_none());
    }

    #[test]
    fn ntp_service_set_server() {
        let service = NtpService::new("pool.ntp.org".to_string());
        assert_eq!(service.server(), "pool.ntp.org");
        service.update_cached(100);
        assert!(service.is_synced());

        service.set_server("time.cloudflare.com".to_string());
        assert_eq!(service.server(), "time.cloudflare.com");
        assert!(!service.is_synced());
    }

    #[test]
    fn ntp_service_generation_tracking() {
        let service = NtpService::new("pool.ntp.org".to_string());
        assert_eq!(service.generation.load(Ordering::SeqCst), 0);

        service.set_server("time.cloudflare.com".to_string());
        assert_eq!(service.generation.load(Ordering::SeqCst), 1);

        // Setting same server should not advance generation
        service.set_server("time.cloudflare.com".to_string());
        assert_eq!(service.generation.load(Ordering::SeqCst), 1);

        service.clear_cache();
        assert_eq!(service.generation.load(Ordering::SeqCst), 2);
    }
}
