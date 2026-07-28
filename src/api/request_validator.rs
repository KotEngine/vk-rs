//! Request validation for VK API

use crate::exception::{VkResult, VkError};
use std::collections::HashMap;

/// Request validator trait
pub trait RequestValidator: Send + Sync {
    /// Validate request parameters
    fn validate(&self, method: &str, params: &HashMap<String, String>) -> VkResult<()>;
    
    /// Transform request parameters (optional)
    fn transform(&self, _method: &str, params: HashMap<String, String>) -> HashMap<String, String> {
        params
    }
}

/// Translate friendly types validator
/// Converts lists to comma-separated strings and booleans to 0/1
pub struct TranslateFriendlyTypesValidator;

impl TranslateFriendlyTypesValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self
    }
    
    /// Convert list to comma-separated string
    fn convert_list_to_string(list: &[String]) -> String {
        list.join(",")
    }
    
    /// Convert boolean to 0/1 string
    fn convert_bool_to_string(value: bool) -> String {
        if value {
            "1".to_string()
        } else {
            "0".to_string()
        }
    }
    
    /// Validate and transform parameters
    fn validate_and_transform(&self, _method: &str, params: HashMap<String, String>) -> HashMap<String, String> {
        let mut transformed = HashMap::new();
        
        for (key, value) in params {
            let transformed_value = match key.as_str() {
                // Handle list parameters
                "user_ids" | "group_ids" | "peer_ids" | "chat_ids" | "domain" | 
                "fields" | "name_case" | "filter" | "message" | "text" |
                "attachments" | "payload" | "keyboard" | "access_token" => {
                    // Check if value looks like a JSON array
                    if value.starts_with('[') && value.ends_with(']') {
                        // Try to parse as array and convert to comma-separated string
                        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&value) {
                            Self::convert_list_to_string(&parsed)
                        } else {
                            value
                        }
                    } else {
                        value
                    }
                },
                // Handle boolean parameters
                "v" | "group_id" | "peer_id" | "chat_id" | "user_id" |
                "from_group" | "random_id" | "lat" | "long" |
                "title" | "sticker_id" | "sound" | "mute_until" |
                "admin_id" | "reason" | "unmute_date" => {
                    if value.starts_with('t') || value.starts_with('T') || value.starts_with('1') {
                        Self::convert_bool_to_string(true)
                    } else if value.starts_with('f') || value.starts_with('F') || value.starts_with('0') {
                        Self::convert_bool_to_string(false)
                    } else {
                        value
                    }
                },
                _ => value,
            };
            
            transformed.insert(key, transformed_value);
        }
        
        transformed
    }
}

impl RequestValidator for TranslateFriendlyTypesValidator {
    fn validate(&self, method: &str, _params: &HashMap<String, String>) -> VkResult<()> {
        // Basic validation - check for required parameters for common methods
        match method {
            "messages.send" => {
                // messages.send requires peer_id or user_id or chat_id
                if !(_params.contains_key("peer_id") || 
                     _params.contains_key("user_id") || 
                     _params.contains_key("chat_id")) {
                    return Err(VkError::Validation(
                        "messages.send requires peer_id, user_id, or chat_id".to_string()
                    ));
                }
                
                // Check for message text
                if !_params.contains_key("message") && !_params.contains_key("attachment") {
                    return Err(VkError::Validation(
                        "messages.send requires message or attachment".to_string()
                    ));
                }
            },
            "wall.post" => {
                // wall.post requires message or attachment
                if !_params.contains_key("message") && !_params.contains_key("attachment") {
                    return Err(VkError::Validation(
                        "wall.post requires message or attachment".to_string()
                    ));
                }
            },
            "photos.getMessagesUploadServer" => {
                // photos.getMessagesUploadServer requires peer_id
                if !_params.contains_key("peer_id") {
                    return Err(VkError::Validation(
                        "photos.getMessagesUploadServer requires peer_id".to_string()
                    ));
                }
            },
            _ => {
                // For other methods, just check that we have some parameters
                if _params.is_empty() {
                    tracing::warn!("Request to method '{}' has no parameters", method);
                }
            }
        }
        
        Ok(())
    }
    
    fn transform(&self, method: &str, params: HashMap<String, String>) -> HashMap<String, String> {
        self.validate_and_transform(method, params)
    }
}

/// Required fields validator
pub struct RequiredFieldsValidator {
    required_fields: HashMap<String, Vec<String>>,
}

impl RequiredFieldsValidator {
    /// Create a new validator with required fields mapping
    pub fn new() -> Self {
        let mut required_fields = HashMap::new();
        
        // Define required fields for common methods
        required_fields.insert("messages.send".to_string(), vec!["peer_id".to_string()]);
        required_fields.insert("wall.post".to_string(), vec!["message".to_string()]);
        required_fields.insert("photos.getMessagesUploadServer".to_string(), vec!["peer_id".to_string()]);
        required_fields.insert("docs.getMessagesUploadServer".to_string(), vec!["peer_id".to_string()]);
        required_fields.insert("audio.getUploadServer".to_string(), vec!["artist".to_string(), "title".to_string()]);
        
        Self {
            required_fields,
        }
    }
    
    /// Add required fields for a method
    pub fn add_required_fields(&mut self, method: &str, fields: Vec<String>) {
        self.required_fields.insert(method.to_string(), fields);
    }
    
    /// Check if a method has required fields
    fn has_required_fields(&self, method: &str) -> Option<&Vec<String>> {
        self.required_fields.get(method)
    }
}

impl RequestValidator for RequiredFieldsValidator {
    fn validate(&self, method: &str, params: &HashMap<String, String>) -> VkResult<()> {
        if let Some(required) = self.has_required_fields(method) {
            for field in required {
                if !params.contains_key(field) {
                    return Err(VkError::Validation(
                        format!("Method '{}' requires field '{}'", method, field)
                    ));
                }
            }
        }
        
        Ok(())
    }
}

/// Parameter range validator
pub struct ParameterRangeValidator {
    min_values: HashMap<String, i64>,
    max_values: HashMap<String, i64>,
}

impl ParameterRangeValidator {
    /// Create a new parameter range validator
    pub fn new() -> Self {
        let mut min_values = HashMap::new();
        let mut max_values = HashMap::new();
        
        // Common parameter ranges
        min_values.insert("random_id".to_string(), 0);
        max_values.insert("random_id".to_string(), 2_147_483_647); // i32::MAX
        
        min_values.insert("count".to_string(), 0);
        max_values.insert("count".to_string(), 100); // VK API limit
        
        min_values.insert("offset".to_string(), 0);
        max_values.insert("offset".to_string(), 10_000);
        
        Self {
            min_values,
            max_values,
        }
    }
    
    /// Add parameter range
    pub fn add_range(&mut self, param: &str, min: i64, max: i64) {
        self.min_values.insert(param.to_string(), min);
        self.max_values.insert(param.to_string(), max);
    }
    
    /// Get parameter range
    fn get_range(&self, param: &str) -> Option<(i64, i64)> {
        if let (Some(&min), Some(&max)) = (self.min_values.get(param), self.max_values.get(param)) {
            Some((min, max))
        } else {
            None
        }
    }
}

impl RequestValidator for ParameterRangeValidator {
    fn validate(&self, _method: &str, params: &HashMap<String, String>) -> VkResult<()> {
        for (param, value_str) in params {
            if let Ok(value) = value_str.parse::<i64>() {
                if let Some((min, max)) = self.get_range(param) {
                    if value < min || value > max {
                        return Err(VkError::Validation(
                            format!("Parameter '{}' value {} is out of range [{}, {}]", param, value, min, max)
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Combined request validator
pub struct CombinedRequestValidator {
    validators: Vec<Box<dyn RequestValidator>>,
}

impl CombinedRequestValidator {
    /// Create a new combined validator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }
    
    /// Add a validator
    pub fn add_validator(&mut self, validator: Box<dyn RequestValidator>) {
        self.validators.push(validator);
    }
    
    /// Create with default validators
    pub fn with_defaults() -> Self {
        let mut validator = Self::new();
        validator.add_validator(Box::new(TranslateFriendlyTypesValidator::new()));
        validator.add_validator(Box::new(RequiredFieldsValidator::new()));
        validator.add_validator(Box::new(ParameterRangeValidator::new()));
        validator
    }
}

impl RequestValidator for CombinedRequestValidator {
    fn validate(&self, method: &str, params: &HashMap<String, String>) -> VkResult<()> {
        for validator in &self.validators {
            validator.validate(method, params)?;
        }
        Ok(())
    }
    
    fn transform(&self, method: &str, params: HashMap<String, String>) -> HashMap<String, String> {
        let mut transformed = params;
        for validator in &self.validators {
            transformed = validator.transform(method, transformed);
        }
        transformed
    }
}

/// Create a combined request validator with default validators
pub fn create_default_request_validator() -> Box<dyn RequestValidator> {
    Box::new(CombinedRequestValidator::with_defaults())
}

/// Create a simple request validator that only does type translation
pub fn create_simple_request_validator() -> Box<dyn RequestValidator> {
    Box::new(TranslateFriendlyTypesValidator::new())
}