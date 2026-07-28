//! Callback button event data payloads

use serde::{Deserialize, Serialize};

/// Show snackbar on callback button press
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowSnackbarEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub text: String,
}

impl ShowSnackbarEvent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            event_type: "show_snackbar".to_string(),
            text: text.into(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Open link from callback button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLinkEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub link: String,
}

impl OpenLinkEvent {
    pub fn new(link: impl Into<String>) -> Self {
        Self {
            event_type: "open_link".to_string(),
            link: link.into(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Open VK mini-app from callback button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAppEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<i64>,
    pub app_id: i64,
    pub hash: String,
}

impl OpenAppEvent {
    pub fn new(app_id: i64, hash: impl Into<String>) -> Self {
        Self {
            event_type: "open_app".to_string(),
            owner_id: None,
            app_id,
            hash: hash.into(),
        }
    }

    pub fn with_owner_id(mut self, owner_id: i64) -> Self {
        self.owner_id = Some(owner_id);
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}
