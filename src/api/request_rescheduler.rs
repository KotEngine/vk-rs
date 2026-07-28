//! Request rescheduling for VK API

use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;
use crate::exception::{VkResult, VkError};

/// Request rescheduler trait
#[async_trait]
pub trait RequestRescheduler: Send + Sync {
    /// Handle failed request and return whether to retry
    async fn handle_failure(&self, error: &VkError, attempt: u32) -> VkResult<bool>;
    
    /// Get maximum retry attempts
    fn max_attempts(&self) -> u32;
    
    /// Get base delay between retries
    fn base_delay(&self) -> Duration;
    
    /// Check if error is retryable
    fn is_retryable(&self, error: &VkError) -> bool;
}

/// Blocking request rescheduler
/// Implements exponential backoff with jitter for VK API errors
pub struct BlockingRequestRescheduler {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    jitter_factor: f64,
    retryable_codes: Vec<i32>,
}

impl BlockingRequestRescheduler {
    /// Create a new blocking request rescheduler
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.1,
            retryable_codes: vec![6, 29], // Common VK retryable error codes
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
        jitter_factor: f64,
    ) -> Self {
        Self {
            max_attempts,
            base_delay,
            max_delay,
            jitter_factor,
            retryable_codes: vec![6, 29],
        }
    }
    
    /// Add retryable error code
    pub fn add_retryable_code(&mut self, code: i32) {
        self.retryable_codes.push(code);
    }
    
    /// Calculate delay with exponential backoff and jitter
    fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        
        // Exponential backoff: base_delay * 2^(attempt-1)
        let mut delay = self.base_delay * 2_u32.pow(attempt.saturating_sub(1));
        
        // Cap at max_delay
        if delay > self.max_delay {
            delay = self.max_delay;
        }
        
        // Add jitter (±10% by default)
        if self.jitter_factor > 0.0 {
            let jitter_range = (delay.as_millis() as f64 * self.jitter_factor) as u64;
            let jitter = (rand::random::<i64>() % (jitter_range * 2) as i64) - jitter_range as i64;
            delay = Duration::from_millis(delay.as_millis() as u64 + jitter as u64);
        }
        
        delay
    }
    
    /// Check if error code is retryable
    fn is_retryable_code(&self, code: i32) -> bool {
        self.retryable_codes.contains(&code) || code == 6 || code == 29
    }
    
    /// Handle VK specific errors
    async fn handle_vk_error(&self, error: &VkError, attempt: u32) -> VkResult<bool> {
        match error {
            VkError::Api { code, message } => {
                if self.is_retryable_code(*code) {
                    let delay = self.calculate_delay(attempt);
                    tracing::warn!("VK API error {} (attempt {}), retrying in {:?}: {}", code, attempt, delay, message);
                    
                    sleep(delay).await;
                    Ok(true)
                } else {
                    tracing::error!("Non-retryable VK API error {}: {}", code, message);
                    Ok(false)
                }
            },
            VkError::RateLimit => {
                let delay = self.calculate_delay(attempt);
                tracing::warn!("Rate limit exceeded (attempt {}), retrying in {:?}...", attempt, delay);
                
                sleep(delay).await;
                Ok(true)
            },
            VkError::Captcha { sid, img } => {
                tracing::warn!("Captcha required: sid={}, img={}", sid, img);
                // Don't retry captcha errors automatically
                Ok(false)
            },
            _ => {
                tracing::error!("Non-retryable error: {}", error);
                Ok(false)
            }
        }
    }
}

#[async_trait]
impl RequestRescheduler for BlockingRequestRescheduler {
    async fn handle_failure(&self, error: &VkError, attempt: u32) -> VkResult<bool> {
        if attempt >= self.max_attempts {
            tracing::error!("Max retry attempts ({}) reached", self.max_attempts);
            return Ok(false);
        }

        self.handle_vk_error(error, attempt).await
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    fn base_delay(&self) -> Duration {
        self.base_delay
    }

    fn is_retryable(&self, error: &VkError) -> bool {
        match error {
            VkError::Api { code, message } => {
                self.is_retryable_code(*code)
                    || message.to_lowercase().contains("rate limit")
                    || message.to_lowercase().contains("too many requests")
            }
            VkError::RateLimit => true,
            VkError::Http(_) => true,
            VkError::Io(_) => true,
            _ => false,
        }
    }
}

impl Default for BlockingRequestRescheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Immediate request rescheduler
/// Never retries requests, fails immediately
pub struct ImmediateRequestRescheduler;

impl ImmediateRequestRescheduler {
    /// Create a new immediate rescheduler
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImmediateRequestRescheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RequestRescheduler for ImmediateRequestRescheduler {
    async fn handle_failure(&self, _error: &VkError, _attempt: u32) -> VkResult<bool> {
        Ok(false)
    }
    
    fn max_attempts(&self) -> u32 {
        1
    }
    
    fn base_delay(&self) -> Duration {
        Duration::from_millis(0)
    }
    
    fn is_retryable(&self, _error: &VkError) -> bool {
        false
    }
}

/// Adaptive request rescheduler
/// Adjusts retry strategy based on error type and recent failures
pub struct AdaptiveRequestRescheduler {
    base_scheduler: BlockingRequestRescheduler,
    recent_failures: std::sync::Arc<std::sync::atomic::AtomicU32>,
    failure_threshold: u32,
    adaptive_delay: Duration,
}

impl AdaptiveRequestRescheduler {
    /// Create a new adaptive rescheduler
    pub fn new() -> Self {
        Self {
            base_scheduler: BlockingRequestRescheduler::new(),
            recent_failures: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            failure_threshold: 5,
            adaptive_delay: Duration::from_secs(5),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(
        base_scheduler: BlockingRequestRescheduler,
        failure_threshold: u32,
        adaptive_delay: Duration,
    ) -> Self {
        Self {
            base_scheduler,
            recent_failures: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            failure_threshold,
            adaptive_delay,
        }
    }
    
    /// Increment failure counter
    fn increment_failure(&self) {
        self.recent_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Reset failure counter
    fn reset_failures(&self) {
        self.recent_failures.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Get current failure count
    fn get_failure_count(&self) -> u32 {
        self.recent_failures.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    /// Check if we should use adaptive delay
    fn should_use_adaptive_delay(&self) -> bool {
        self.get_failure_count() >= self.failure_threshold
    }
}

impl Default for AdaptiveRequestRescheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RequestRescheduler for AdaptiveRequestRescheduler {
    async fn handle_failure(&self, error: &VkError, attempt: u32) -> VkResult<bool> {
        self.increment_failure();
        
        if self.should_use_adaptive_delay() {
            tracing::warn!("High failure rate detected, using adaptive delay");
            sleep(self.adaptive_delay).await;
            return Ok(true);
        }
        
        let result = self.base_scheduler.handle_failure(error, attempt).await;
        
        if matches!(result, Ok(true)) {
            // Reset failure counter on successful retry
            self.reset_failures();
        }
        
        result
    }
    
    fn max_attempts(&self) -> u32 {
        self.base_scheduler.max_attempts()
    }
    
    fn base_delay(&self) -> Duration {
        self.base_scheduler.base_delay()
    }
    
    fn is_retryable(&self, error: &VkError) -> bool {
        self.base_scheduler.is_retryable(error)
    }
}

/// Create a blocking rescheduler with default settings
pub fn create_blocking_rescheduler() -> Box<dyn RequestRescheduler> {
    Box::new(BlockingRequestRescheduler::new())
}

/// Create an immediate rescheduler
pub fn create_immediate_rescheduler() -> Box<dyn RequestRescheduler> {
    Box::new(ImmediateRequestRescheduler::new())
}

/// Create an adaptive rescheduler
pub fn create_adaptive_rescheduler() -> Box<dyn RequestRescheduler> {
    Box::new(AdaptiveRequestRescheduler::new())
}

/// Configuration for rescheduler
pub struct ReschedulerConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter_factor: f64,
    pub failure_threshold: u32,
    pub adaptive_delay: Duration,
}

impl Default for ReschedulerConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.1,
            failure_threshold: 5,
            adaptive_delay: Duration::from_secs(5),
        }
    }
}

impl ReschedulerConfig {
    /// Create a blocking rescheduler from config
    pub fn create_blocking_rescheduler(&self) -> Box<dyn RequestRescheduler> {
        Box::new(BlockingRequestRescheduler::with_config(
            self.max_attempts,
            self.base_delay,
            self.max_delay,
            self.jitter_factor,
        ))
    }
    
    /// Create an adaptive rescheduler from config
    pub fn create_adaptive_rescheduler(&self) -> Box<dyn RequestRescheduler> {
        let base_scheduler = BlockingRequestRescheduler::with_config(
            self.max_attempts,
            self.base_delay,
            self.max_delay,
            self.jitter_factor,
        );
        Box::new(AdaptiveRequestRescheduler::with_config(
            base_scheduler,
            self.failure_threshold,
            self.adaptive_delay,
        ))
    }
}