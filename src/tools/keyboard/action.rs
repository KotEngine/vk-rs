//! Keyboard button actions

use serde_json::{json, Value};

/// Keyboard button payload
pub type Payload = String;

/// Keyboard button action types
#[derive(Debug, Clone)]
pub enum ButtonAction {
    Text {
        label: String,
        payload: Option<Payload>,
    },
    Callback {
        label: String,
        payload: Payload,
    },
    OpenLink {
        label: String,
        link: String,
        payload: Option<Payload>,
    },
    Location {
        payload: Option<Payload>,
    },
    VkPay {
        payload: Option<Payload>,
        hash: Option<String>,
    },
    VkApps {
        app_id: i64,
        owner_id: i64,
        label: Option<String>,
        payload: Option<Payload>,
        hash: Option<String>,
    },
}

impl ButtonAction {
    pub fn text(label: impl Into<String>) -> Self {
        Self::Text {
            label: label.into(),
            payload: None,
        }
    }

    pub fn text_with_payload(label: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::Text {
            label: label.into(),
            payload: Some(payload.into()),
        }
    }

    pub fn callback(label: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::Callback {
            label: label.into(),
            payload: payload.into(),
        }
    }

    pub fn open_link(label: impl Into<String>, link: impl Into<String>) -> Self {
        Self::OpenLink {
            label: label.into(),
            link: link.into(),
            payload: None,
        }
    }

    pub fn location() -> Self {
        Self::Location { payload: None }
    }

    pub fn vkpay() -> Self {
        Self::VkPay {
            payload: None,
            hash: None,
        }
    }

    pub fn vk_apps(app_id: i64, owner_id: i64) -> Self {
        Self::VkApps {
            app_id,
            owner_id,
            label: None,
            payload: None,
            hash: None,
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            Self::Text { label, payload } => {
                let mut action = json!({ "type": "text", "label": label });
                if let Some(p) = payload {
                    action["payload"] = json!(p);
                }
                action
            }
            Self::Callback { label, payload } => {
                json!({ "type": "callback", "label": label, "payload": payload })
            }
            Self::OpenLink { label, link, payload } => {
                let mut action = json!({ "type": "open_link", "label": label, "link": link });
                if let Some(p) = payload {
                    action["payload"] = json!(p);
                }
                action
            }
            Self::Location { payload } => {
                let mut action = json!({ "type": "location" });
                if let Some(p) = payload {
                    action["payload"] = json!(p);
                }
                action
            }
            Self::VkPay { payload, hash } => {
                let mut action = json!({ "type": "vkpay" });
                if let Some(p) = payload {
                    action["payload"] = json!(p);
                }
                if let Some(h) = hash {
                    action["hash"] = json!(h);
                }
                action
            }
            Self::VkApps {
                app_id,
                owner_id,
                label,
                payload,
                hash,
            } => {
                let mut action = json!({
                    "type": "open_app",
                    "app_id": app_id,
                    "owner_id": owner_id,
                });
                if let Some(l) = label {
                    action["label"] = json!(l);
                }
                if let Some(p) = payload {
                    action["payload"] = json!(p);
                }
                if let Some(h) = hash {
                    action["hash"] = json!(h);
                }
                action
            }
        }
    }
}
