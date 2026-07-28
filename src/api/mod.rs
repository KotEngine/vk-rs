//! VK API client module

pub mod api;
pub mod token;
pub mod request_validator;
pub mod response_validator;
pub mod request_rescheduler;
pub mod shared;
pub mod execute;
pub mod methods;
pub mod methods_extra;
pub mod vkscript;
pub mod method_rate_limiter;

pub use api::*;
pub use method_rate_limiter::*;
pub use execute::*;
pub use vkscript::*;
pub use token::*;
pub use request_validator::*;
pub use response_validator::*;
pub use request_rescheduler::*;
pub use shared::*;

use async_trait::async_trait;

/// Trait for VK API implementations
#[async_trait]
pub trait VkApi: Send + Sync {
    /// Make a single API request
    async fn request(&self, method: &str, params: &std::collections::HashMap<String, String>) -> crate::exception::VkResult<serde_json::Value>;
    
    /// Make multiple API requests
    async fn request_many(&self, requests: &[ApiRequest]) -> crate::exception::VkResult<Vec<serde_json::Value>>;
    
    /// Validate request parameters
    fn validate_request(&self, method: &str, params: &std::collections::HashMap<String, String>) -> crate::exception::VkResult<()>;
    
    /// Validate API response
    async fn validate_response(&self, response: &serde_json::Value) -> crate::exception::VkResult<serde_json::Value>;
}

/// API request structure
#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub method: String,
    pub params: std::collections::HashMap<String, String>,
    pub data: Option<std::collections::HashMap<String, String>>,
}

impl ApiRequest {
    pub fn new(method: &str) -> Self {
        Self {
            method: method.to_string(),
            params: std::collections::HashMap::new(),
            data: None,
        }
    }
    
    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn with_data(mut self, data: std::collections::HashMap<String, String>) -> Self {
        self.data = Some(data);
        self
    }
}

impl Default for ApiRequest {
    fn default() -> Self {
        Self::new("")
    }
}