//! Photo uploaders

use std::collections::HashMap;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use super::base::{BaseUploader, Uploader};

/// Upload photo to messages (most common for bots)
pub struct PhotoMessageUploader {
    peer_id: Option<i64>,
}

impl PhotoMessageUploader {
    pub fn new() -> Self {
        Self { peer_id: None }
    }

    pub fn with_peer_id(mut self, peer_id: i64) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let mut params = HashMap::new();
        if let Some(peer_id) = self.peer_id {
            params.insert("peer_id".to_string(), peer_id.to_string());
        }

        let server = api.request("photos.getMessagesUploadServer", &params).await?;
        let uploaded = upload_photo_server(api, &server, file_path, "photo", "picture.jpg").await?;

        let mut save_params = HashMap::new();
        if let Some(obj) = uploaded.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    save_params.insert(k.clone(), s.to_string());
                } else if let Some(n) = v.as_i64() {
                    save_params.insert(k.clone(), n.to_string());
                }
            }
        }

        let saved = api.request("photos.saveMessagesPhoto", &save_params).await?;
        Ok(saved.get(0).cloned().unwrap_or(saved))
    }
}

impl Default for PhotoMessageUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for PhotoMessageUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        photo_attachment_from_raw(self.raw_upload(api, file_path).await?)
    }

    fn attachment_type(&self) -> &str {
        "photo"
    }
}

/// Upload photo to wall
pub struct PhotoWallUploader {
    group_id: Option<i64>,
}

impl PhotoWallUploader {
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

        let server = api.request("photos.getWallUploadServer", &params).await?;
        let uploaded = upload_photo_server(api, &server, file_path, "photo", "picture.jpg").await?;

        let mut save_params = HashMap::new();
        if let Some(gid) = self.group_id {
            save_params.insert("group_id".to_string(), gid.to_string());
        }
        if let Some(obj) = uploaded.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    save_params.insert(k.clone(), s.to_string());
                }
            }
        }

        let saved = api.request("photos.saveWallPhoto", &save_params).await?;
        Ok(saved.get(0).cloned().unwrap_or(saved))
    }
}

impl Default for PhotoWallUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for PhotoWallUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        photo_attachment_from_raw(self.raw_upload(api, file_path).await?)
    }

    fn attachment_type(&self) -> &str {
        "photo"
    }
}

/// Upload owner profile photo
pub struct PhotoFaviconUploader;

impl PhotoFaviconUploader {
    pub fn new() -> Self {
        Self
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let server = api
            .request("photos.getOwnerPhotoUploadServer", &HashMap::new())
            .await?;
        upload_photo_server(api, &server, file_path, "photo", "picture.jpg").await
    }
}

impl Default for PhotoFaviconUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for PhotoFaviconUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        photo_attachment_from_raw(self.raw_upload(api, file_path).await?)
    }

    fn attachment_type(&self) -> &str {
        "photo"
    }
}

/// Upload market photo
pub struct PhotoMarketUploader {
    group_id: Option<i64>,
}

impl PhotoMarketUploader {
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
        let server = api.request("photos.getMarketUploadServer", &params).await?;
        let uploaded = upload_photo_server(api, &server, file_path, "file", "picture.jpg").await?;

        let mut save_params = HashMap::new();
        if let Some(gid) = self.group_id {
            save_params.insert("group_id".to_string(), gid.to_string());
        }
        if let Some(obj) = uploaded.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    save_params.insert(k.clone(), s.to_string());
                }
            }
        }

        let saved = api.request("photos.saveMarketPhoto", &save_params).await?;
        Ok(saved.get(0).cloned().unwrap_or(saved))
    }
}

impl Default for PhotoMarketUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Uploader for PhotoMarketUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        photo_attachment_from_raw(self.raw_upload(api, file_path).await?)
    }

    fn attachment_type(&self) -> &str {
        "photo"
    }
}

/// Upload photo to album
pub struct PhotoToAlbumUploader {
    album_id: i64,
}

impl PhotoToAlbumUploader {
    pub fn new(album_id: i64) -> Self {
        Self { album_id }
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("album_id".to_string(), self.album_id.to_string());
        let server = api.request("photos.getUploadServer", &params).await?;
        let uploaded = upload_photo_server(api, &server, file_path, "file1", "picture.jpg").await?;

        let mut save_params = HashMap::new();
        save_params.insert("album_id".to_string(), self.album_id.to_string());
        if let Some(obj) = uploaded.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    save_params.insert(k.clone(), s.to_string());
                }
            }
        }

        let saved = api.request("photos.save", &save_params).await?;
        Ok(saved.get(0).cloned().unwrap_or(saved))
    }
}

#[async_trait::async_trait]
impl Uploader for PhotoToAlbumUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        photo_attachment_from_raw(self.raw_upload(api, file_path).await?)
    }

    fn attachment_type(&self) -> &str {
        "photo"
    }
}

/// Upload chat photo
pub struct PhotoChatFaviconUploader {
    chat_id: i64,
}

impl PhotoChatFaviconUploader {
    pub fn new(chat_id: i64) -> Self {
        Self { chat_id }
    }

    pub async fn raw_upload(&self, api: &Api, file_path: &str) -> VkResult<serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("chat_id".to_string(), self.chat_id.to_string());
        let server = api.request("photos.getChatUploadServer", &params).await?;
        let uploaded = upload_photo_server(api, &server, file_path, "file", "picture.jpg").await?;

        let mut save_params = HashMap::new();
        save_params.insert("chat_id".to_string(), self.chat_id.to_string());
        if let Some(obj) = uploaded.as_object() {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    save_params.insert(k.clone(), s.to_string());
                }
            }
        }

        let saved = api.request("photos.saveChatPhoto", &save_params).await?;
        Ok(saved.get(0).cloned().unwrap_or(saved))
    }
}

#[async_trait::async_trait]
impl Uploader for PhotoChatFaviconUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        photo_attachment_from_raw(self.raw_upload(api, file_path).await?)
    }

    fn attachment_type(&self) -> &str {
        "photo"
    }
}

async fn upload_photo_server(
    _api: &Api,
    server: &serde_json::Value,
    file_path: &str,
    field_name: &str,
    default_filename: &str,
) -> VkResult<serde_json::Value> {
    let upload_url = server
        .get("upload_url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| crate::exception::VkError::Validation("Missing upload_url".to_string()))?;

    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(default_filename);

    let bytes = BaseUploader::read_file(file_path).await?;
    BaseUploader::upload_multipart(upload_url, field_name, filename, bytes).await
}

fn photo_attachment_from_raw(photo: serde_json::Value) -> VkResult<String> {
    let owner_id = photo.get("owner_id").and_then(|v| v.as_i64()).unwrap_or(0);
    let id = photo.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let access_key = photo.get("access_key").and_then(|v| v.as_str());
    Ok(BaseUploader::generate_attachment_string(
        "photo",
        owner_id,
        id,
        access_key,
    ))
}
