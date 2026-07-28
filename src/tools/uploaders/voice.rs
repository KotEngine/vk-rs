//! Voice message and graffiti uploaders

use std::collections::HashMap;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use super::base::{BaseUploader, Uploader};

/// Upload voice message (type=audio_message)
pub struct VoiceMessageUploader {
    peer_id: Option<i64>,
}

impl VoiceMessageUploader {
    pub fn new() -> Self {
        Self { peer_id: None }
    }

    pub fn with_peer_id(mut self, peer_id: i64) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    async fn upload_doc_type(
        &self,
        api: &Api,
        file_path: &str,
        doc_type: &str,
        field_name: &str,
        default_filename: &str,
    ) -> VkResult<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("type".to_string(), doc_type.to_string());
        if let Some(peer_id) = self.peer_id {
            params.insert("peer_id".to_string(), peer_id.to_string());
        }

        let server = api.request("docs.getMessagesUploadServer", &params).await?;
        let upload_url = server
            .get("upload_url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| {
                crate::exception::VkError::Validation("Missing upload_url".to_string())
            })?;

        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(default_filename);

        let bytes = BaseUploader::read_file(file_path).await?;
        let uploaded =
            BaseUploader::upload_multipart(upload_url, field_name, filename, bytes).await?;

        let mut save_params = HashMap::new();
        if let Some(obj) = uploaded.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    save_params.insert(k.clone(), s.to_string());
                }
            }
        }

        let saved = api.request("docs.save", &save_params).await?;
        Ok(saved.get("doc").cloned().unwrap_or(saved))
    }
}

impl Default for VoiceMessageUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for VoiceMessageUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        let doc = self
            .upload_doc_type(api, file_path, "audio_message", "file", "voice.ogg")
            .await?;
        let owner_id = doc.get("owner_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let id = doc.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let access_key = doc.get("access_key").and_then(|v| v.as_str());
        Ok(BaseUploader::generate_attachment_string(
            "doc",
            owner_id,
            id,
            access_key,
        ))
    }

    fn attachment_type(&self) -> &str {
        "doc"
    }
}

/// Upload graffiti sticker
pub struct GraffitiUploader {
    peer_id: Option<i64>,
}

impl GraffitiUploader {
    pub fn new() -> Self {
        Self { peer_id: None }
    }

    pub fn with_peer_id(mut self, peer_id: i64) -> Self {
        self.peer_id = Some(peer_id);
        self
    }
}

impl Default for GraffitiUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for GraffitiUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        let uploader = VoiceMessageUploader {
            peer_id: self.peer_id,
        };
        let doc = uploader
            .upload_doc_type(api, file_path, "graffiti", "file", "graffiti.png")
            .await?;
        let owner_id = doc.get("owner_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let id = doc.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let access_key = doc.get("access_key").and_then(|v| v.as_str());
        Ok(BaseUploader::generate_attachment_string(
            "doc",
            owner_id,
            id,
            access_key,
        ))
    }

    fn attachment_type(&self) -> &str {
        "doc"
    }
}
