//! Document uploaders

use std::collections::HashMap;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use super::base::{BaseUploader, Uploader};

/// Upload document via `docs.getUploadServer`
pub struct DocUploader {
    peer_id: Option<i64>,
    group_id: Option<i64>,
}

impl DocUploader {
    pub fn new() -> Self {
        Self {
            peer_id: None,
            group_id: None,
        }
    }

    pub fn with_peer_id(mut self, peer_id: i64) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    pub fn with_group_id(mut self, group_id: i64) -> Self {
        self.group_id = Some(group_id);
        self
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let mut params = HashMap::new();
        if let Some(peer_id) = self.peer_id {
            params.insert("peer_id".to_string(), peer_id.to_string());
        }
        if let Some(group_id) = self.group_id {
            params.insert("group_id".to_string(), group_id.to_string());
        }

        let server = api.request("docs.getUploadServer", &params).await?;
        let upload_url = server
            .get("upload_url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| {
                crate::exception::VkError::Validation("Missing upload_url".to_string())
            })?;

        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file.bin");

        let bytes = BaseUploader::read_file(file_path).await?;
        let uploader =
            BaseUploader::upload_multipart(upload_url, "file", filename, bytes).await?;

        let mut save_params = HashMap::new();
        if let Some(gid) = self.group_id {
            save_params.insert("group_id".to_string(), gid.to_string());
        }
        if let Some(obj) = uploader.as_object() {
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

impl Default for DocUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for DocUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        let doc = self.raw_upload(api, file_path).await?;
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

/// Upload document to messages
pub struct DocMessagesUploader {
    peer_id: Option<i64>,
}

impl DocMessagesUploader {
    pub fn new() -> Self {
        Self { peer_id: None }
    }

    pub fn with_peer_id(mut self, peer_id: i64) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("type".to_string(), "doc".to_string());
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
            .unwrap_or("file.bin");

        let bytes = BaseUploader::read_file(file_path).await?;
        let uploader = BaseUploader::upload_multipart(upload_url, "file", filename, bytes).await?;

        let mut save_params = HashMap::new();
        if let Some(obj) = uploader.as_object() {
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

impl Default for DocMessagesUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for DocMessagesUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        let doc = self.raw_upload(api, file_path).await?;
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

/// Upload document to wall
pub struct DocWallUploader {
    group_id: Option<i64>,
}

impl DocWallUploader {
    pub fn new() -> Self {
        Self { group_id: None }
    }

    pub fn with_group_id(mut self, group_id: i64) -> Self {
        self.group_id = Some(group_id);
        self
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let mut params = HashMap::new();
        if let Some(group_id) = self.group_id {
            params.insert("group_id".to_string(), group_id.to_string());
        }

        let server = api.request("docs.getWallUploadServer", &params).await?;
        let upload_url = server
            .get("upload_url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| {
                crate::exception::VkError::Validation("Missing upload_url".to_string())
            })?;

        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file.bin");

        let bytes = BaseUploader::read_file(file_path).await?;
        let uploader = BaseUploader::upload_multipart(upload_url, "file", filename, bytes).await?;

        let mut save_params = HashMap::new();
        if let Some(gid) = self.group_id {
            save_params.insert("group_id".to_string(), gid.to_string());
        }
        if let Some(obj) = uploader.as_object() {
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

impl Default for DocWallUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for DocWallUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        let doc = self.raw_upload(api, file_path).await?;
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
