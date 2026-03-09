# rs-cache-kit

Generic LRU cache with TTL, tags, and thread safety for Rust.

## Installation

```toml
[dependencies]
philiprehberger-cache-kit = "0.1"
```

## Usage

### Basic Cache

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

## License

MIT
