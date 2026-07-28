//! Response validation for VK API

use crate::exception::{VkResult, VkError};
use serde_json::Value;

/// Response validator trait
pub trait ResponseValidator: Send + Sync {
    /// Validate response
    fn validate(&self, response: &Value) -> VkResult<Value>;
    
    /// Check if response has error
    fn has_error(&self, response: &Value) -> bool;
    
    /// Extract error from response
    fn extract_error(&self, response: &Value) -> Option<VkError>;
}

/// JSON response validator
/// Checks for 'response' field and validates its structure
pub struct JsonResponseValidator {
    require_response: bool,
}

impl JsonResponseValidator {
    /// Create a new JSON response validator
    pub fn new() -> Self {
        Self {
            require_response: true,
        }
    }
    
    /// Create with optional response requirement
    pub fn with_require_response(require_response: bool) -> Self {
        Self {
            require_response,
        }
    }
    
    /// Extract response field
    fn extract_response_field<'a>(&self, response: &'a Value) -> Option<&'a Value> {
        response.get("response")
    }
    
    /// Validate response structure
    fn validate_response_structure(&self, response: &Value) -> VkResult<()> {
        if !response.is_object() {
            return Err(VkError::Validation(
                "Response is not a JSON object".to_string(),
            ));
        }
        
        // Check for error field
        if response.get("error").is_some() {
            return Err(VkError::Validation("Response contains error field".to_string()));
        }
        
        // Check for response field if required
        if self.require_response && !response.get("response").is_some() {
            return Err(VkError::Validation("Response missing required 'response' field".to_string()));
        }
        
        Ok(())
    }
}

impl ResponseValidator for JsonResponseValidator {
    fn validate(&self, response: &Value) -> VkResult<Value> {
        self.validate_response_structure(response)?;
        
        if let Some(response_field) = self.extract_response_field(response) {
            Ok(response_field.clone())
        } else {
            Ok(response.clone())
        }
    }
    
    fn has_error(&self, response: &Value) -> bool {
        response.get("error").is_some()
    }
    
    fn extract_error(&self, response: &Value) -> Option<VkError> {
        if let Some(error_obj) = response.get("error") {
            if let Some(code) = error_obj.get("error_code").and_then(|c| c.as_i64()) {
                if let Some(message) = error_obj.get("error_msg").and_then(|m| m.as_str()) {
                    return Some(VkError::Api {
                        code: code as i32,
                        message: message.to_string(),
                    });
                }
            }
        }
        None
    }
}

/// VK API error validator
/// Checks for VK API specific error codes and messages
pub struct VkApiErrorValidator {
    ignore_codes: Vec<i32>,
}

impl VkApiErrorValidator {
    /// Create a new VK API error validator
    pub fn new() -> Self {
        Self {
            ignore_codes: Vec::new(),
        }
    }
    
    /// Create with ignored error codes
    pub fn with_ignore_codes(codes: Vec<i32>) -> Self {
        Self {
            ignore_codes: codes,
        }
    }
    
    /// Add ignored error code
    pub fn add_ignore_code(&mut self, code: i32) {
        self.ignore_codes.push(code);
    }
    
    /// Check if error code should be ignored
    fn should_ignore_error(&self, code: i32) -> bool {
        self.ignore_codes.contains(&code)
    }
    
    /// Parse VK API error
    fn parse_vk_error(&self, error_obj: &Value) -> Option<VkError> {
        if let Some(code) = error_obj.get("error_code").and_then(|c| c.as_i64()) {
            if let Some(message) = error_obj.get("error_msg").and_then(|m| m.as_str()) {
                // Handle captcha error specially
                if code == 14 {
                    if let Some(sid) = error_obj.get("captcha_sid").and_then(|s| s.as_str()) {
                        if let Some(img) = error_obj.get("captcha_img").and_then(|i| i.as_str()) {
                            return Some(VkError::Captcha {
                                sid: sid.to_string(),
                                img: img.to_string(),
                            });
                        }
                    }
                }
                
                // Check if error code should be ignored
                if self.should_ignore_error(code as i32) {
                    return None;
                }
                
                return Some(VkError::Api {
                    code: code as i32,
                    message: message.to_string(),
                });
            }
        }
        
        // Parse error from request_failed field
        if let Some(request_failed) = error_obj.get("request_failed") {
            if let Some(code) = request_failed.get("error_code").and_then(|c| c.as_i64()) {
                if let Some(message) = request_failed.get("error_msg").and_then(|m| m.as_str()) {
                    return Some(VkError::Api {
                        code: code as i32,
                        message: message.to_string(),
                    });
                }
            }
        }
        
        None
    }
}

impl ResponseValidator for VkApiErrorValidator {
    fn validate(&self, response: &Value) -> VkResult<Value> {
        if let Some(error_obj) = response.get("error") {
            if let Some(parsed_error) = self.parse_vk_error(error_obj) {
                return Err(parsed_error);
            }
        }
        
        Ok(response.clone())
    }
    
    fn has_error(&self, response: &Value) -> bool {
        response.get("error").is_some() || response.get("request_failed").is_some()
    }
    
    fn extract_error(&self, response: &Value) -> Option<VkError> {
        if let Some(error_obj) = response.get("error") {
            self.parse_vk_error(error_obj)
        } else if let Some(request_failed) = response.get("request_failed") {
            self.parse_vk_error(request_failed)
        } else {
            None
        }
    }
}

/// Rate limit validator
/// Checks for rate limit errors and provides backoff information
pub struct RateLimitValidator {
    retry_after_header: String,
}

impl RateLimitValidator {
    /// Create a new rate limit validator
    pub fn new() -> Self {
        Self {
            retry_after_header: "retry-after".to_string(),
        }
    }
    
    /// Create with custom retry-after header name
    pub fn with_retry_after_header(header: &str) -> Self {
        Self {
            retry_after_header: header.to_string(),
        }
    }
    
    /// Check for rate limit error
    #[allow(dead_code)]
    fn is_rate_limit_error(&self, error: &VkError) -> bool {
        match error {
            VkError::Api { code, message } => {
                // Common VK rate limit error codes
                *code == 6 || *code == 29 || 
                message.to_lowercase().contains("rate limit") ||
                message.to_lowercase().contains("too many requests")
            },
            _ => false,
        }
    }
}

impl ResponseValidator for RateLimitValidator {
    fn validate(&self, response: &Value) -> VkResult<Value> {
        // Check if response has rate limit headers
        if let Some(headers) = response.get("headers") {
            if let Some(retry_after) = headers.get(&self.retry_after_header).and_then(|r| r.as_u64()) {
                tracing::warn!("Rate limit exceeded. Retry after {} seconds", retry_after);
                return Err(VkError::RateLimit);
            }
        }
        
        Ok(response.clone())
    }
    
    fn has_error(&self, _response: &Value) -> bool {
        false // Rate limit errors are handled in the API client, not here
    }
    
    fn extract_error(&self, _response: &Value) -> Option<VkError> {
        None
    }
}

/// Combined response validator
pub struct CombinedResponseValidator {
    validators: Vec<Box<dyn ResponseValidator>>,
}

impl CombinedResponseValidator {
    /// Create a new combined validator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }
    
    /// Add a validator
    pub fn add_validator(&mut self, validator: Box<dyn ResponseValidator>) {
        self.validators.push(validator);
    }
    
    /// Create with default validators
    pub fn with_defaults() -> Self {
        let mut validator = Self::new();
        validator.add_validator(Box::new(JsonResponseValidator::new()));
        validator.add_validator(Box::new(VkApiErrorValidator::new()));
        validator.add_validator(Box::new(RateLimitValidator::new()));
        validator
    }
}

impl ResponseValidator for CombinedResponseValidator {
    fn validate(&self, response: &Value) -> VkResult<Value> {
        for validator in &self.validators {
            let validated = validator.validate(response)?;
            // Use the validated response for next validators
            return validator.validate(&validated);
        }
        Ok(response.clone())
    }
    
    fn has_error(&self, response: &Value) -> bool {
        for validator in &self.validators {
            if validator.has_error(response) {
                return true;
            }
        }
        false
    }
    
    fn extract_error(&self, response: &Value) -> Option<VkError> {
        for validator in &self.validators {
            if let Some(error) = validator.extract_error(response) {
                return Some(error);
            }
        }
        None
    }
}

/// Create a combined response validator with default validators
pub fn create_default_response_validator() -> Box<dyn ResponseValidator> {
    Box::new(CombinedResponseValidator::with_defaults())
}

/// Create a simple JSON validator
pub fn create_json_validator() -> Box<dyn ResponseValidator> {
    Box::new(JsonResponseValidator::new())
}

/// Create VK API error validator with ignored codes
pub fn create_vk_error_validator(ignored_codes: Vec<i32>) -> Box<dyn ResponseValidator> {
    let validator = VkApiErrorValidator::with_ignore_codes(ignored_codes);
    Box::new(validator)
}