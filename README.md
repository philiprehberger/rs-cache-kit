# rs-cache-kit

[![CI](https://github.com/philiprehberger/rs-cache-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-cache-kit/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-cache-kit.svg)](https://crates.io/crates/philiprehberger-cache-kit)
[![License](https://img.shields.io/github/license/philiprehberger/rs-cache-kit)](LICENSE)

Generic LRU cache with TTL, tags, and thread safety for Rust.

## Installation

```toml
[dependencies]
philiprehberger-cache-kit = "0.4.0"
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

### Maintenance

```rust
cache.len()             // entry count (alias for size)
cache.is_empty()        // check if empty
cache.max_size()        // max capacity
cache.keys()            // all non-expired keys
cache.remove_expired()  // clean up expired entries
```


## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## License

MIT
