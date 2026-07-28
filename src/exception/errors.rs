//! VK API error types

use thiserror::Error;

/// Common VK API error types
#[derive(Debug, Error)]
pub enum VkError {
    #[error("VK API error {code}: {message}")]
    Api { 
        code: i32, 
        message: String 
    },
    
    #[error("Captcha required: sid={sid}")]
    Captcha { 
        sid: String, 
        img: String 
    },
    
    #[error("Auth error")]
    Auth,
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Rate limited")]
    RateLimit,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Timeout error")]
    Timeout,
    
    #[error("Connection error")]
    Connection,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Captcha error details
#[derive(Debug, Clone)]
pub struct CaptchaError {
    pub sid: String,
    pub img: String,
    pub ts: Option<i64>,
}

impl CaptchaError {
    pub fn new(sid: String, img: String) -> Self {
        Self {
            sid,
            img,
            ts: None,
        }
    }
    
    pub fn with_ts(mut self, ts: i64) -> Self {
        self.ts = Some(ts);
        self
    }
}

/// Result type for VK API operations
pub type VkResult<T> = Result<T, VkError>;

/// Convert VK API error response to VkError
impl VkError {
    pub fn from_api_response(code: i32, message: String) -> Self {
        Self::Api { code, message }
    }
    
    pub fn from_captcha(sid: String, img: String) -> Self {
        Self::Captcha { sid, img }
    }
}

impl From<CaptchaError> for VkError {
    fn from(captcha: CaptchaError) -> Self {
        Self::Captcha { 
            sid: captcha.sid, 
            img: captcha.img 
        }
    }
}