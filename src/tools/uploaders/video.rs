//! Video uploader

use std::collections::HashMap;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;
use super::base::{BaseUploader, Uploader};

/// Upload video to wall or messages
pub struct VideoUploader {
    group_id: Option<i64>,
    wallpost: bool,
}

impl VideoUploader {
    pub fn new() -> Self {
        Self {
            group_id: None,
            wallpost: false,
        }
    }

    pub fn with_group_id(mut self, group_id: i64) -> Self {
        self.group_id = Some(group_id);
        self
    }

    pub fn wallpost(mut self, wallpost: bool) -> Self {
        self.wallpost = wallpost;
        self
    }
}

#[async_trait::async_trait]
impl Uploader for VideoUploader {
    async fn upload(&self, api: &Api, file_path: &str) -> VkResult<String> {
        let mut params = HashMap::new();
        if let Some(gid) = self.group_id {
            params.insert("group_id".to_string(), gid.to_string());
        }
        if self.wallpost {
            params.insert("wallpost".to_string(), "1".to_string());
        }

        let server = api.request("video.save", &params).await?;
        let upload_url = server
            .get("upload_url")
            .and_then(|u| u.as_str())
            .ok_or_else(|| {
                crate::exception::VkError::Validation("Missing upload_url".to_string())
            })?;

        let bytes = BaseUploader::read_file(file_path).await?;
        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("video.mp4");

        let uploaded = BaseUploader::upload_multipart(upload_url, "video_file", filename, bytes).await?;

        let owner_id = uploaded
            .get("owner_id")
            .or_else(|| server.get("owner_id"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let video_id = uploaded
            .get("video_id")
            .or_else(|| server.get("video_id"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(BaseUploader::generate_attachment_string("video", owner_id, video_id, None))
    }

    fn attachment_type(&self) -> &str {
        "video"
    }
}
