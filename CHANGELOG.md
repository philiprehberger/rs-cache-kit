# Changelog
n## 0.3.4 (2026-03-16)

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
