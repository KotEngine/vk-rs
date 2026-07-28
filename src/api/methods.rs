//! Typed helpers for common VK API methods

use std::collections::HashMap;

use serde_json::Value;

use super::{Api, VkApi};
use crate::exception::VkResult;
use crate::tools::utils::random_id;

impl Api {
    pub async fn messages_send(
        &self,
        peer_id: i64,
        message: &str,
        keyboard: Option<&str>,
        attachment: Option<&str>,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert("message".to_string(), message.to_string());
        params.insert("random_id".to_string(), random_id().to_string());
        if let Some(kb) = keyboard {
            params.insert("keyboard".to_string(), kb.to_string());
        }
        if let Some(att) = attachment {
            params.insert("attachment".to_string(), att.to_string());
        }
        self.request("messages.send", &params).await
    }

    pub async fn messages_edit(
        &self,
        peer_id: i64,
        message_id: i64,
        message: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert("message_id".to_string(), message_id.to_string());
        params.insert("message".to_string(), message.to_string());
        self.request("messages.edit", &params).await
    }

    pub async fn messages_delete(
        &self,
        peer_id: i64,
        message_ids: &[i64],
        delete_for_all: bool,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert(
            "message_ids".to_string(),
            message_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        if delete_for_all {
            params.insert("delete_for_all".to_string(), "1".to_string());
        }
        self.request("messages.delete", &params).await
    }

    pub async fn messages_get_by_id(&self, message_ids: &str) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("message_ids".to_string(), message_ids.to_string());
        self.request("messages.getById", &params).await
    }

    pub async fn messages_get_conversation_members(
        &self,
        peer_id: i64,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        self.request("messages.getConversationMembers", &params).await
    }

    pub async fn messages_set_activity(
        &self,
        peer_id: i64,
        activity_type: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert("type".to_string(), activity_type.to_string());
        self.request("messages.setActivity", &params).await
    }

    pub async fn users_get(&self, user_ids: &[i64], fields: Option<&str>) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert(
            "user_ids".to_string(),
            user_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        if let Some(f) = fields {
            params.insert("fields".to_string(), f.to_string());
        }
        self.request("users.get", &params).await
    }

    pub async fn groups_get_by_id(&self, group_ids: &[i64]) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert(
            "group_ids".to_string(),
            group_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        self.request("groups.getById", &params).await
    }

    pub async fn wall_post(
        &self,
        owner_id: i64,
        message: &str,
        attachments: Option<&str>,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("message".to_string(), message.to_string());
        if let Some(att) = attachments {
            params.insert("attachments".to_string(), att.to_string());
        }
        self.request("wall.post", &params).await
    }

    pub async fn photos_get_wall_upload_server(
        &self,
        group_id: Option<i64>,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        if let Some(gid) = group_id {
            params.insert("group_id".to_string(), gid.to_string());
        }
        self.request("photos.getWallUploadServer", &params).await
    }

    pub async fn docs_get_messages_upload_server(
        &self,
        peer_id: i64,
        doc_type: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert("type".to_string(), doc_type.to_string());
        self.request("docs.getMessagesUploadServer", &params).await
    }

    pub async fn utils_get_short_link(&self, url: &str) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("url".to_string(), url.to_string());
        self.request("utils.getShortLink", &params).await
    }

    pub async fn storage_get(&self, keys: &[&str]) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("keys".to_string(), keys.join(","));
        self.request("storage.get", &params).await
    }

    pub async fn storage_set(&self, key: &str, value: &str) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("key".to_string(), key.to_string());
        params.insert("value".to_string(), value.to_string());
        self.request("storage.set", &params).await
    }

    pub async fn messages_mark_as_read(
        &self,
        peer_id: i64,
        start_message_id: Option<i64>,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        if let Some(id) = start_message_id {
            params.insert("start_message_id".to_string(), id.to_string());
        }
        self.request("messages.markAsRead", &params).await
    }

    pub async fn messages_get_history(
        &self,
        peer_id: i64,
        count: u32,
        offset: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert("count".to_string(), count.to_string());
        params.insert("offset".to_string(), offset.to_string());
        self.request("messages.getHistory", &params).await
    }

    pub async fn messages_search(
        &self,
        peer_id: i64,
        query: &str,
        count: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert("q".to_string(), query.to_string());
        params.insert("count".to_string(), count.to_string());
        self.request("messages.search", &params).await
    }

    pub async fn messages_pin(&self, peer_id: i64, message_id: i64) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        params.insert("message_id".to_string(), message_id.to_string());
        self.request("messages.pin", &params).await
    }

    pub async fn messages_unpin(&self, peer_id: i64) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        self.request("messages.unpin", &params).await
    }

    pub async fn photos_save_wall_photo(
        &self,
        group_id: Option<i64>,
        photo: &str,
        server: i64,
        hash: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("photo".to_string(), photo.to_string());
        params.insert("server".to_string(), server.to_string());
        params.insert("hash".to_string(), hash.to_string());
        if let Some(gid) = group_id {
            params.insert("group_id".to_string(), gid.to_string());
        }
        self.request("photos.saveWallPhoto", &params).await
    }

    pub async fn docs_save(
        &self,
        file: &str,
        title: Option<&str>,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("file".to_string(), file.to_string());
        if let Some(t) = title {
            params.insert("title".to_string(), t.to_string());
        }
        self.request("docs.save", &params).await
    }

    pub async fn groups_get(&self, user_id: i64, extended: bool) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("user_id".to_string(), user_id.to_string());
        if extended {
            params.insert("extended".to_string(), "1".to_string());
        }
        self.request("groups.get", &params).await
    }

    pub async fn friends_get(
        &self,
        user_id: Option<i64>,
        count: u32,
        fields: Option<&str>,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        if let Some(uid) = user_id {
            params.insert("user_id".to_string(), uid.to_string());
        }
        params.insert("count".to_string(), count.to_string());
        if let Some(f) = fields {
            params.insert("fields".to_string(), f.to_string());
        }
        self.request("friends.get", &params).await
    }

    pub async fn account_get_profile_info(&self) -> VkResult<Value> {
        self.request("account.getProfileInfo", &HashMap::new()).await
    }

    pub async fn apps_get(&self, app_id: i64) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("app_id".to_string(), app_id.to_string());
        self.request("apps.get", &params).await
    }
}
