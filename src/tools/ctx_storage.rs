//! Global typed context storage (TypeMap).
//!
//! Keys are types themselves, so the compiler guarantees that the type stored
//! under a given slot matches what is retrieved — no string keys, no runtime
//! downcast surprises. Use it to share long-lived dependencies (database pools,
//! HTTP clients, config) across handlers.
//!
//! ```
//! use vkontakte::tools::CtxStorage;
//! use std::sync::Arc;
//!
//! struct Database { url: String }
//!
//! let ctx = CtxStorage::new();
//! ctx.insert(Database { url: "postgres://".into() });
//!
//! let db = ctx.get::<Database>().unwrap();
//! assert_eq!(db.url, "postgres://");
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Type-keyed global storage accessible anywhere in the bot.
///
/// Each type `T: Send + Sync + 'static` gets its own slot. This removes the
/// class of bugs where a string key is misspelled or holds a value of the
/// wrong type.
pub struct CtxStorage {
    inner: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl CtxStorage {
    /// Create an empty storage.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Insert (or replace) a value keyed by its own type.
    ///
    /// ```
    /// use vkontakte::tools::CtxStorage;
    /// let ctx = CtxStorage::new();
    /// ctx.insert(42i32);
    /// ctx.insert(7i32); // overwrites the previous i32
    /// assert_eq!(*ctx.get::<i32>().unwrap(), 7);
    /// ```
    pub fn insert<T: Send + Sync + 'static>(&self, value: T) {
        let mut inner = self.inner.write().expect("CtxStorage poisoned");
        inner.insert(TypeId::of::<T>(), Arc::new(value));
    }

    /// Insert an already-`Arc`'d value, avoiding a reference-count bump.
    pub fn insert_arc<T: Send + Sync + 'static>(&self, value: Arc<T>) {
        let mut inner = self.inner.write().expect("CtxStorage poisoned");
        inner.insert(TypeId::of::<T>(), value);
    }

    /// Get a shared reference to the value of type `T`, if registered.
    ///
    /// Returns `Arc<T>` so callers do not need `T: Clone` — useful for heavy
    /// resources like connection pools.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let inner = self.inner.read().ok()?;
        inner
            .get(&TypeId::of::<T>())
            .and_then(|any| Arc::clone(any).downcast::<T>().ok())
    }

    /// Remove and return the value of type `T`.
    pub fn remove<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let mut inner = self.inner.write().ok()?;
        inner
            .remove(&TypeId::of::<T>())
            .and_then(|any| Arc::downcast::<T>(any).ok())
    }

    /// Returns `true` if a value of type `T` is currently registered.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.inner
            .read()
            .map(|inner| inner.contains_key(&TypeId::of::<T>()))
            .unwrap_or(false)
    }

    /// Number of registered values.
    pub fn len(&self) -> usize {
        self.inner.read().map(|inner| inner.len()).unwrap_or(0)
    }

    /// Returns `true` if no values are registered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove every registered value.
    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.clear();
        }
    }
}

impl Default for CtxStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CtxStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.len();
        f.debug_struct("CtxStorage").field("entries", &count).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Database {
        url: String,
    }

    #[test]
    fn insert_and_get_by_type() {
        let ctx = CtxStorage::new();
        ctx.insert(Database {
            url: "postgres://db".into(),
        });
        ctx.insert(42i32);

        let db = ctx.get::<Database>().expect("Database registered");
        assert_eq!(db.url, "postgres://db");
        assert_eq!(*ctx.get::<i32>().unwrap(), 42);
    }

    #[test]
    fn different_types_coexist() {
        let ctx = CtxStorage::new();
        ctx.insert(1i32);
        ctx.insert(2u64);
        ctx.insert("three".to_string());

        assert_eq!(*ctx.get::<i32>().unwrap(), 1);
        assert_eq!(*ctx.get::<u64>().unwrap(), 2);
        assert_eq!(ctx.get::<String>().unwrap().as_str(), "three");
    }

    #[test]
    fn overwrite_replaces_value() {
        let ctx = CtxStorage::new();
        ctx.insert(10i32);
        ctx.insert(20i32);
        assert_eq!(*ctx.get::<i32>().unwrap(), 20);
    }

    #[test]
    fn missing_type_returns_none() {
        let ctx = CtxStorage::new();
        assert!(ctx.get::<i32>().is_none());
    }

    #[test]
    fn remove_returns_value() {
        let ctx = CtxStorage::new();
        ctx.insert(5i32);
        assert_eq!(*ctx.remove::<i32>().unwrap(), 5);
        assert!(ctx.get::<i32>().is_none());
    }

    #[test]
    fn contains_and_len() {
        let ctx = CtxStorage::new();
        assert!(ctx.is_empty());
        ctx.insert(1i32);
        assert!(ctx.contains::<i32>());
        assert!(!ctx.contains::<u64>());
        assert_eq!(ctx.len(), 1);
    }

    #[test]
    fn arc_insert_avoids_clone() {
        let ctx = CtxStorage::new();
        let original = Arc::new(vec![1, 2, 3]);
        ctx.insert_arc(original.clone());
        // Same allocation — Arc::ptr_eq holds.
        let got = ctx.get::<Vec<i32>>().unwrap();
        assert!(Arc::ptr_eq(&got, &original));
    }

    #[test]
    fn clear_removes_all() {
        let ctx = CtxStorage::new();
        ctx.insert(1i32);
        ctx.insert("x".to_string());
        ctx.clear();
        assert!(ctx.is_empty());
    }
}
