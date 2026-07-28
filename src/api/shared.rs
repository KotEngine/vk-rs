//! Arc-wrapped API for sharing across async tasks

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::{Api, ApiRequest, VkApi};
use crate::exception::VkResult;

/// Shared API reference for polling and handlers
pub struct SharedApi(pub Arc<Api>);

#[async_trait]
impl VkApi for SharedApi {
    async fn request(&self, method: &str, params: &HashMap<String, String>) -> VkResult<Value> {
        self.0.request(method, params).await
    }

    async fn request_many(&self, requests: &[ApiRequest]) -> VkResult<Vec<Value>> {
        self.0.request_many(requests).await
    }

    fn validate_request(&self, method: &str, params: &HashMap<String, String>) -> VkResult<()> {
        self.0.validate_request(method, params)
    }

    async fn validate_response(&self, response: &Value) -> VkResult<Value> {
        self.0.validate_response(response).await
    }
}

/// Create a boxed shared API from Arc
pub fn shared_api(api: Arc<Api>) -> Box<dyn VkApi> {
    Box::new(SharedApi(api))
}

#[async_trait]
impl VkApi for Arc<Api> {
    async fn request(&self, method: &str, params: &HashMap<String, String>) -> VkResult<Value> {
        self.as_ref().request(method, params).await
    }

    async fn request_many(&self, requests: &[ApiRequest]) -> VkResult<Vec<Value>> {
        self.as_ref().request_many(requests).await
    }

    fn validate_request(&self, method: &str, params: &HashMap<String, String>) -> VkResult<()> {
        self.as_ref().validate_request(method, params)
    }

    async fn validate_response(&self, response: &Value) -> VkResult<Value> {
        self.as_ref().validate_response(response).await
    }
}
