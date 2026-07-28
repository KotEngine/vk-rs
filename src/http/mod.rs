//! HTTP client abstraction for VK API

pub mod reqwest_client;

pub use reqwest_client::*;

use async_trait::async_trait;
use std::collections::HashMap;
use crate::exception::VkResult;

/// HTTP client trait for making HTTP requests
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// Make a text request
    async fn request_text(
        &self, 
        url: &str, 
        method: &str, 
        params: &HashMap<String, String>, 
        data: Option<&HashMap<String, String>>
    ) -> VkResult<String>;
    
    /// Make a JSON request
    async fn request_json(
        &self, 
        url: &str, 
        method: &str, 
        params: &HashMap<String, String>, 
        data: Option<&HashMap<String, String>>
    ) -> VkResult<serde_json::Value>;
    
    /// Make a multipart request
    async fn request_multipart(
        &self, 
        url: &str, 
        form: reqwest::multipart::Form
    ) -> VkResult<serde_json::Value>;
}