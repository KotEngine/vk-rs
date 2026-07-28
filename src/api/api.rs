//! Main VK API client implementation

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

use std::time::Duration;

use crate::api::*;
use crate::api::request_validator::create_default_request_validator as create_request_validator;
use crate::api::response_validator::create_default_response_validator as create_response_validator;
use crate::exception::*;
use crate::http::*;

/// Main VK API client
pub struct Api {
    token_generator: Box<dyn TokenGenerator>,
    http_client: Box<dyn HttpClient>,
    request_rescheduler: Box<dyn RequestRescheduler>,
    response_validators: Vec<Box<dyn ResponseValidator>>,
    request_validators: Vec<Box<dyn RequestValidator>>,
    /// Returns captcha key to retry the request, or `None` to abort
    captcha_handler: Option<Box<dyn Fn(CaptchaError) -> Option<String> + Send + Sync>>,
    ignore_errors: bool,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    method_rate_limiter: Option<Arc<super::method_rate_limiter::MethodPrefixRateLimiter>>,
}

impl Api {
    /// Create a new API client
    pub fn new(token: &str) -> VkResult<Self> {
        Self::with_token_generator(single_token(token))
    }
    
    /// Create with token generator
    pub fn with_token_generator(token_generator: Box<dyn TokenGenerator>) -> VkResult<Self> {
        let http_client: Box<dyn HttpClient> = create_http_client()?;
        let request_rescheduler = create_blocking_rescheduler();
        let response_validators = vec![create_response_validator()];
        let request_validators = vec![create_request_validator()];
        
        Ok(Self {
            token_generator,
            http_client,
            request_rescheduler,
            response_validators,
            request_validators,
            captcha_handler: None,
            ignore_errors: false,
            rate_limiter: None,
            method_rate_limiter: None,
        })
    }

    /// VK default limits: 3 rps global, 20 rps for `messages.*`
    pub fn with_vk_rate_limits(mut self) -> Self {
        self.method_rate_limiter =
            Some(Arc::new(super::method_rate_limiter::MethodPrefixRateLimiter::for_vk_defaults()));
        self
    }

    pub fn with_method_rate_limiter(
        mut self,
        limiter: Arc<super::method_rate_limiter::MethodPrefixRateLimiter>,
    ) -> Self {
        self.method_rate_limiter = Some(limiter);
        self
    }

    /// Attach a simple global token-bucket limiter (`requests_per_second`)
    pub fn with_requests_per_second(self, rps: f64) -> Self {
        self.with_rate_limiter(Arc::new(TokenBucketRateLimiter::new(
            rps.ceil() as u64,
            Duration::from_secs_f64(1.0 / rps.max(0.1)),
        )))
    }
    
    /// Create with HTTP client
    pub fn with_http_client(mut self, http_client: Box<dyn HttpClient>) -> Self {
        self.http_client = http_client;
        self
    }
    
    /// Create with request rescheduler
    pub fn with_request_rescheduler(mut self, rescheduler: Box<dyn RequestRescheduler>) -> Self {
        self.request_rescheduler = rescheduler;
        self
    }
    
    /// Create with response validators
    pub fn with_response_validators(mut self, validators: Vec<Box<dyn ResponseValidator>>) -> Self {
        self.response_validators = validators;
        self
    }
    
    /// Create with request validators
    pub fn with_request_validators(mut self, validators: Vec<Box<dyn RequestValidator>>) -> Self {
        self.request_validators = validators;
        self
    }
    
    /// Set captcha handler — return solved captcha key for retry
    pub fn with_captcha_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(CaptchaError) -> Option<String> + Send + Sync + 'static,
    {
        self.captcha_handler = Some(Box::new(handler));
        self
    }
    
    /// Set ignore errors flag
    pub fn with_ignore_errors(mut self, ignore: bool) -> Self {
        self.ignore_errors = ignore;
        self
    }
    
    /// Set rate limiter
    pub fn with_rate_limiter(mut self, limiter: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }
    
    /// Add response validator
    pub fn add_response_validator(&mut self, validator: Box<dyn ResponseValidator>) {
        self.response_validators.push(validator);
    }
    
    /// Add request validator
    pub fn add_request_validator(&mut self, validator: Box<dyn RequestValidator>) {
        self.request_validators.push(validator);
    }
    
    fn resolve_captcha_key(&self, captcha: CaptchaError) -> VkResult<Option<String>> {
        if let Some(handler) = &self.captcha_handler {
            Ok(handler(captcha))
        } else if self.ignore_errors {
            Ok(None)
        } else {
            Err(VkError::from(captcha))
        }
    }

    /// Apply rate limiting for a specific API method
    async fn apply_rate_limit(&self, method: &str) -> VkResult<()> {
        if let Some(limiter) = &self.method_rate_limiter {
            limiter.check_for_method(method).await?;
        } else if let Some(limiter) = &self.rate_limiter {
            limiter.check_rate_limit().await?;
        }
        Ok(())
    }
    
    /// Validate request parameters
    fn run_request_validators(&self, method: &str, params: &HashMap<String, String>) -> VkResult<()> {
        for validator in &self.request_validators {
            validator.validate(method, params)?;
        }
        Ok(())
    }
    
    /// Transform request parameters
    fn transform_request(&self, method: &str, params: HashMap<String, String>) -> HashMap<String, String> {
        let mut transformed = params;
        for validator in &self.request_validators {
            transformed = validator.transform(method, transformed);
        }
        transformed
    }
    
    /// Validate response
    async fn run_response_validators(&self, response: &serde_json::Value) -> VkResult<serde_json::Value> {
        let mut validated = response.clone();
        for validator in &self.response_validators {
            validated = validator.validate(&validated)?;
        }
        Ok(validated)
    }
    
    /// Make HTTP request with retry logic
    #[tracing::instrument(name = "api_http", skip_all, fields(method = %method))]
    async fn make_http_request(
        &self,
        method: &str,
        params: &HashMap<String, String>,
        data: Option<&HashMap<String, String>>,
    ) -> VkResult<serde_json::Value> {
        let mut attempt = 0u32;

        loop {
            self.apply_rate_limit(method).await?;
            tracing::debug!(attempt, multipart = data.is_some(), "sending api request");

            let response = if let Some(data) = data {
                self.http_client
                    .request_multipart(
                        &format!("{}{}", crate::constants::VK_API_URL, method),
                        self.create_multipart_form(data)?,
                    )
                    .await
            } else {
                self.http_client
                    .request_json(
                        &format!("{}{}", crate::constants::VK_API_URL, method),
                        "POST",
                        params,
                        data,
                    )
                    .await
            };

            match response {
                Ok(json_response) => return self.run_response_validators(&json_response).await,
                Err(error) => {
                    if self.request_rescheduler.is_retryable(&error)
                        && attempt < self.request_rescheduler.max_attempts()
                    {
                        let should_retry = self
                            .request_rescheduler
                            .handle_failure(&error, attempt)
                            .await?;
                        if should_retry {
                            attempt += 1;
                            tracing::warn!(attempt, error = %error, "api request failed, retrying");
                            continue;
                        }
                    }
                    tracing::error!(attempt, error = %error, "api request failed");
                    return Err(error);
                }
            }
        }
    }
    
    /// Create multipart form from data
    fn create_multipart_form(&self, data: &HashMap<String, String>) -> VkResult<reqwest::multipart::Form> {
        let mut form = reqwest::multipart::Form::new();
        
        for (key, value) in data {
            form = form.text(key.clone(), value.clone());
        }
        
        Ok(form)
    }
}

#[async_trait]
impl VkApi for Api {
    #[tracing::instrument(name = "api_call", skip_all, fields(method = %method))]
    async fn request(&self, method: &str, params: &HashMap<String, String>) -> VkResult<serde_json::Value> {
        self.run_request_validators(method, params)?;

        let transformed_params = self.transform_request(method, params.clone());
        let token = self.token_generator.get_token();
        let mut params_with_token = transformed_params;
        params_with_token.insert("access_token".to_string(), token.to_string());
        params_with_token.insert("v".to_string(), crate::constants::VK_API_VERSION.to_string());

        let mut captcha_attempts = 0u32;
        const MAX_CAPTCHA_ATTEMPTS: u32 = 3;

        loop {
            match self
                .make_http_request(method, &params_with_token, None)
                .await
            {
                Ok(value) => return Ok(value),
                Err(VkError::Captcha { sid, img }) => {
                    if captcha_attempts >= MAX_CAPTCHA_ATTEMPTS {
                        return Err(VkError::Captcha { sid, img });
                    }
                    captcha_attempts += 1;
                    tracing::warn!(attempt = captcha_attempts, %sid, "captcha requested by vk");
                    let key = self.resolve_captcha_key(CaptchaError::new(sid.clone(), img.clone()))?;
                    let Some(key) = key else {
                        return Err(VkError::Captcha { sid, img });
                    };
                    params_with_token.insert("captcha_sid".to_string(), sid);
                    params_with_token.insert("captcha_key".to_string(), key);
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    async fn request_many(&self, requests: &[ApiRequest]) -> VkResult<Vec<serde_json::Value>> {
        let mut results = Vec::new();
        
        for request in requests {
            // Validate request
            self.validate_request(&request.method, &request.params)?;
            
            // Transform parameters
            let transformed_params = self.transform_request(&request.method, request.params.clone());
            
            // Get token
            let token = self.token_generator.get_token();
            let mut params_with_token = transformed_params.clone();
            params_with_token.insert("access_token".to_string(), token.to_string());
            params_with_token.insert("v".to_string(), crate::constants::VK_API_VERSION.to_string());
            
            // Make request
            let result = self.make_http_request(
                &request.method,
                &params_with_token,
                request.data.as_ref(),
            )
            .await?;
            
            results.push(result);
        }
        
        Ok(results)
    }
    
    fn validate_request(&self, method: &str, params: &HashMap<String, String>) -> VkResult<()> {
        self.run_request_validators(method, params)
    }
    
    async fn validate_response(&self, response: &serde_json::Value) -> VkResult<serde_json::Value> {
        self.run_response_validators(response).await
    }
}

/// Rate limiter trait
#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check_rate_limit(&self) -> VkResult<()>;
    async fn record_request(&self) -> VkResult<()>;
}

/// Simple token bucket rate limiter
pub struct TokenBucketRateLimiter {
    tokens: Arc<RwLock<u64>>,
    capacity: u64,
    refill_rate: std::time::Duration,
    last_refill: Arc<RwLock<std::time::Instant>>,
}

impl TokenBucketRateLimiter {
    /// Create a new token bucket rate limiter
    pub fn new(capacity: u64, refill_rate: std::time::Duration) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(capacity)),
            capacity,
            refill_rate,
            last_refill: Arc::new(RwLock::new(std::time::Instant::now())),
        }
    }
    
    /// Refill tokens
    async fn refill_tokens(&self) -> VkResult<()> {
        let mut tokens = self.tokens.write().await;
        let mut last_refill = self.last_refill.write().await;
        
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(*last_refill);
        
        if elapsed >= self.refill_rate {
            let tokens_to_add = (elapsed.as_millis() / self.refill_rate.as_millis()) as u64;
            *tokens = (*tokens + tokens_to_add).min(self.capacity);
            *last_refill = now;
        }
        
        Ok(())
    }
}

#[async_trait]
impl RateLimiter for TokenBucketRateLimiter {
    async fn check_rate_limit(&self) -> VkResult<()> {
        self.refill_tokens().await?;
        
        let mut tokens = self.tokens.write().await;
        if *tokens == 0 {
            return Err(VkError::RateLimit);
        }
        
        *tokens -= 1;
        Ok(())
    }
    
    async fn record_request(&self) -> VkResult<()> {
        self.refill_tokens().await?;
        
        let mut tokens = self.tokens.write().await;
        if *tokens == 0 {
            return Err(VkError::RateLimit);
        }
        
        *tokens -= 1;
        Ok(())
    }
}

/// Create API client with token
pub fn api(token: &str) -> VkResult<Api> {
    Api::new(token)
}

/// Create API client with token generator
pub fn api_with_token_generator(token_generator: Box<dyn TokenGenerator>) -> VkResult<Api> {
    Api::with_token_generator(token_generator)
}

/// Create API client with configuration
pub fn api_with_config(
    token: &str,
    max_retries: u32,
    timeout: std::time::Duration,
) -> VkResult<Api> {
    let mut api = Api::new(token)?;
    
    // Configure request rescheduler
    let rescheduler = BlockingRequestRescheduler::with_config(
        max_retries,
        Duration::from_secs(1),
        Duration::from_secs(60),
        0.1,
    );
    api = api.with_request_rescheduler(Box::new(rescheduler));
    
    // Configure HTTP client
    let http_client = create_http_client_with_config(
        None,
        Some(timeout),
        Some("vkontakte/0.1.0"),
    )?;
    api = api.with_http_client(http_client);
    
    Ok(api)
}