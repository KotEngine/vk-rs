//! Token management for VK API

use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::exception::{VkResult, VkError};

/// Token generator trait
#[async_trait]
pub trait TokenGenerator: Send + Sync {
    /// Get current token
    fn get_token(&self) -> &str;
    
    /// Rotate token (if supported)
    async fn rotate_token(&mut self) -> VkResult<String>;
    
    /// Check if token rotation is supported
    fn supports_rotation(&self) -> bool;
}

/// Single token implementation
pub struct SingleToken {
    token: String,
}

impl SingleToken {
    /// Create a new single token
    pub fn new(token: String) -> Self {
        Self { token }
    }
    
    /// Create a new single token from string reference
    pub fn from_str(token: &str) -> Self {
        Self {
            token: token.to_string(),
        }
    }
}

#[async_trait]
impl TokenGenerator for SingleToken {
    fn get_token(&self) -> &str {
        &self.token
    }
    
    async fn rotate_token(&mut self) -> VkResult<String> {
        Err(VkError::Validation("Single token does not support rotation".to_string()))
    }
    
    fn supports_rotation(&self) -> bool {
        false
    }
}

/// Consistent token generator that rotates tokens round-robin
pub struct ConsistentToken {
    tokens: Vec<String>,
    current: AtomicUsize,
}

impl ConsistentToken {
    /// Create a new consistent token generator
    pub fn new(tokens: Vec<String>) -> VkResult<Self> {
        if tokens.is_empty() {
            return Err(VkError::Validation("Token list cannot be empty".to_string()));
        }
        
        Ok(Self {
            tokens,
            current: AtomicUsize::new(0),
        })
    }
    
    /// Create from string slice
    pub fn from_slice(tokens: &[&str]) -> VkResult<Self> {
        let token_vec: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
        Self::new(token_vec)
    }
    
    /// Get current token index
    fn current_index(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }
    
    /// Move to next token index
    fn advance_token(&self) {
        let current = self.current.load(Ordering::Relaxed);
        let next = (current + 1) % self.tokens.len();
        self.current.store(next, Ordering::Relaxed);
    }
    fn get_token_at(&self, index: usize) -> &str {
        &self.tokens[index % self.tokens.len()]
    }
}

#[async_trait]
impl TokenGenerator for ConsistentToken {
    fn get_token(&self) -> &str {
        self.get_token_at(self.current_index())
    }

    async fn rotate_token(&mut self) -> VkResult<String> {
        self.advance_token();
        Ok(self.get_token().to_string())
    }

    fn supports_rotation(&self) -> bool {
        true
    }
}

/// Token generator that loads tokens from environment variables
pub struct EnvTokenGenerator {
    #[allow(dead_code)]
    prefix: String,
    tokens: Vec<String>,
    current: AtomicUsize,
}

impl EnvTokenGenerator {
    pub fn new(prefix: &str) -> VkResult<Self> {
        let tokens = Self::load_tokens_from_env(prefix)?;
        Ok(Self {
            prefix: prefix.to_string(),
            tokens,
            current: AtomicUsize::new(0),
        })
    }

    fn load_tokens_from_env(prefix: &str) -> VkResult<Vec<String>> {
        let mut tokens = Vec::new();
        let mut i = 1;

        loop {
            let env_var = format!("{}_{}", prefix, i);
            match std::env::var(&env_var) {
                Ok(token) if !token.trim().is_empty() => {
                    tokens.push(token);
                    i += 1;
                }
                Ok(_) => i += 1,
                Err(_) => break,
            }
        }

        if tokens.is_empty() {
            return Err(VkError::Validation(format!(
                "No tokens found with prefix {}",
                prefix
            )));
        }

        Ok(tokens)
    }
}

#[async_trait]
impl TokenGenerator for EnvTokenGenerator {
    fn get_token(&self) -> &str {
        let current = self.current.load(Ordering::Relaxed);
        &self.tokens[current % self.tokens.len()]
    }

    async fn rotate_token(&mut self) -> VkResult<String> {
        let current = self.current.load(Ordering::Relaxed);
        let next = (current + 1) % self.tokens.len();
        self.current.store(next, Ordering::Relaxed);
        Ok(self.get_token().to_string())
    }

    fn supports_rotation(&self) -> bool {
        !self.tokens.is_empty()
    }
}

/// Token management utilities
pub struct TokenManager {
    generator: Box<dyn TokenGenerator>,
    last_used: std::time::Instant,
    usage_count: u64,
}

impl TokenManager {
    /// Create a new token manager
    pub fn new(generator: Box<dyn TokenGenerator>) -> Self {
        Self {
            generator,
            last_used: std::time::Instant::now(),
            usage_count: 0,
        }
    }
    
    /// Get current token
    pub fn get_token(&self) -> &str {
        self.generator.get_token()
    }
    
    /// Get token with rotation option
    pub async fn get_token_with_rotation(&mut self, rotate: bool) -> VkResult<String> {
        if rotate && self.generator.supports_rotation() {
            self.generator.rotate_token().await
        } else {
            Ok(self.generator.get_token().to_string())
        }
    }
    
    /// Mark token as used
    pub fn mark_used(&mut self) {
        self.last_used = std::time::Instant::now();
        self.usage_count += 1;
    }
    
    /// Get token usage statistics
    pub fn get_stats(&self) -> TokenStats {
        TokenStats {
            last_used: self.last_used,
            usage_count: self.usage_count,
            supports_rotation: self.generator.supports_rotation(),
        }
    }
    
    /// Replace the token generator
    pub fn set_generator(&mut self, generator: Box<dyn TokenGenerator>) {
        self.generator = generator;
        self.last_used = std::time::Instant::now();
        self.usage_count = 0;
    }
}

/// Token usage statistics
#[derive(Debug, Clone)]
pub struct TokenStats {
    pub last_used: std::time::Instant,
    pub usage_count: u64,
    pub supports_rotation: bool,
}

/// Create a single token from string
pub fn single_token(token: &str) -> Box<dyn TokenGenerator> {
    Box::new(SingleToken::from_str(token))
}

/// Create a consistent token generator from multiple tokens
pub fn consistent_token(tokens: &[&str]) -> VkResult<Box<dyn TokenGenerator>> {
    let generator = ConsistentToken::from_slice(tokens)?;
    Ok(Box::new(generator))
}

/// Create an environment token generator
pub fn env_token(prefix: &str) -> VkResult<Box<dyn TokenGenerator>> {
    let generator = EnvTokenGenerator::new(prefix)?;
    Ok(Box::new(generator))
}