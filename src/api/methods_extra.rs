//! Additional VK API method helpers (wall, polls, stories, market)

use std::collections::HashMap;

use serde_json::Value;

use super::{Api, VkApi};
use crate::exception::VkResult;

impl Api {
    pub async fn wall_get(
        &self,
        owner_id: i64,
        count: u32,
        offset: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("count".to_string(), count.to_string());
        params.insert("offset".to_string(), offset.to_string());
        self.request("wall.get", &params).await
    }

    pub async fn wall_edit(
        &self,
        owner_id: i64,
        post_id: i64,
        message: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("post_id".to_string(), post_id.to_string());
        params.insert("message".to_string(), message.to_string());
        self.request("wall.edit", &params).await
    }

    pub async fn wall_delete(&self, owner_id: i64, post_id: i64) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("post_id".to_string(), post_id.to_string());
        self.request("wall.delete", &params).await
    }

    pub async fn wall_create_comment(
        &self,
        owner_id: i64,
        post_id: i64,
        message: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("post_id".to_string(), post_id.to_string());
        params.insert("message".to_string(), message.to_string());
        self.request("wall.createComment", &params).await
    }

    pub async fn polls_create(
        &self,
        question: &str,
        answers: &[&str],
        is_anonymous: bool,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("question".to_string(), question.to_string());
        params.insert("add_answers".to_string(), answers.join(","));
        if is_anonymous {
            params.insert("is_anonymous".to_string(), "1".to_string());
        }
        self.request("polls.create", &params).await
    }

    pub async fn polls_get_voters(
        &self,
        owner_id: i64,
        poll_id: i64,
        answer_ids: &[i64],
        count: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("poll_id".to_string(), poll_id.to_string());
        params.insert(
            "answer_ids".to_string(),
            answer_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        params.insert("count".to_string(), count.to_string());
        self.request("polls.getVoters", &params).await
    }

    pub async fn stories_get(&self, owner_id: i64, extended: bool) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        if extended {
            params.insert("extended".to_string(), "1".to_string());
        }
        self.request("stories.get", &params).await
    }

    pub async fn stories_send_interaction(
        &self,
        owner_id: i64,
        story_id: i64,
        access_key: &str,
        message: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("story_id".to_string(), story_id.to_string());
        params.insert("access_key".to_string(), access_key.to_string());
        params.insert("message".to_string(), message.to_string());
        self.request("stories.sendInteraction", &params).await
    }

    pub async fn market_get(
        &self,
        owner_id: i64,
        count: u32,
        offset: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("count".to_string(), count.to_string());
        params.insert("offset".to_string(), offset.to_string());
        self.request("market.get", &params).await
    }

    pub async fn market_add(
        &self,
        owner_id: i64,
        name: &str,
        description: &str,
        category_id: i64,
        price: f64,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("name".to_string(), name.to_string());
        params.insert("description".to_string(), description.to_string());
        params.insert("category_id".to_string(), category_id.to_string());
        params.insert("price".to_string(), price.to_string());
        self.request("market.add", &params).await
    }

    pub async fn board_get_topics(
        &self,
        group_id: i64,
        count: u32,
        offset: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("group_id".to_string(), group_id.to_string());
        params.insert("count".to_string(), count.to_string());
        params.insert("offset".to_string(), offset.to_string());
        self.request("board.getTopics", &params).await
    }

    pub async fn board_add_topic(
        &self,
        group_id: i64,
        title: &str,
        text: &str,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("group_id".to_string(), group_id.to_string());
        params.insert("title".to_string(), title.to_string());
        params.insert("text".to_string(), text.to_string());
        self.request("board.addTopic", &params).await
    }

    pub async fn likes_add(
        &self,
        item_type: &str,
        owner_id: i64,
        item_id: i64,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("type".to_string(), item_type.to_string());
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("item_id".to_string(), item_id.to_string());
        self.request("likes.add", &params).await
    }

    pub async fn likes_delete(
        &self,
        item_type: &str,
        owner_id: i64,
        item_id: i64,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("type".to_string(), item_type.to_string());
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("item_id".to_string(), item_id.to_string());
        self.request("likes.delete", &params).await
    }

    pub async fn photos_get(
        &self,
        owner_id: i64,
        album_id: &str,
        count: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("album_id".to_string(), album_id.to_string());
        params.insert("count".to_string(), count.to_string());
        self.request("photos.get", &params).await
    }

    pub async fn photos_get_albums(&self, owner_id: i64) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        self.request("photos.getAlbums", &params).await
    }

    pub async fn video_get(
        &self,
        owner_id: i64,
        count: u32,
        offset: u32,
    ) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("count".to_string(), count.to_string());
        params.insert("offset".to_string(), offset.to_string());
        self.request("video.get", &params).await
    }

    pub async fn audio_get(&self, owner_id: i64, count: u32) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("count".to_string(), count.to_string());
        self.request("audio.get", &params).await
    }

    pub async fn docs_get(&self, owner_id: i64, count: u32) -> VkResult<Value> {
        let mut params = HashMap::new();
        params.insert("owner_id".to_string(), owner_id.to_string());
        params.insert("count".to_string(), count.to_string());
        self.request("docs.get", &params).await
    }
}
