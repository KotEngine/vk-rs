//! Fixed-capacity LRU-style map

use std::collections::HashMap;
use std::hash::Hash;

/// Map that evicts oldest entries when capacity is exceeded
pub struct LimitedDict<K, V> {
    capacity: usize,
    order: Vec<K>,
    data: HashMap<K, V>,
}

impl<K, V> LimitedDict<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: Vec::new(),
            data: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.data.get_mut(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if !self.data.contains_key(&key) {
            self.order.push(key.clone());
        }
        let old = self.data.insert(key, value);
        while self.order.len() > self.capacity {
            if let Some(old_key) = self.order.first().cloned() {
                self.order.remove(0);
                self.data.remove(&old_key);
            }
        }
        old
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.order.retain(|k| k != key);
        self.data.remove(key)
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.data.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_oldest() {
        let mut map = LimitedDict::new(2);
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);
        map.insert("c".to_string(), 3);
        assert!(map.get(&"a".to_string()).is_none());
        assert_eq!(map.get(&"c".to_string()), Some(&3));
    }
}
