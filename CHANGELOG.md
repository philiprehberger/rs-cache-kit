# Changelog

## 0.2.0

- Add `Debug` trait implementation for `Cache`
- Add `Default` trait implementation for `Cache` (defaults to max_size=100, no TTL)
- Fix `has()` method to no longer require `V: Clone`
- Add comprehensive test suite covering get/set, TTL, LRU eviction, tags, thread safety

## 0.1.0

- Initial release
