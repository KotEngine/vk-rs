//! VK text formatting with UTF-16-LE byte offsets

use serde_json::{json, Value};

/// VK format type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatType {
    Bold,
    Italic,
    Underline,
    Url,
    Strikethrough,
}

impl FormatType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::Underline => "underline",
            Self::Url => "url",
            Self::Strikethrough => "strikethrough",
        }
    }
}

/// Calculate UTF-16 code unit offset for VK format_data
pub fn calculate_offset(string: &str) -> usize {
    string.encode_utf16().count()
}

/// Formatted text fragment
#[derive(Debug, Clone)]
pub struct Format {
    pub string: String,
    pub format_type: Option<FormatType>,
    pub offset: usize,
    pub length: usize,
    pub data: Value,
    pub other_formats: Vec<Format>,
}

impl Format {
    pub fn plain(string: impl Into<String>) -> Self {
        let string = string.into();
        Self {
            length: string.chars().count(),
            string,
            format_type: None,
            offset: 0,
            data: Value::Null,
            other_formats: Vec::new(),
        }
    }

    fn with_type(string: impl Into<String>, format_type: FormatType, data: Value) -> Self {
        let string = string.into();
        Self {
            length: string.chars().count(),
            string,
            format_type: Some(format_type),
            offset: 0,
            data,
            other_formats: Vec::new(),
        }
    }

    fn add_offset_recursive(formats: &mut [Format], offset: usize) {
        for fmt in formats.iter_mut() {
            fmt.offset += offset;
            Self::add_offset_recursive(&mut fmt.other_formats, offset);
        }
    }

    pub fn add_other(mut self, other: Format) -> Self {
        let rhs_offset = calculate_offset(&self.string);
        let mut other = other;
        other.offset += rhs_offset;
        Self::add_offset_recursive(&mut other.other_formats, rhs_offset);
        self.string.push_str(&other.string);
        self.other_formats.push(other);
        self.length = self.string.chars().count();
        self
    }

    pub fn as_data(&self, offset: usize, version: i32) -> Value {
        let mut items = Vec::new();

        if let Some(ref fmt_type) = self.format_type {
            let mut item = json!({
                "type": fmt_type.as_str(),
                "offset": self.offset + offset,
                "length": self.length,
            });
            if fmt_type == &FormatType::Url {
                if let Some(url) = self.data.get("url") {
                    item["url"] = url.clone();
                }
            }
            items.push(item);
        }

        for fmt in &self.other_formats {
            if let Some(arr) = fmt.as_data(0, version).get("items").and_then(|i| i.as_array()) {
                items.extend(arr.iter().cloned());
            }
        }

        json!({ "version": version, "items": items })
    }

    pub fn as_raw_data(&self, offset: usize) -> String {
        self.as_data(offset, 1).to_string()
    }

    /// Render plain text and VK format_data JSON string
    pub fn render(&self) -> (String, String) {
        (self.string.clone(), self.as_raw_data(0))
    }
}

impl std::ops::Add<Format> for Format {
    type Output = Format;
    fn add(self, rhs: Format) -> Format {
        self.add_other(rhs)
    }
}

impl std::ops::Add<&str> for Format {
    type Output = Format;
    fn add(mut self, rhs: &str) -> Format {
        self.string.push_str(rhs);
        self.length = self.string.chars().count();
        self
    }
}

pub fn bold(string: impl Into<String>) -> Format {
    Format::with_type(string, FormatType::Bold, Value::Null)
}

pub fn italic(string: impl Into<String>) -> Format {
    Format::with_type(string, FormatType::Italic, Value::Null)
}

pub fn underline(string: impl Into<String>) -> Format {
    Format::with_type(string, FormatType::Underline, Value::Null)
}

pub fn url(string: impl Into<String>, href: impl Into<String>) -> Format {
    Format::with_type(string, FormatType::Url, json!({ "url": href.into() }))
}

pub fn strikethrough(string: impl Into<String>) -> Format {
    Format::with_type(string, FormatType::Strikethrough, Value::Null)
}

/// Chain multiple formats: `bold(italic("hello"))`
pub fn chain(parts: &[Format]) -> Format {
    let mut result = Format::plain("");
    for part in parts {
        result = result.add_other(part.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_offset_ascii() {
        assert_eq!(calculate_offset("hello"), 5);
    }

    #[test]
    fn utf16_offset_cyrillic() {
        assert_eq!(calculate_offset("привет"), 6);
    }

    #[test]
    fn bold_render_has_items() {
        let fmt = bold("test");
        let data = fmt.as_data(0, 1);
        assert_eq!(data["items"][0]["type"], "bold");
    }
}
