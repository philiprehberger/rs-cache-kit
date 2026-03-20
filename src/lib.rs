//! Generic LRU cache with TTL, tags, and async support.
//!
//! # Example
//!
//! ```rust
//! use philiprehberger_cache_kit::Cache;
//!
//! let cache: Cache<String, String> = Cache::new(100, None);
//! cache.set("key".into(), "value".into());
//! assert_eq!(cache.get(&"key".into()), Some("value".into()));
//! ```

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Snapshot of cache performance counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheStats {
    /// Number of successful cache hits.
    pub hits: u64,
    /// Number of cache misses (key absent or expired).
    pub misses: u64,
    /// Number of entries evicted (LRU or expired-on-evict).
    pub evictions: u64,
}

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
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
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

impl<K, V> Default for Cache<K, V>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new(100, None)
    }
}

impl<K, V> fmt::Debug for Cache<K, V>
where
    K: Eq + Hash + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.read().unwrap();
        f.debug_struct("Cache")
            .field("size", &inner.items.len())
            .field("max_size", &inner.max_size)
            .field("default_ttl", &inner.default_ttl)
            .finish()
    }
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
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
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
            let expired_key = inner
                .order
                .iter()
                .find(|k| {
                    inner
                        .items
                        .get(*k)
                        .is_some_and(|e| e.expires_at.is_some_and(|t| now > t))
                })
                .cloned();

            if let Some(ek) = expired_key {
                inner.items.remove(&ek);
                inner.order.retain(|k| k != &ek);
                self.evictions.fetch_add(1, Ordering::Relaxed);
                evicted = true;
            }

            if !evicted {
                if let Some(lru_key) = inner.order.pop_back() {
                    inner.items.remove(&lru_key);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        inner.items.insert(
            key.clone(),
            Entry {
                value,
                expires_at,
                tags: tag_set,
            },
        );
        inner.order.push_front(key);
    }

    /// Get a value from the cache. Returns None if not found or expired.
    ///
    /// Increments the hit counter on success, or the miss counter on failure.
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let mut inner = self.inner.write().unwrap();
        let entry = match inner.items.get(key) {
            Some(e) => e,
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        if let Some(expires_at) = entry.expires_at {
            if Instant::now() > expires_at {
                inner.items.remove(key);
                inner.order.retain(|k| k != key);
                self.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        }

        let value = entry.value.clone();
        inner.order.retain(|k| k != key);
        inner.order.push_front(key.clone());
        self.hits.fetch_add(1, Ordering::Relaxed);
        Some(value)
    }

    /// Check if a key exists and is not expired.
    pub fn has(&self, key: &K) -> bool {
        let mut inner = self.inner.write().unwrap();
        let entry = match inner.items.get(key) {
            Some(e) => e,
            None => return false,
        };

        if let Some(expires_at) = entry.expires_at {
            if Instant::now() > expires_at {
                inner.items.remove(key);
                inner.order.retain(|k| k != key);
                return false;
            }
        }

        true
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
        let keys: Vec<K> = inner
            .items
            .iter()
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

    /// Returns true if the cache has no entries.
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().items.is_empty()
    }

    /// Returns the maximum number of entries the cache can hold.
    pub fn max_size(&self) -> usize {
        self.inner.read().unwrap().max_size
    }

    /// Returns a list of all non-expired keys in the cache.
    pub fn keys(&self) -> Vec<K> {
        let inner = self.inner.read().unwrap();
        let now = Instant::now();
        inner
            .items
            .iter()
            .filter(|(_, entry)| {
                entry.expires_at.map_or(true, |t| now <= t)
            })
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Remove all expired entries from the cache. Returns the number of entries removed.
    pub fn remove_expired(&self) -> usize {
        let mut inner = self.inner.write().unwrap();
        let now = Instant::now();
        let expired_keys: Vec<K> = inner
            .items
            .iter()
            .filter(|(_, entry)| entry.expires_at.is_some_and(|t| now > t))
            .map(|(k, _)| k.clone())
            .collect();
        let count = expired_keys.len();
        for key in &expired_keys {
            inner.items.remove(key);
        }
        inner.order.retain(|k| !expired_keys.contains(k));
        count
    }

    /// Get a value from the cache, or insert one computed by the given closure if absent or expired.
    pub fn get_or_insert_with<F>(&self, key: K, f: F) -> V
    where
        V: Clone,
        F: FnOnce() -> V,
    {
        // Try to get first
        if let Some(val) = self.get(&key) {
            return val;
        }
        // Compute and insert
        let value = f();
        self.set(key, value.clone());
        value
    }

    /// Return a snapshot of cache performance counters.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Retrieve multiple values at once. Returns a map of keys to their cached values,
    /// omitting any keys that are absent or expired.
    pub fn get_many(&self, keys: &[K]) -> HashMap<K, V>
    where
        V: Clone,
    {
        let mut result = HashMap::with_capacity(keys.len());
        for key in keys {
            if let Some(val) = self.get(key) {
                result.insert(key.clone(), val);
            }
        }
        result
    }

    /// Delete all entries for which the predicate returns `true`.
    /// Returns the number of entries removed.
    pub fn delete_where<F>(&self, predicate: F) -> usize
    where
        F: Fn(&K, &V) -> bool,
    {
        let mut inner = self.inner.write().unwrap();
        let keys_to_remove: Vec<K> = inner
            .items
            .iter()
            .filter(|(k, entry)| predicate(k, &entry.value))
            .map(|(k, _)| k.clone())
            .collect();
        let count = keys_to_remove.len();
        for key in &keys_to_remove {
            inner.items.remove(key);
        }
        inner.order.retain(|k| !keys_to_remove.contains(k));
        count
    }

    /// Return the number of entries (alias for [`size()`](Self::size)).
    pub fn len(&self) -> usize {
        self.size()
    }
}

impl<K, V> Clone for Cache<K, V>
where
    K: Eq + Hash + Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            hits: Arc::clone(&self.hits),
            misses: Arc::clone(&self.misses),
            evictions: Arc::clone(&self.evictions),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let cache = Cache::new(10, None);
        cache.set("key", "value");
        assert_eq!(cache.get(&"key"), Some("value"));
    }

    #[test]
    fn test_get_missing_key() {
        let cache: Cache<&str, &str> = Cache::new(10, None);
        assert_eq!(cache.get(&"missing"), None);
    }

    #[test]
    fn test_overwrite_value() {
        let cache = Cache::new(10, None);
        cache.set("key", "v1");
        cache.set("key", "v2");
        assert_eq!(cache.get(&"key"), Some("v2"));
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn test_delete() {
        let cache = Cache::new(10, None);
        cache.set("key", "value");
        assert!(cache.delete(&"key"));
        assert_eq!(cache.get(&"key"), None);
        assert!(!cache.delete(&"key"));
    }

    #[test]
    fn test_has() {
        let cache = Cache::new(10, None);
        cache.set("key", "value");
        assert!(cache.has(&"key"));
        assert!(!cache.has(&"missing"));
    }

    #[test]
    fn test_clear() {
        let cache = Cache::new(10, None);
        cache.set("a", 1);
        cache.set("b", 2);
        assert_eq!(cache.size(), 2);
        cache.clear();
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = Cache::new(3, None);
        cache.set("a", 1);
        cache.set("b", 2);
        cache.set("c", 3);
        // "a" is LRU, should be evicted
        cache.set("d", 4);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.size(), 3);
    }

    #[test]
    fn test_lru_access_updates_order() {
        let cache = Cache::new(3, None);
        cache.set("a", 1);
        cache.set("b", 2);
        cache.set("c", 3);
        // Access "a" to make it recently used
        cache.get(&"a");
        // Now "b" is LRU
        cache.set("d", 4);
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), None);
    }

    #[test]
    fn test_ttl_expiration() {
        let cache = Cache::new(10, None);
        cache.set_with("key", "value", Some(Duration::from_millis(1)), &[]);
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get(&"key"), None);
    }

    #[test]
    fn test_has_with_expired_ttl() {
        let cache = Cache::new(10, None);
        cache.set_with("key", "value", Some(Duration::from_millis(1)), &[]);
        std::thread::sleep(Duration::from_millis(10));
        assert!(!cache.has(&"key"));
    }

    #[test]
    fn test_default_ttl() {
        let cache = Cache::new(10, Some(Duration::from_millis(1)));
        cache.set("key", "value");
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get(&"key"), None);
    }

    #[test]
    fn test_tag_invalidation() {
        let cache = Cache::new(10, None);
        cache.set_with("a", 1, None, &["group1"]);
        cache.set_with("b", 2, None, &["group1", "group2"]);
        cache.set_with("c", 3, None, &["group2"]);
        let removed = cache.invalidate_by_tag("group1");
        assert_eq!(removed, 2);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"c"), Some(3));
    }

    #[test]
    fn test_clone_shares_state() {
        let cache = Cache::new(10, None);
        let cache2 = cache.clone();
        cache.set("key", "value");
        assert_eq!(cache2.get(&"key"), Some("value"));
    }

    #[test]
    fn test_debug_impl() {
        let cache: Cache<&str, &str> = Cache::new(10, None);
        let debug = format!("{:?}", cache);
        assert!(debug.contains("Cache"));
        assert!(debug.contains("max_size"));
    }

    #[test]
    fn test_default_impl() {
        let cache: Cache<String, String> = Cache::default();
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_thread_safety() {
        let cache = Cache::new(100, None);
        let mut handles = vec![];

        for i in 0..10 {
            let c = cache.clone();
            handles.push(std::thread::spawn(move || {
                c.set(i, i * 10);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(cache.size(), 10);
    }

    #[test]
    fn test_is_empty() {
        let cache: Cache<&str, &str> = Cache::new(10, None);
        assert!(cache.is_empty());
        cache.set("key", "value");
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_max_size() {
        let cache: Cache<&str, &str> = Cache::new(42, None);
        assert_eq!(cache.max_size(), 42);
    }

    #[test]
    fn test_keys() {
        let cache = Cache::new(10, None);
        cache.set("a", 1);
        cache.set("b", 2);
        let mut keys = cache.keys();
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn test_keys_excludes_expired() {
        let cache = Cache::new(10, None);
        cache.set_with("fresh", 1, None, &[]);
        cache.set_with("expired", 2, Some(Duration::from_millis(1)), &[]);
        std::thread::sleep(Duration::from_millis(10));
        let keys = cache.keys();
        assert_eq!(keys, vec!["fresh"]);
    }

    #[test]
    fn test_remove_expired() {
        let cache = Cache::new(10, None);
        cache.set_with("fresh", 1, None, &[]);
        cache.set_with("stale1", 2, Some(Duration::from_millis(1)), &[]);
        cache.set_with("stale2", 3, Some(Duration::from_millis(1)), &[]);
        std::thread::sleep(Duration::from_millis(10));
        let removed = cache.remove_expired();
        assert_eq!(removed, 2);
        assert_eq!(cache.size(), 1);
        assert!(cache.has(&"fresh"));
    }

    #[test]
    fn test_get_or_insert_with_existing() {
        let cache = Cache::new(10, None);
        cache.set("key", 42);
        let val = cache.get_or_insert_with("key", || 99);
        assert_eq!(val, 42);
    }

    #[test]
    fn test_get_or_insert_with_missing() {
        let cache = Cache::new(10, None);
        let val = cache.get_or_insert_with("key", || 99);
        assert_eq!(val, 99);
        assert_eq!(cache.get(&"key"), Some(99));
    }

    #[test]
    fn test_get_or_insert_with_expired() {
        let cache = Cache::new(10, None);
        cache.set_with("key", 42, Some(Duration::from_millis(1)), &[]);
        std::thread::sleep(Duration::from_millis(10));
        let val = cache.get_or_insert_with("key", || 99);
        assert_eq!(val, 99);
    }

    #[test]
    fn test_stats_hits_and_misses() {
        let cache = Cache::new(10, None);
        cache.set("a", 1);
        cache.set("b", 2);

        // Two hits
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), Some(2));

        // Two misses
        assert_eq!(cache.get(&"c"), None);
        assert_eq!(cache.get(&"d"), None);

        let s = cache.stats();
        assert_eq!(s.hits, 2);
        assert_eq!(s.misses, 2);
        assert_eq!(s.evictions, 0);
    }

    #[test]
    fn test_stats_evictions() {
        let cache = Cache::new(2, None);
        cache.set("a", 1);
        cache.set("b", 2);
        // This triggers eviction of "a"
        cache.set("c", 3);

        let s = cache.stats();
        assert_eq!(s.evictions, 1);
    }

    #[test]
    fn test_stats_miss_on_expired() {
        let cache = Cache::new(10, None);
        cache.set_with("key", 1, Some(Duration::from_millis(1)), &[]);
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get(&"key"), None);

        let s = cache.stats();
        assert_eq!(s.misses, 1);
        assert_eq!(s.hits, 0);
    }

    #[test]
    fn test_get_many() {
        let cache = Cache::new(10, None);
        cache.set("a", 1);
        cache.set("b", 2);
        cache.set("c", 3);

        let result = cache.get_many(&["a", "c", "missing"]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[&"a"], 1);
        assert_eq!(result[&"c"], 3);
    }

    #[test]
    fn test_get_many_empty() {
        let cache: Cache<&str, i32> = Cache::new(10, None);
        let result = cache.get_many(&["a", "b"]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_delete_where() {
        let cache = Cache::new(10, None);
        cache.set("a", 1);
        cache.set("b", 20);
        cache.set("c", 3);
        cache.set("d", 40);

        let removed = cache.delete_where(|_k, v| *v >= 10);
        assert_eq!(removed, 2);
        assert_eq!(cache.size(), 2);
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"c"), Some(3));
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"d"), None);
    }

    #[test]
    fn test_delete_where_none_match() {
        let cache = Cache::new(10, None);
        cache.set("a", 1);
        cache.set("b", 2);

        let removed = cache.delete_where(|_k, v| *v > 100);
        assert_eq!(removed, 0);
        assert_eq!(cache.size(), 2);
    }

    #[test]
    fn test_len_alias() {
        let cache = Cache::new(10, None);
        assert_eq!(cache.len(), 0);
        cache.set("a", 1);
        cache.set("b", 2);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.len(), cache.size());
    }
}
