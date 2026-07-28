//! Error handler for VK API errors

use std::collections::HashMap;

use async_trait::async_trait;

use crate::exception::{VkError, VkResult};

/// Error handler trait (object-safe)
#[async_trait]
pub trait ErrorHandler: Send + Sync {
    async fn handle(&self, error: &VkError) -> VkResult<()>;
}

/// Error handler for specific VK error codes
#[async_trait]
pub trait VkErrorHandler: Send + Sync {
    async fn handle(&self, error: &VkError) -> VkResult<()>;
}

/// Error handler for captcha errors
#[async_trait]
pub trait VkCaptchaHandler: Send + Sync {
    async fn handle(&self, captcha_sid: &str, captcha_img: &str) -> VkResult<()>;
}

/// Default error handler
pub struct DefaultErrorHandler {
    error_handlers: HashMap<i32, Box<dyn VkErrorHandler>>,
    captcha_handler: Option<Box<dyn VkCaptchaHandler>>,
    ignore_errors: bool,
}

impl DefaultErrorHandler {
    pub fn new() -> Self {
        Self {
            error_handlers: HashMap::new(),
            captcha_handler: None,
            ignore_errors: false,
        }
    }

    pub fn with_ignore_errors(mut self, ignore: bool) -> Self {
        self.ignore_errors = ignore;
        self
    }

    pub fn register(&mut self, code: i32, handler: Box<dyn VkErrorHandler>) -> &mut Self {
        self.error_handlers.insert(code, handler);
        self
    }

    pub fn register_captcha(&mut self, handler: Box<dyn VkCaptchaHandler>) -> &mut Self {
        self.captcha_handler = Some(handler);
        self
    }
}

impl Default for DefaultErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorHandler for DefaultErrorHandler {
    async fn handle(&self, error: &VkError) -> VkResult<()> {
        match error {
            VkError::Captcha { sid, img } => {
                if let Some(handler) = &self.captcha_handler {
                    handler.handle(sid, img).await?;
                } else if !self.ignore_errors {
                    return Err(VkError::Captcha {
                        sid: sid.clone(),
                        img: img.clone(),
                    });
                }
            }
            VkError::Api { code, message } => {
                if let Some(handler) = self.error_handlers.get(code) {
                    handler.handle(error).await?;
                } else if !self.ignore_errors {
                    return Err(VkError::Api {
                        code: *code,
                        message: message.clone(),
                    });
                }
            }
            _ if !self.ignore_errors => {
                return Err(VkError::Internal(error.to_string()));
            }
            _ => {}
        }

        Ok(())
    }
}

/// Simple captcha handler that logs the captcha URL
pub struct SimpleCaptchaHandler;

#[async_trait]
impl VkCaptchaHandler for SimpleCaptchaHandler {
    async fn handle(&self, captcha_sid: &str, captcha_img: &str) -> VkResult<()> {
        tracing::warn!("Captcha required: sid={}, img={}", captcha_sid, captcha_img);
        tracing::info!(
            "Captcha URL: https://api.vk.com/captcha.php?sid={}",
            captcha_sid
        );
        Ok(())
    }
}

/// Logging error handler
pub struct LoggingErrorHandler {
    level: tracing::Level,
}

impl LoggingErrorHandler {
    pub fn new() -> Self {
        Self {
            level: tracing::Level::ERROR,
        }
    }

    pub fn with_level(mut self, level: tracing::Level) -> Self {
        self.level = level;
        self
    }
}

impl Default for LoggingErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorHandler for LoggingErrorHandler {
    async fn handle(&self, error: &VkError) -> VkResult<()> {
        match self.level {
            tracing::Level::ERROR => tracing::error!("VK API error: {}", error),
            tracing::Level::WARN => tracing::warn!("VK API error: {}", error),
            tracing::Level::INFO => tracing::info!("VK API error: {}", error),
            tracing::Level::DEBUG => tracing::debug!("VK API error: {}", error),
            tracing::Level::TRACE => tracing::trace!("VK API error: {}", error),
        }
        Ok(())
    }
}
