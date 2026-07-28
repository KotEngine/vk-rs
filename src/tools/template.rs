//! Message templates and carousels

use serde_json::{json, Value};

/// VK template / carousel element
#[derive(Debug, Clone)]
pub struct TemplateElement {
    pub title: String,
    pub description: Option<String>,
    pub photo_id: Option<String>,
    pub buttons: Vec<Value>,
    pub action: Option<Value>,
}

impl TemplateElement {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            photo_id: None,
            buttons: Vec::new(),
            action: None,
        }
    }

    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn photo_id(mut self, photo_id: impl Into<String>) -> Self {
        self.photo_id = Some(photo_id.into());
        self
    }

    pub fn button(mut self, button: Value) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn open_link(mut self, link: impl Into<String>, title: impl Into<String>) -> Self {
        self.action = Some(json!({
            "type": "open_link",
            "link": link.into(),
            "target": "internal",
        }));
        self.title = title.into();
        self
    }

    pub fn to_json(&self) -> Value {
        let mut obj = json!({ "title": self.title });
        if let Some(desc) = &self.description {
            obj["description"] = json!(desc);
        }
        if let Some(photo) = &self.photo_id {
            obj["photo_id"] = json!(photo);
        }
        if !self.buttons.is_empty() {
            obj["buttons"] = Value::Array(self.buttons.clone());
        }
        if let Some(action) = &self.action {
            obj["action"] = action.clone();
        }
        obj
    }
}

/// Message template / carousel builder
#[derive(Debug, Clone, Default)]
pub struct Template {
    pub template_type: String,
    pub elements: Vec<TemplateElement>,
}

impl Template {
    pub fn new() -> Self {
        Self {
            template_type: "carousel".to_string(),
            elements: Vec::new(),
        }
    }

    pub fn carousel() -> Self {
        Self::new()
    }

    pub fn add(mut self, element: TemplateElement) -> Self {
        self.elements.push(element);
        self
    }

    pub fn element(mut self, element: TemplateElement) -> Self {
        self.elements.push(element);
        self
    }

    pub fn to_json(&self) -> Value {
        json!({
            "type": self.template_type,
            "elements": self.elements.iter().map(|e| e.to_json()).collect::<Vec<_>>(),
        })
    }

    pub fn to_attachment_json(&self) -> String {
        self.to_json().to_string()
    }
}
