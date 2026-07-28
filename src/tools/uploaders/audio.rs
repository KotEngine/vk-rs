//! Audio uploader

use std::collections::HashMap;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use super::base::{BaseUploader, Uploader};

/// Upload audio file via `audio.getUploadServer`
pub struct AudioUploader;

impl AudioUploader {
    pub fn new() -> Self {
        Self
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let server = api.request("audio.getUploadServer", &HashMap::new()).await?;
        let upload_url = server
            .get("upload_url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| {
                crate::exception::VkError::Validation("Missing upload_url".to_string())
            })?;

        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.mp3");

        let bytes = BaseUploader::read_file(file_path).await?;
        let uploaded =
            BaseUploader::upload_multipart(upload_url, "file", filename, bytes).await?;

        let mut save_params = HashMap::new();
        if let Some(obj) = uploaded.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    save_params.insert(k.clone(), s.to_string());
                }
            }
        }

        let saved = api.request("audio.save", &save_params).await?;
        Ok(saved.get(0).cloned().unwrap_or(saved))
    }
}

impl Default for AudioUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for AudioUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        let audio = self.raw_upload(api, file_path).await?;
        let owner_id = audio.get("owner_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let id = audio.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let access_key = audio.get("access_key").and_then(|v| v.as_str());
        Ok(BaseUploader::generate_attachment_string(
            "audio",
            owner_id,
            id,
            access_key,
        ))
    }

    fn attachment_type(&self) -> &str {
        "audio"
    }
}
