//! Button colors

/// VK keyboard button colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonColor {
    Primary,
    Secondary,
    Negative,
    Positive,
}

impl ButtonColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Negative => "negative",
            Self::Positive => "positive",
        }
    }
}
