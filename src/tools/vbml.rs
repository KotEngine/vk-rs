//! VBML-style pattern matching for message text

use std::collections::HashMap;

use regex::Regex;
use serde_json::{json, Value};

/// Compiled VBML pattern segment
#[derive(Debug, Clone)]
enum Segment {
    Literal(String),
    Capture {
        name: String,
        greedy: bool,
        word: bool,
    },
    Optional(Box<Pattern>),
}

/// Compiled VBML pattern
#[derive(Debug, Clone)]
pub struct Pattern {
    segments: Vec<Segment>,
}

impl Pattern {
    pub fn compile(pattern: &str) -> Result<Self, String> {
        let mut segments = Vec::new();
        let mut chars = pattern.chars().peekable();
        let mut literal = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    if !literal.is_empty() {
                        segments.push(Segment::Literal(std::mem::take(&mut literal)));
                    }
                    let mut name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '}' || c == '!' {
                            break;
                        }
                        name.push(chars.next().unwrap());
                    }
                    if name.is_empty() {
                        return Err("empty capture name in VBML pattern".to_string());
                    }
                    let mut greedy = false;
                    let mut word = true;
                    if chars.peek() == Some(&'!') {
                        chars.next();
                        let mut modifier = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '}' {
                                break;
                            }
                            modifier.push(chars.next().unwrap());
                        }
                        match modifier.as_str() {
                            "greedy" => {
                                greedy = true;
                                word = false;
                            }
                            "word" => word = true,
                            "int" | "float" => word = false,
                            other => return Err(format!("unknown VBML modifier: {other}")),
                        }
                    }
                    if chars.next() != Some('}') {
                        return Err("unclosed VBML capture".to_string());
                    }
                    segments.push(Segment::Capture { name, greedy, word });
                }
                '[' => {
                    if !literal.is_empty() {
                        segments.push(Segment::Literal(std::mem::take(&mut literal)));
                    }
                    let mut inner = String::new();
                    let mut depth = 1usize;
                    while let Some(c) = chars.next() {
                        match c {
                            '[' => {
                                depth += 1;
                                inner.push(c);
                            }
                            ']' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                inner.push(c);
                            }
                            _ => inner.push(c),
                        }
                    }
                    let sub = Pattern::compile(&inner)?;
                    segments.push(Segment::Optional(Box::new(sub)));
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        literal.push(next);
                    }
                }
                c => literal.push(c),
            }
        }

        if !literal.is_empty() {
            segments.push(Segment::Literal(literal));
        }

        Ok(Self { segments })
    }

    /// Match text and return captured values as JSON map
    pub fn check(&self, text: &str) -> Option<HashMap<String, Value>> {
        let mut ctx = MatchContext {
            text,
            pos: 0,
            captures: HashMap::new(),
        };
        if self.match_segments(&self.segments, &mut ctx) && ctx.pos == text.len() {
            Some(ctx.captures)
        } else {
            None
        }
    }

    fn match_segments(&self, segments: &[Segment], ctx: &mut MatchContext<'_>) -> bool {
        let start = ctx.pos;
        for seg in segments {
            if !self.match_segment(seg, ctx) {
                ctx.pos = start;
                return false;
            }
        }
        true
    }

    fn match_segment(&self, seg: &Segment, ctx: &mut MatchContext<'_>) -> bool {
        match seg {
            Segment::Literal(lit) => {
                if ctx.text[ctx.pos..].starts_with(lit) {
                    ctx.pos += lit.len();
                    true
                } else {
                    false
                }
            }
            Segment::Capture { name, greedy, word } => {
                let rest = &ctx.text[ctx.pos..];
                if rest.is_empty() {
                    return false;
                }
                let captured = if *greedy {
                    rest.to_string()
                } else if *word {
                    rest.split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string()
                } else {
                    let re = Regex::new(r"^-?\d+(\.\d+)?").unwrap();
                    re.find(rest)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                };
                if captured.is_empty() {
                    return false;
                }
                ctx.pos += captured.len();
                ctx.captures
                    .insert(name.clone(), Value::String(captured));
                true
            }
            Segment::Optional(sub) => {
                let saved = ctx.pos;
                let saved_caps = ctx.captures.clone();
                if sub.match_segments(&sub.segments, ctx) {
                    true
                } else {
                    ctx.pos = saved;
                    ctx.captures = saved_caps;
                    true
                }
            }
        }
    }
}

struct MatchContext<'a> {
    text: &'a str,
    pos: usize,
    captures: HashMap<String, Value>,
}

/// VBML patcher — runs multiple patterns, first match wins
#[derive(Debug, Default)]
pub struct Patcher {
    patterns: Vec<Pattern>,
}

impl Patcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pattern(mut self, pattern: &str) -> Result<Self, String> {
        self.patterns.push(Pattern::compile(pattern)?);
        Ok(self)
    }

    pub fn check(&self, text: &str) -> Option<HashMap<String, Value>> {
        for pattern in &self.patterns {
            if let Some(ctx) = pattern.check(text) {
                return Some(ctx);
            }
        }
        None
    }
}

/// Convert capture map to `RuleResult`-compatible JSON context
pub fn captures_to_context(captures: HashMap<String, Value>) -> Value {
    let mut map = serde_json::Map::new();
    for (k, v) in captures {
        map.insert(k, v);
    }
    json!(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_and_capture() {
        let p = Pattern::compile("hello {name}").unwrap();
        let caps = p.check("hello world").unwrap();
        assert_eq!(caps.get("name").and_then(|v| v.as_str()), Some("world"));
    }

    #[test]
    fn greedy_capture() {
        let p = Pattern::compile("cmd {args!greedy}").unwrap();
        let caps = p.check("cmd a b c").unwrap();
        assert_eq!(caps.get("args").and_then(|v| v.as_str()), Some("a b c"));
    }

    #[test]
    fn optional_segment() {
        let p = Pattern::compile("hi[ there]").unwrap();
        assert!(p.check("hi there").is_some());
        assert!(p.check("hi").is_some());
    }
}
