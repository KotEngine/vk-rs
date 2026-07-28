//! HTTP client implementation using reqwest

use async_trait::async_trait;
use std::collections::HashMap;
use crate::exception::{VkError, VkResult};
use super::HttpClient;

/// Reqwest HTTP client implementation
pub struct ReqwestClient {
    client: reqwest::Client,
    #[allow(dead_code)]
    base_url: String,
    timeout: std::time::Duration,
}

impl ReqwestClient {
    /// Create a new ReqwestClient
    pub fn new() -> VkResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("vkontakte/0.1.0")
            .build()
            .map_err(VkError::from)?;
            
        Ok(Self {
            client,
            base_url: crate::constants::VK_API_URL.to_string(),
            timeout: std::time::Duration::from_secs(30),
        })
    }
    
    /// Create a new ReqwestClient with custom base URL
    pub fn with_base_url(base_url: &str) -> VkResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("vkontakte/0.1.0")
            .build()
            .map_err(VkError::from)?;
            
        Ok(Self {
            client,
            base_url: base_url.to_string(),
            timeout: std::time::Duration::from_secs(30),
        })
    }
    
    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set custom user agent
    pub fn with_user_agent(mut self, user_agent: &str) -> VkResult<Self> {
        self.client = reqwest::Client::builder()
            .timeout(self.timeout)
            .user_agent(user_agent)
            .build()
            .map_err(VkError::from)?;
        Ok(self)
    }
    
    /// Build URL with parameters
    #[allow(dead_code)]
    fn build_url(&self, method: &str) -> String {
        format!("{}{}", self.base_url, method)
    }
    
    /// Prepare query parameters
    fn prepare_params<'a>(&self, params: &'a HashMap<String, String>) -> Vec<(&'a str, &'a str)> {
        params.iter()
            .filter_map(|(k, v)| Some((k.as_str(), v.as_str())))
            .collect()
    }
}

#[async_trait]
impl HttpClient for ReqwestClient {
    async fn request_text(
        &self, 
        url: &str, 
        method: &str, 
        params: &HashMap<String, String>, 
        data: Option<&HashMap<String, String>>
    ) -> VkResult<String> {
        let client_method = method.to_lowercase();
        let prepared_params = self.prepare_params(params);
        
        let response = match client_method.as_str() {
            "get" => {
                self.client
                    .get(url)
                    .query(&prepared_params)
                    .timeout(self.timeout)
                    .send()
                    .await
            },
            "post" => {
                if let Some(data) = data {
                    self.client
                        .post(url)
                        .query(&prepared_params)
                        .form(data)
                        .timeout(self.timeout)
                        .send()
                        .await
                } else {
                    self.client
                        .post(url)
                        .query(&prepared_params)
                        .timeout(self.timeout)
                        .send()
                        .await
                }
            },
            _ => {
                return Err(VkError::Validation(format!("Unsupported HTTP method: {}", method)));
            }
        };
        
        let response = response.map_err(VkError::from)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(VkError::Validation(format!("HTTP error {}: {}", status, error_text)));
        }
        
        let text = response.text().await.map_err(VkError::from)?;
        Ok(text)
    }
    
    async fn request_json(
        &self, 
        url: &str, 
        method: &str, 
        params: &HashMap<String, String>, 
        data: Option<&HashMap<String, String>>
    ) -> VkResult<serde_json::Value> {
        let client_method = method.to_lowercase();
        let prepared_params = self.prepare_params(params);
        
        let response = match client_method.as_str() {
            "get" => {
                self.client
                    .get(url)
                    .query(&prepared_params)
                    .timeout(self.timeout)
                    .send()
                    .await
            },
            "post" => {
                if let Some(data) = data {
                    self.client
                        .post(url)
                        .query(&prepared_params)
                        .form(data)
                        .timeout(self.timeout)
                        .send()
                        .await
                } else {
                    self.client
                        .post(url)
                        .query(&prepared_params)
                        .timeout(self.timeout)
                        .send()
                        .await
                }
            },
            _ => {
                return Err(VkError::Validation(format!("Unsupported HTTP method: {}", method)));
            }
        };
        
        let response = response.map_err(VkError::from)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(VkError::Validation(format!("HTTP error {}: {}", status, error_text)));
        }
        
        let json = response.json().await.map_err(VkError::from)?;
        Ok(json)
    }
    
    async fn request_multipart(
        &self, 
        url: &str, 
        form: reqwest::multipart::Form
    ) -> VkResult<serde_json::Value> {
        let response = self.client
            .post(url)
            .multipart(form)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(VkError::from)?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(VkError::Validation(format!("HTTP error {}: {}", status, error_text)));
        }
        
        let json = response.json().await.map_err(VkError::from)?;
        Ok(json)
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new().expect("Failed to create ReqwestClient")
    }
}

/// Create a new HTTP client
pub fn create_http_client() -> VkResult<Box<dyn HttpClient>> {
    Ok(Box::new(ReqwestClient::new()?))
}

/// Create a new HTTP client with custom configuration
pub fn create_http_client_with_config(
    base_url: Option<&str>,
    timeout: Option<std::time::Duration>,
    user_agent: Option<&str>,
) -> VkResult<Box<dyn HttpClient>> {
    let mut client = if let Some(url) = base_url {
        ReqwestClient::with_base_url(url)?
    } else {
        ReqwestClient::new()?
    };

    if let Some(timeout_duration) = timeout {
        client = client.with_timeout(timeout_duration);
    }

    if let Some(ua) = user_agent {
        client = client.with_user_agent(ua)?;
    }

    Ok(Box::new(client))
}