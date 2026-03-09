use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

struct Entry<V> {
    value: V,
    expires_at: Option<Instant>,
    tags: HashSet<String>,
}

/// A thread-safe in-memory LRU cache with TTL and tag-based invalidation.
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone,
{
    inner: Arc<RwLock<CacheInner<K, V>>>,
}

struct CacheInner<K, V>
where
    K: Eq + Hash + Clone,
{
    items: HashMap<K, Entry<V>>,
    order: VecDeque<K>,
    max_size: usize,
    default_ttl: Option<Duration>,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Create a new cache with the given max size and optional default TTL.
    pub fn new(max_size: usize, default_ttl: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                items: HashMap::with_capacity(max_size),
                order: VecDeque::with_capacity(max_size),
                max_size,
                default_ttl,
            })),
        }
    }

    /// Set a value with optional TTL and tags.
    pub fn set(&self, key: K, value: V) {
        self.set_with(key, value, None, &[]);
    }

    /// Set a value with custom TTL and tags.
    pub fn set_with(&self, key: K, value: V, ttl: Option<Duration>, tags: &[&str]) {
        let mut inner = self.inner.write().unwrap();
        let effective_ttl = ttl.or(inner.default_ttl);
        let expires_at = effective_ttl.map(|d| Instant::now() + d);
        let tag_set: HashSet<String> = tags.iter().map(|s| s.to_string()).collect();

        if inner.items.contains_key(&key) {
            inner.order.retain(|k| k != &key);
        } else if inner.items.len() >= inner.max_size {
            // Evict: prefer expired, then LRU
            let mut evicted = false;
            let now = Instant::now();
            let expired_key = inner.order.iter().find(|k| {
                inner.items.get(*k).map_or(false, |e| e.expires_at.map_or(false, |t| now > t))
            }).cloned();

            if let Some(ek) = expired_key {
                inner.items.remove(&ek);
                inner.order.retain(|k| k != &ek);
                evicted = true;
            }

            if !evicted {
                if let Some(lru_key) = inner.order.pop_back() {
                    inner.items.remove(&lru_key);
                }
            }
        }

        inner.items.insert(key.clone(), Entry { value, expires_at, tags: tag_set });
        inner.order.push_front(key);
    }

    /// Get a value from the cache. Returns None if not found or expired.
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let mut inner = self.inner.write().unwrap();
        let entry = inner.items.get(key)?;

        if let Some(expires_at) = entry.expires_at {
            if Instant::now() > expires_at {
                inner.items.remove(key);
                inner.order.retain(|k| k != key);
                return None;
            }
        }

        let value = entry.value.clone();
        inner.order.retain(|k| k != key);
        inner.order.push_front(key.clone());
        Some(value)
    }

    /// Check if a key exists and is not expired.
    pub fn has(&self, key: &K) -> bool
    where
        V: Clone,
    {
        self.get(key).is_some()
    }

    /// Delete an entry by key.
    pub fn delete(&self, key: &K) -> bool {
        let mut inner = self.inner.write().unwrap();
        if inner.items.remove(key).is_some() {
            inner.order.retain(|k| k != key);
            true
        } else {
            false
        }
    }

    /// Invalidate all entries with the given tag. Returns count removed.
    pub fn invalidate_by_tag(&self, tag: &str) -> usize {
        let mut inner = self.inner.write().unwrap();
        let keys: Vec<K> = inner.items.iter()
            .filter(|(_, v)| v.tags.contains(tag))
            .map(|(k, _)| k.clone())
            .collect();
        let count = keys.len();
        for key in &keys {
            inner.items.remove(key);
        }
        inner.order.retain(|k| !keys.contains(k));
        count
    }

    /// Remove all entries.
    pub fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.items.clear();
        inner.order.clear();
    }

    /// Return the number of entries.
    pub fn size(&self) -> usize {
        self.inner.read().unwrap().items.len()
    }
}

impl<K, V> Clone for Cache<K, V>
where
    K: Eq + Hash + Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
