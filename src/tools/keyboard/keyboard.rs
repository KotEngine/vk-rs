//! Keyboard builder

use super::button::KeyboardButton;
use super::{ButtonAction, ButtonColor};
use serde_json::json;

/// VK inline/regular keyboard
#[derive(Debug, Clone, Default)]
pub struct Keyboard {
    one_time: bool,
    inline: bool,
    rows: Vec<Vec<KeyboardButton>>,
}

impl Keyboard {
    pub fn new(one_time: bool, inline: bool) -> Self {
        Self {
            one_time,
            inline,
            rows: Vec::new(),
        }
    }

    pub fn one_time(mut self, one_time: bool) -> Self {
        self.one_time = one_time;
        self
    }

    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    pub fn row(&mut self) -> &mut Self {
        self.rows.push(Vec::new());
        self
    }

    pub fn add(
        &mut self,
        action: ButtonAction,
        color: Option<ButtonColor>,
    ) -> &mut Self {
        if self.rows.is_empty() {
            self.row();
        }
        if let Some(row) = self.rows.last_mut() {
            row.push(KeyboardButton::new(action, color));
        }
        self
    }

    pub fn add_text(&mut self, label: impl Into<String>, color: Option<ButtonColor>) -> &mut Self {
        self.add(ButtonAction::text(label), color)
    }

    pub fn get_json(&self) -> String {
        self.to_json()
    }

    pub fn to_json(&self) -> String {
        let buttons: Vec<_> = self
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|b| b.to_json())
                    .collect::<Vec<_>>()
            })
            .collect();

        json!({
            "one_time": self.one_time,
            "inline": self.inline,
            "buttons": buttons,
        })
        .to_string()
    }
}
