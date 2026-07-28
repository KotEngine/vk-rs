//! VK `execute` API helper

use std::collections::HashMap;

use serde_json::Value;

use crate::api::{Api, ApiRequest, VkApi};
use crate::exception::VkResult;

/// Builder for VK execute scripts
#[derive(Debug, Default)]
pub struct ExecuteBuilder {
    calls: Vec<String>,
}

impl ExecuteBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append API method call: `return API.method(params);`
    pub fn call(mut self, method: &str, params: &HashMap<String, String>) -> Self {
        let mut parts: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{k}: \"{v}\""))
            .collect();
        parts.sort();
        let inner = parts.join(", ");
        self.calls
            .push(format!("API.{method}({{{inner}}})"));
        self
    }

    /// Append raw VKScript fragment
    pub fn raw(mut self, code: &str) -> Self {
        self.calls.push(code.to_string());
        self
    }

    pub fn build(self) -> String {
        if self.calls.is_empty() {
            return String::new();
        }
        if self.calls.len() == 1 {
            return format!("return {};", self.calls[0]);
        }
        format!(
            "return [{}];",
            self.calls
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl Api {
    /// Run VKScript via `execute` method
    pub async fn execute(&self, code: &str) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("code".to_string(), code.to_string());
        self.request("execute", &params).await
    }

    /// Run built execute script
    pub async fn execute_builder(&self, builder: ExecuteBuilder) -> VkResult<Value> {
        self.execute(&builder.build()).await
    }

    /// Batch requests via execute when possible, otherwise sequential `request_many`
    pub async fn request_batch(&self, requests: &[ApiRequest]) -> VkResult<Vec<Value>> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }
        if requests.len() == 1 {
            let r = &requests[0];
            let v = self.request(&r.method, &r.params).await?;
            return Ok(vec![v]);
        }

        let mut builder = ExecuteBuilder::new();
        for req in requests {
            builder = builder.call(&req.method, &req.params);
        }
        let response = self.execute_builder(builder).await?;
        if let Some(arr) = response.as_array() {
            return Ok(arr.clone());
        }
        Ok(vec![response])
    }
}
