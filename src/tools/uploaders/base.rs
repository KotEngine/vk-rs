//! Base uploader utilities

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;

/// Base file uploader trait
#[async_trait]
pub trait Uploader: Send + Sync {
    /// Upload file and return VK attachment string
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String>;

    /// Attachment type prefix (photo, doc, video, ...)
    fn attachment_type(&self) -> &str;
}

/// Shared uploader utilities
pub struct BaseUploader;

impl BaseUploader {
    pub fn generate_attachment_string(
        attachment_type: &str,
        owner_id: i64,
        item_id: i64,
        access_key: Option<&str>,
    ) -> String {
        match access_key {
            Some(key) => format!("{attachment_type}{owner_id}_{item_id}_{key}"),
            None => format!("{attachment_type}{owner_id}_{item_id}"),
        }
    }

    pub async fn read_file(path: &str) -> VkResult<Vec<u8>> {
        tokio::fs::read(path)
            .await
            .map_err(crate::exception::VkError::Io)
    }

    pub async fn upload_multipart(
        upload_url: &str,
        field_name: &str,
        filename: &str,
        bytes: Vec<u8>,
    ) -> VkResult<Value> {
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| crate::exception::VkError::Validation(e.to_string()))?;

        let form = reqwest::multipart::Form::new().part(field_name.to_string(), part);

        let client = reqwest::Client::new();
        let response = client
            .post(upload_url)
            .multipart(form)
            .send()
            .await
            .map_err(crate::exception::VkError::Http)?;

        let text = response
            .text()
            .await
            .map_err(crate::exception::VkError::Http)?;

        serde_json::from_str(&text).map_err(crate::exception::VkError::Json)
    }

    pub async fn get_owner_id(api: &Api, params: &HashMap<String, String>) -> VkResult<i64> {
        if let Some(gid) = params
            .get("group_id")
            .and_then(|g| g.parse::<i64>().ok())
        {
            return Ok(-gid);
        }
        if let Some(uid) = params.get("user_id").and_then(|u| u.parse::<i64>().ok()) {
            return Ok(uid);
        }
        if let Some(oid) = params.get("owner_id").and_then(|o| o.parse::<i64>().ok()) {
            return Ok(oid);
        }

        if let Ok(resp) = api.request("groups.getById", &HashMap::new()).await {
            if let Some(id) = resp
                .get("groups")
                .and_then(|g| g.as_array())
                .and_then(|a| a.first())
                .and_then(|g| g.get("id"))
                .and_then(|id| id.as_i64())
            {
                return Ok(-id);
            }
        }

        let resp = api.request("users.get", &HashMap::new()).await?;
        Ok(resp
            .get(0)
            .and_then(|u| u.get("id"))
            .and_then(|id| id.as_i64())
            .unwrap_or(0))
    }
}
