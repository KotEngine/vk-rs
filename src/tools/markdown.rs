//! Markdown to VK Format parser

use crate::tools::formatting::{bold, chain, italic, underline, url, Format};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Text,
    Esc,
    Backslash,
    BoldOpen,
    BoldClose,
    ItalicOpen,
    ItalicClose,
    UnderlineOpen,
    UnderlineClose,
    UrlOpen,
    UrlMid,
    UrlClose,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    value: String,
}

/// Parse markdown string into a VK `Format` object
pub fn parse_markdown(text: &str) -> Format {
    let tokens = tokenize(text);
    let mut stack: Vec<Frame> = vec![Frame::root()];

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token.kind {
            TokenKind::Text | TokenKind::Esc | TokenKind::Backslash => {
                let ch = match token.kind {
                    TokenKind::Esc => token.value.chars().nth(1).unwrap_or(' '),
                    TokenKind::Backslash => '\\',
                    _ => token.value.chars().next().unwrap_or(' '),
                };
                stack.last_mut().unwrap().parts.push(Format::plain(ch.to_string()));
            }
            TokenKind::BoldOpen | TokenKind::ItalicOpen => {
                let fmt = if token.kind == TokenKind::BoldOpen {
                    "bold"
                } else {
                    "italic"
                };
                if stack.last().unwrap().ctx_type.as_deref() == Some(fmt) {
                    close_frame(&mut stack, token.value.clone());
                } else {
                    stack.push(Frame::new(fmt, token.value.clone()));
                }
            }
            TokenKind::BoldClose | TokenKind::ItalicClose => {
                let fmt = if token.kind == TokenKind::BoldClose {
                    "bold"
                } else {
                    "italic"
                };
                if stack.last().unwrap().ctx_type.as_deref() == Some(fmt) {
                    close_frame(&mut stack, token.value.clone());
                } else {
                    stack
                        .last_mut()
                        .unwrap()
                        .parts
                        .push(Format::plain(token.value.clone()));
                }
            }
            TokenKind::UnderlineOpen => {
                stack.push(Frame::new("underline", token.value.clone()));
            }
            TokenKind::UnderlineClose => {
                if stack.last().unwrap().ctx_type.as_deref() == Some("underline") {
                    close_frame(&mut stack, token.value.clone());
                } else {
                    stack
                        .last_mut()
                        .unwrap()
                        .parts
                        .push(Format::plain(token.value.clone()));
                }
            }
            TokenKind::UrlOpen => {
                stack.push(Frame::new("url", token.value.clone()));
            }
            TokenKind::UrlMid => {
                if let Some(frame) = stack.last_mut() {
                    if frame.ctx_type.as_deref() == Some("url") {
                        let label = frame
                            .parts
                            .iter()
                            .map(|f| f.string.as_str())
                            .collect::<String>();
                        frame.url_label = Some(label);
                        frame.parts.clear();
                    }
                }
            }
            TokenKind::UrlClose => {
                if stack.last().unwrap().ctx_type.as_deref() == Some("url") {
                    let frame = stack.pop().unwrap();
                    let href = frame
                        .parts
                        .iter()
                        .map(|f| f.string.as_str())
                        .collect::<String>();
                    let label = frame.url_label.unwrap_or_else(|| href.clone());
                    stack.last_mut().unwrap().parts.push(url(label, href));
                }
            }
        }
        i += 1;
    }

    while stack.len() > 1 {
        close_frame(&mut stack, String::new());
    }

    let parts = stack.pop().unwrap().parts;
    if parts.is_empty() {
        Format::plain("")
    } else if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        chain(&parts)
    }
}

#[derive(Debug)]
struct Frame {
    ctx_type: Option<String>,
    parts: Vec<Format>,
    open_marker: String,
    url_label: Option<String>,
}

impl Frame {
    fn root() -> Self {
        Self {
            ctx_type: None,
            parts: Vec::new(),
            open_marker: String::new(),
            url_label: None,
        }
    }

    fn new(ctx_type: &str, marker: String) -> Self {
        Self {
            ctx_type: Some(ctx_type.to_string()),
            parts: Vec::new(),
            open_marker: marker,
            url_label: None,
        }
    }
}

fn close_frame(stack: &mut Vec<Frame>, closing: String) {
    if stack.len() <= 1 {
        return;
    }
    let frame = stack.pop().unwrap();
    let inner = frame
        .parts
        .into_iter()
        .reduce(|a, b| a.add_other(b))
        .unwrap_or_else(|| Format::plain(""));
    if inner.string.is_empty() {
        stack
            .last_mut()
            .unwrap()
            .parts
            .push(Format::plain(frame.open_marker + &closing));
        return;
    }
    let formatted = match frame.ctx_type.as_deref() {
        Some("bold") => bold(inner.string),
        Some("italic") => italic(inner.string),
        Some("underline") => underline(inner.string),
        _ => inner,
    };
    stack.last_mut().unwrap().parts.push(formatted);
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::Esc,
                        value: format!("\\{next}"),
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Backslash,
                        value: "\\".to_string(),
                    });
                }
            }
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if tokens.last().map(|t| t.kind) == Some(TokenKind::BoldOpen) {
                        tokens.push(Token {
                            kind: TokenKind::BoldClose,
                            value: "**".to_string(),
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::BoldOpen,
                            value: "**".to_string(),
                        });
                    }
                } else if tokens.last().map(|t| t.kind) == Some(TokenKind::ItalicOpen) {
                    tokens.push(Token {
                        kind: TokenKind::ItalicClose,
                        value: "*".to_string(),
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::ItalicOpen,
                        value: "*".to_string(),
                    });
                }
            }
            '<' => {
                let mut tag = String::from('<');
                while let Some(c) = chars.next() {
                    tag.push(c);
                    if c == '>' {
                        break;
                    }
                }
                if tag == "<u>" {
                    tokens.push(Token {
                        kind: TokenKind::UnderlineOpen,
                        value: tag,
                    });
                } else if tag == "</u>" {
                    tokens.push(Token {
                        kind: TokenKind::UnderlineClose,
                        value: tag,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Text,
                        value: tag,
                    });
                }
            }
            '[' => {
                tokens.push(Token {
                    kind: TokenKind::UrlOpen,
                    value: "[".to_string(),
                });
            }
            ']' if chars.peek() == Some(&'(') => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::UrlMid,
                    value: "](".to_string(),
                });
            }
            ')' => {
                tokens.push(Token {
                    kind: TokenKind::UrlClose,
                    value: ")".to_string(),
                });
            }
            _ => {
                let mut s = ch.to_string();
                while let Some(&next) = chars.peek() {
                    if matches!(next, '*' | '[' | ']' | '(' | ')' | '<' | '\\') {
                        break;
                    }
                    s.push(chars.next().unwrap());
                }
                tokens.push(Token {
                    kind: TokenKind::Text,
                    value: s,
                });
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bold() {
        let fmt = parse_markdown("**hi**");
        assert!(!fmt.string.is_empty());
    }
}
