//! Cache for `messages.getConversationMembers` responses

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde_json::Value;

use crate::api::{Api, VkApi};
use crate::exception::VkResult;

struct CacheEntry {
    data: Value,
    fetched_at: Instant,
}

/// TTL cache for conversation members per peer
pub struct ConversationMembersCache {
    ttl: Duration,
    store: DashMap<i64, CacheEntry>,
}

impl ConversationMembersCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            store: DashMap::new(),
        }
    }

    pub fn with_default_ttl() -> Self {
        Self::new(Duration::from_secs(60))
    }

    pub async fn get_members(&self, api: &Api, peer_id: i64) -> VkResult<Value> {
        if let Some(entry) = self.store.get(&peer_id) {
            if entry.fetched_at.elapsed() < self.ttl {
                return Ok(entry.data.clone());
            }
        }

        let mut params = std::collections::HashMap::new();
        params.insert("peer_id".to_string(), peer_id.to_string());
        let data = api
            .request("messages.getConversationMembers", &params)
            .await?;

        self.store.insert(
            peer_id,
            CacheEntry {
                data: data.clone(),
                fetched_at: Instant::now(),
            },
        );

        Ok(data)
    }

    pub fn invalidate(&self, peer_id: i64) {
        self.store.remove(&peer_id);
    }

    pub fn clear(&self) {
        self.store.clear();
    }

    pub async fn member_ids(&self, api: &Api, peer_id: i64) -> VkResult<Vec<i64>> {
        let data = self.get_members(api, peer_id).await?;
        let ids = data
            .get("items")
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("member_id").and_then(|id| id.as_i64()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(ids)
    }

    pub async fn is_admin(
        &self,
        api: &Api,
        peer_id: i64,
        user_id: i64,
    ) -> VkResult<bool> {
        let data = self.get_members(api, peer_id).await?;
        if let Some(items) = data.get("items").and_then(|i| i.as_array()) {
            for member in items {
                let member_id = member.get("member_id").and_then(|m| m.as_i64());
                let is_admin = member
                    .get("is_admin")
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false)
                    || member
                        .get("is_owner")
                        .and_then(|o| o.as_bool())
                        .unwrap_or(false);
                if member_id == Some(user_id) && is_admin {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// Shared global cache (optional singleton for bots)
pub type SharedConversationCache = Arc<ConversationMembersCache>;

pub fn shared_conversation_cache() -> SharedConversationCache {
    Arc::new(ConversationMembersCache::with_default_ttl())
}
