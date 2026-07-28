use std::collections::HashMap;

use vkontakte::api::response_validator::{JsonResponseValidator, VkApiErrorValidator};
use vkontakte::api::request_validator::TranslateFriendlyTypesValidator;
use vkontakte::api::{RateLimiter, RequestValidator, ResponseValidator, TokenBucketRateLimiter};
use vkontakte::exception::VkError;
use serde_json::json;

#[test]
fn request_validator_converts_list() {
    let validator = TranslateFriendlyTypesValidator::new();
    let mut params = HashMap::new();
    params.insert("user_ids".to_string(), "[\"1\",\"2\",\"3\"]".to_string());

    let out = validator.transform("users.get", params);
    assert_eq!(out.get("user_ids"), Some(&"1,2,3".to_string()));
}

#[test]
fn response_validator_extracts_response_field() {
    let validator = JsonResponseValidator::new();
    let raw = json!({"response": [{"id": 1}]});
    let out = validator.validate(&raw).unwrap();
    assert_eq!(out[0]["id"], 1);
}

#[test]
fn response_validator_maps_api_error() {
    let validator = VkApiErrorValidator::new();
    let raw = json!({"error": {"error_code": 5, "error_msg": "Auth failed"}});
    let err = validator.validate(&raw).unwrap_err();
    assert!(matches!(err, VkError::Api { code: 5, .. }));
}

#[tokio::test]
async fn rate_limiter_blocks_when_empty() {
    let limiter = TokenBucketRateLimiter::new(1, std::time::Duration::from_secs(60));
    limiter.check_rate_limit().await.unwrap();
    let err = limiter.check_rate_limit().await.unwrap_err();
    assert!(matches!(err, VkError::RateLimit));
}
