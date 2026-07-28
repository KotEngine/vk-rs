//! Per-method-prefix rate limiting for VK API

use std::collections::HashMap;
use std::sync::Arc;

use super::RateLimiter;
use crate::exception::VkResult;
use crate::tools::rate_limiter::VkRateLimiter;

/// Rate limiter that applies different buckets per method prefix (e.g. `messages.`)
pub struct MethodPrefixRateLimiter {
    default: Arc<dyn RateLimiter>,
    prefixes: HashMap<String, Arc<dyn RateLimiter>>,
}

impl MethodPrefixRateLimiter {
    pub fn new(default_rps: f64) -> Self {
        Self {
            default: Arc::new(VkRateLimiter::new(default_rps)),
            prefixes: HashMap::new(),
        }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>, rps: f64) -> Self {
        self.prefixes
            .insert(prefix.into(), Arc::new(VkRateLimiter::new(rps)));
        self
    }

    pub fn for_vk_defaults() -> Self {
        Self::new(3.0).with_prefix("messages.", 20.0)
    }

    fn limiter_for(&self, method: &str) -> Arc<dyn RateLimiter> {
        for (prefix, limiter) in &self.prefixes {
            if method.starts_with(prefix) {
                return limiter.clone();
            }
        }
        self.default.clone()
    }

    pub async fn check_for_method(&self, method: &str) -> VkResult<()> {
        self.limiter_for(method).check_rate_limit().await
    }
}
