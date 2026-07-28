//! Parse VK message attachments from JSON

use serde_json::Value;

use super::{Attachment, AttachmentType};

/// Parsed attachment with optional nested data
#[derive(Debug, Clone)]
pub struct ParsedAttachment {
    pub attachment: Attachment,
    pub raw: Value,
}

/// Parse attachments array from a message object
pub fn parse_attachments_from_message(message: &Value) -> Vec<ParsedAttachment> {
    message
        .get("attachments")
        .and_then(|a| a.as_array())
        .map(|arr| arr.iter().filter_map(parse_single_attachment).collect())
        .unwrap_or_default()
}

pub fn parse_single_attachment(att: &Value) -> Option<ParsedAttachment> {
    let att_type = att.get("type")?.as_str()?;
    let inner = att.get(att_type)?;
    let owner_id = inner.get("owner_id").and_then(|o| o.as_i64()).unwrap_or(0);
    let id = inner.get("id").and_then(|i| i.as_i64()).unwrap_or(0);
    let mut attachment = Attachment::new(AttachmentType::from_str(att_type), owner_id, id);
    if let Some(key) = inner.get("access_key").and_then(|k| k.as_str()) {
        attachment = attachment.with_access_key(key.to_string());
    }
    attachment = attachment.with_data(inner.clone());
    Some(ParsedAttachment {
        attachment,
        raw: att.clone(),
    })
}

/// Extract photo sizes URL (largest available)
pub fn photo_largest_url(parsed: &ParsedAttachment) -> Option<String> {
    if parsed.attachment.attachment_type != AttachmentType::Photo {
        return None;
    }
    let sizes = parsed.raw.get("photo")?.get("sizes")?.as_array()?;
    sizes
        .iter()
        .max_by_key(|s| s.get("width").and_then(|w| w.as_u64()).unwrap_or(0))
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from)
}

/// Extract doc URL
pub fn doc_url(parsed: &ParsedAttachment) -> Option<String> {
    parsed
        .raw
        .get("doc")?
        .get("url")
        .and_then(|u| u.as_str())
        .map(String::from)
}

/// Extract audio artist + title
pub fn audio_title(parsed: &ParsedAttachment) -> Option<String> {
    let audio = parsed.raw.get("audio")?;
    let artist = audio.get("artist").and_then(|a| a.as_str()).unwrap_or("");
    let title = audio.get("title").and_then(|t| t.as_str()).unwrap_or("");
    Some(format!("{artist} — {title}"))
}

/// Extract video title
pub fn video_title(parsed: &ParsedAttachment) -> Option<String> {
    parsed
        .raw
        .get("video")?
        .get("title")
        .and_then(|t| t.as_str())
        .map(String::from)
}

/// Filter parsed attachments by type
pub fn filter_by_type<'a>(
    list: &'a [ParsedAttachment],
    ty: &AttachmentType,
) -> Vec<&'a ParsedAttachment> {
    list.iter().filter(|p| &p.attachment.attachment_type == ty).collect()
}

/// Join attachment strings for `messages.send`
pub fn attachment_strings(list: &[ParsedAttachment]) -> String {
    list.iter()
        .map(|p| p.attachment.to_attachment_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_photo_attachment() {
        let msg = json!({
            "attachments": [{
                "type": "photo",
                "photo": { "id": 5, "owner_id": 1, "sizes": [
                    { "width": 100, "url": "http://small" },
                    { "width": 800, "url": "http://large" }
                ]}
            }]
        });
        let parsed = parse_attachments_from_message(&msg);
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            photo_largest_url(&parsed[0]).as_deref(),
            Some("http://large")
        );
    }
}
