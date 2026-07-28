//! Token bucket rate limiter

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::api::RateLimiter;
use crate::exception::VkResult;

/// Configurable rate limiter for VK API
pub struct VkRateLimiter {
    inner: Arc<crate::api::TokenBucketRateLimiter>,
}

impl VkRateLimiter {
    pub fn new(requests_per_second: f64) -> Self {
        let refill = Duration::from_secs_f64(1.0 / requests_per_second.max(0.1));
        Self {
            inner: Arc::new(crate::api::TokenBucketRateLimiter::new(
                requests_per_second.ceil() as u64,
                refill,
            )),
        }
    }
}

#[async_trait]
impl RateLimiter for VkRateLimiter {
    async fn check_rate_limit(&self) -> VkResult<()> {
        self.inner.check_rate_limit().await
    }

    async fn record_request(&self) -> VkResult<()> {
        self.inner.record_request().await
    }
}
