//! Keyboard button

use super::{ButtonAction, ButtonColor};

/// Single keyboard button
#[derive(Debug, Clone)]
pub struct KeyboardButton {
    pub action: ButtonAction,
    pub color: Option<ButtonColor>,
}

impl KeyboardButton {
    pub fn new(action: ButtonAction, color: Option<ButtonColor>) -> Self {
        Self { action, color }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut btn = serde_json::json!({ "action": self.action.to_json() });
        if let Some(color) = &self.color {
            btn["color"] = serde_json::Value::String(color.as_str().to_string());
        }
        btn
    }
}
