# Changelog

## 0.4.1

- Add crate-level documentation with usage examples

## 0.4.0 (2026-03-17)

- Add `CacheStats` struct and `stats()` method for tracking hits, misses, and evictions
- Add `get_many()` method for batch key retrieval
- Add `delete_where()` method for conditional entry removal
- Add `len()` method as alias for `size()`

## 0.3.6

- Fix clippy incompatible_msrv: replace is_none_or with map_or for Rust 1.70 compatibility

## 0.3.5

- Add readme, rust-version, documentation to Cargo.toml
- Remove redundant license-file from Cargo.toml
- Add Development section to README
## 0.3.4 (2026-03-16)

- Update install snippet to use full version

## 0.3.3 (2026-03-16)

- Add README badges
- Synchronize version across Cargo.toml, README, and CHANGELOG

## 0.3.0 (2026-03-13)

- Add `keys()` method — returns all non-expired keys
- Add `get_or_insert_with()` method — atomically get or compute+insert a value
- Add `remove_expired()` method — proactive cleanup of expired entries
- Add `is_empty()` convenience method
- Add `max_size()` getter for inspecting cache capacity

## 0.2.0 (2026-03-12)

- Add `Debug` trait implementation for `Cache`
- Add `Default` trait implementation for `Cache` (defaults to max_size=100, no TTL)
- Fix `has()` method to no longer require `V: Clone`
- Add comprehensive test suite covering get/set, TTL, LRU eviction, tags, thread safety

## 0.1.0 (2026-03-09)

- Initial release
