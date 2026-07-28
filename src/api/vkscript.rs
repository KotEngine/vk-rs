//! VKScript builder for `execute` API calls

use std::collections::HashMap;
use std::fmt::Write;

/// Builder for VKScript code passed to `execute`
#[derive(Debug, Clone, Default)]
pub struct VkScriptBuilder {
    lines: Vec<String>,
    return_var: Option<String>,
}

impl VkScriptBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn raw(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn comment(mut self, text: impl AsRef<str>) -> Self {
        self.lines.push(format!("// {}", text.as_ref()));
        self
    }

    pub fn var(mut self, name: &str, value: impl Into<String>) -> Self {
        self.lines
            .push(format!("var {} = {};", name, value.into()));
        self
    }

    pub fn assign(mut self, name: &str, expr: impl Into<String>) -> Self {
        self.lines
            .push(format!("{} = {};", name, expr.into()));
        self
    }

    /// `API.method(params)` stored in `result_var`
    pub fn api_call(mut self, method: &str, params: &str, result_var: &str) -> Self {
        self.lines.push(format!(
            "var {} = API.{}({{{}}});",
            result_var, method, params
        ));
        self
    }

    /// `API.method(params)` without storing
    pub fn api_call_void(mut self, method: &str, params: &str) -> Self {
        self.lines
            .push(format!("API.{}({{{}}});", method, params));
        self
    }

    pub fn push(mut self, array_var: &str, value: impl Into<String>) -> Self {
        self.lines
            .push(format!("{}.push({});", array_var, value.into()));
        self
    }

    pub fn return_var(mut self, name: &str) -> Self {
        self.return_var = Some(name.to_string());
        self
    }

    pub fn return_expr(mut self, expr: impl Into<String>) -> Self {
        self.lines.push(format!("return {};", expr.into()));
        self
    }

    pub fn build(self) -> String {
        let mut script = self.lines.join("\n");
        if let Some(var) = self.return_var {
            if !script.is_empty() {
                script.push('\n');
            }
            let _ = write!(script, "return {};", var);
        }
        script
    }
}

/// Format VK API params map for VKScript `{key: value}` blocks
pub fn format_vk_params(params: &HashMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| {
            if v.parse::<i64>().is_ok() || v == "true" || v == "false" {
                format!("\"{}\": {}", k, v)
            } else {
                format!("\"{}\": \"{}\"", k, v.replace('"', "\\\""))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Batch VKScript: run multiple API calls and return array of results
pub struct VkScriptBatch {
    calls: Vec<(String, String)>,
}

impl VkScriptBatch {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn add(mut self, method: &str, params: HashMap<String, String>) -> Self {
        self.calls.push((method.to_string(), format_vk_params(&params)));
        self
    }

    pub fn build(self) -> String {
        let mut b = VkScriptBuilder::new().var("results", "[]");
        for (i, (method, params)) in self.calls.into_iter().enumerate() {
            let var = format!("r{}", i);
            b = b.api_call(&method, &params, &var);
            b = b.push("results", var);
        }
        b.return_var("results").build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vkscript_batch_builds_return_array() {
        let script = VkScriptBatch::new()
            .add(
                "users.get",
                HashMap::from([("user_ids".into(), "1".into())]),
            )
            .build();
        assert!(script.contains("API.users.get"));
        assert!(script.contains("return results"));
    }
}
