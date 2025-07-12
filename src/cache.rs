use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A simple LRU cache for API responses
pub struct ResponseCache {
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    max_size: usize,
    ttl: Duration,
}

#[derive(Clone)]
struct CacheEntry {
    response: String,
    timestamp: Instant,
    access_count: u32,
}

impl ResponseCache {
    /// Create a new response cache
    pub fn new(max_size: usize, ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            max_size,
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Generate a cache key from user message and tool results
    pub fn generate_key(user_message: &str, tool_results: &[String]) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(user_message.as_bytes());

        for result in tool_results {
            hasher.update(b"|");
            hasher.update(result.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    /// Get a cached response if available and not expired
    pub fn get(&self, key: &str) -> Option<String> {
        let mut cache = self.cache.lock().unwrap();

        if let Some(entry) = cache.get_mut(key) {
            // Check if entry is expired
            if entry.timestamp.elapsed() > self.ttl {
                cache.remove(key);
                return None;
            }

            // Update access count and timestamp for LRU
            entry.access_count += 1;
            entry.timestamp = Instant::now();

            Some(entry.response.clone())
        } else {
            None
        }
    }

    /// Store a response in the cache
    // TODO: Implement proper LRU eviction when cache is full
    // TODO: Add cache persistence to disk for long-term storage
    // TODO: Add cache compression for large responses
    pub fn put(&self, key: String, response: String) {
        let mut cache = self.cache.lock().unwrap();

        // If cache is full, remove least recently used entry
        if cache.len() >= self.max_size && !cache.contains_key(&key) {
            if let Some(lru_key) = self.find_lru_key(&cache) {
                cache.remove(&lru_key);
            }
        }

        cache.insert(
            key,
            CacheEntry {
                response,
                timestamp: Instant::now(),
                access_count: 1,
            },
        );
    }

    /// Find the least recently used key
    fn find_lru_key(&self, cache: &HashMap<String, CacheEntry>) -> Option<String> {
        cache
            .iter()
            .min_by_key(|(_, entry)| (entry.access_count, entry.timestamp))
            .map(|(key, _)| key.clone())
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        let total_entries = cache.len();
        let expired_entries = cache
            .values()
            .filter(|entry| entry.timestamp.elapsed() > self.ttl)
            .count();

        CacheStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_basic() {
        let cache = ResponseCache::new(10, 60);
        let key = ResponseCache::generate_key("test query", &["result1".to_string()]);

        // Test put and get
        cache.put(key.clone(), "cached response".to_string());
        assert_eq!(cache.get(&key), Some("cached response".to_string()));

        // Test non-existent key
        assert_eq!(cache.get("non-existent"), None);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = ResponseCache::new(10, 1); // 1 second TTL
        let key = ResponseCache::generate_key("test query", &[]);

        cache.put(key.clone(), "response".to_string());
        assert_eq!(cache.get(&key), Some("response".to_string()));

        // Wait for expiration
        thread::sleep(Duration::from_secs(2));
        assert_eq!(cache.get(&key), None);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = ResponseCache::new(2, 60); // Max 2 entries

        let key1 = ResponseCache::generate_key("query1", &[]);
        let key2 = ResponseCache::generate_key("query2", &[]);
        let key3 = ResponseCache::generate_key("query3", &[]);

        cache.put(key1.clone(), "response1".to_string());
        cache.put(key2.clone(), "response2".to_string());

        // Access key1 to make it more recently used
        cache.get(&key1);

        // Add key3, should evict key2
        cache.put(key3.clone(), "response3".to_string());

        assert_eq!(cache.get(&key1), Some("response1".to_string()));
        assert_eq!(cache.get(&key2), None); // Should be evicted
        assert_eq!(cache.get(&key3), Some("response3".to_string()));
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = ResponseCache::generate_key("same query", &["result1".to_string()]);
        let key2 = ResponseCache::generate_key("same query", &["result1".to_string()]);
        let key3 = ResponseCache::generate_key("different query", &["result1".to_string()]);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
