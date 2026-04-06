# rs-cache-kit

[![CI](https://github.com/philiprehberger/rs-cache-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-cache-kit/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-cache-kit.svg)](https://crates.io/crates/philiprehberger-cache-kit)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-cache-kit)](https://github.com/philiprehberger/rs-cache-kit/commits/main)

Generic LRU cache with TTL, tags, and async support for Rust

## Installation

```toml
[dependencies]
philiprehberger-cache-kit = "0.5.0"
```

## Usage

```rust
use philiprehberger_cache_kit::Cache;
use std::time::Duration;

let cache = Cache::new(1000, Some(Duration::from_secs(300)));

cache.set("key".to_string(), "value".to_string());
let val = cache.get(&"key".to_string()); // Some("value")
```

### Custom TTL and Tags

```rust
cache.set_with(
    "user:1".to_string(),
    user_data,
    Some(Duration::from_secs(60)),
    &["users", "team-a"],
);
```

### Tag-Based Invalidation

```rust
let removed = cache.invalidate_by_tag("team-a");
println!("Removed {} entries", removed);
```

### Thread Safety

The cache is `Clone` and uses `Arc<RwLock<...>>` internally — safe to share across threads.

```rust
let cache = Cache::new(100, None);
let cache2 = cache.clone();

std::thread::spawn(move || {
    cache2.set("key".to_string(), "from thread".to_string());
});
```

### Other Operations

```rust
cache.has(&key)      // check existence
cache.delete(&key)   // delete entry
cache.size()         // entry count
cache.clear()        // remove all
```

### Get or Insert

```rust
let value = cache.get_or_insert_with("key".to_string(), || {
    expensive_computation()
});
```

### Cache Stats

Track hit/miss/eviction counters for monitoring and tuning:

```rust
let cache = Cache::new(100, None);
cache.set("a".to_string(), 1);
cache.get(&"a".to_string()); // hit
cache.get(&"z".to_string()); // miss

let stats = cache.stats();
println!("Hits: {}, Misses: {}, Evictions: {}", stats.hits, stats.misses, stats.evictions);
```

### Batch Get

Retrieve multiple keys in one call:

```rust
cache.set("x".to_string(), 1);
cache.set("y".to_string(), 2);

let results = cache.get_many(&["x".to_string(), "y".to_string(), "z".to_string()]);
// returns HashMap with "x" => 1, "y" => 2 (missing keys omitted)
```

### Conditional Delete

Remove entries matching a predicate:

```rust
cache.set("small".to_string(), 1);
cache.set("big".to_string(), 1000);

let removed = cache.delete_where(|_key, value| *value > 100);
// removed == 1, "big" is gone
```

### Peek Without LRU Update

```rust
use philiprehberger_cache_kit::Cache;

let cache = Cache::new(100, None);
cache.set("key", "value");

// Read without affecting eviction order
let val = cache.peek(&"key");
```

### Eviction Callback

```rust
use philiprehberger_cache_kit::Cache;
use std::sync::{Arc, Mutex};

let cache = Cache::new(2, None);
let log = Arc::new(Mutex::new(Vec::new()));
let log2 = log.clone();
cache.on_evict(move |key: &String, _val: &String| {
    log2.lock().unwrap().push(key.clone());
});

cache.set("a".into(), "1".into());
cache.set("b".into(), "2".into());
cache.set("c".into(), "3".into()); // evicts "a"
```

### TTL Remaining

```rust
use philiprehberger_cache_kit::Cache;
use std::time::Duration;

let cache = Cache::new(100, Some(Duration::from_secs(60)));
cache.set("key", "value");

if let Some(remaining) = cache.entry_ttl_remaining(&"key") {
    println!("TTL remaining: {:?}", remaining);
}
```

### Maintenance

```rust
cache.len()             // entry count (alias for size)
cache.is_empty()        // check if empty
cache.max_size()        // max capacity
cache.keys()            // all non-expired keys
cache.remove_expired()  // clean up expired entries
```

## API

| Function / Type | Description |
|-----------------|-------------|
| `Cache::new(max_size, default_ttl)` | Create a new cache with max capacity and optional default TTL |
| `Cache::default()` | Create a cache with max_size=100 and no TTL |
| `cache.set(key, value)` | Insert a value with default TTL and no tags |
| `cache.set_with(key, value, ttl, tags)` | Insert a value with custom TTL and tags |
| `cache.get(key)` | Get a value (returns `None` if missing or expired) |
| `cache.get_many(keys)` | Retrieve multiple values at once |
| `cache.get_or_insert_with(key, f)` | Get or compute and insert a value |
| `cache.has(key)` | Check if a key exists and is not expired |
| `cache.delete(key)` | Delete an entry by key |
| `cache.delete_where(predicate)` | Remove entries matching a predicate |
| `cache.invalidate_by_tag(tag)` | Remove all entries with the given tag |
| `cache.clear()` | Remove all entries |
| `cache.size()` / `cache.len()` | Return the number of entries |
| `cache.is_empty()` | Check if the cache is empty |
| `cache.max_size()` | Return the max capacity |
| `cache.keys()` | Return all non-expired keys |
| `cache.remove_expired()` | Clean up expired entries |
| `cache.peek(key)` | Read a value without updating LRU order |
| `cache.on_evict(callback)` | Register a callback for cache evictions |
| `cache.entry_ttl_remaining(key)` | Check remaining TTL for an entry |
| `cache.stats()` | Return hit/miss/eviction counters as `CacheStats` |
| `CacheStats` | Struct with `hits`, `misses`, `evictions` fields |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-cache-kit)

🐛 [Report issues](https://github.com/philiprehberger/rs-cache-kit/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-cache-kit/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
