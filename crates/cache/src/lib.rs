//! Persistent graph caching using sled

use anyhow::Result;
use sled::Db;

pub struct PersistentCache {
    db: Db,
}

impl PersistentCache {
    /// Open or create a cache database
    pub fn open(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Store a key-value pair
    pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.insert(key, value)?;
        Ok(())
    }

    /// Retrieve a value by key
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(key)?.map(|v| v.to_vec()))
    }

    /// Remove a key
    pub fn remove(&self, key: &str) -> Result<()> {
        self.db.remove(key)?;
        Ok(())
    }

    /// Clear all entries
    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_operations() {
        let cache = PersistentCache::open(".nexus/test_cache.db").unwrap();
        
        cache.put("key1", b"value1").unwrap();
        let result = cache.get("key1").unwrap();
        assert_eq!(result, Some(b"value1".to_vec()));
        
        cache.remove("key1").unwrap();
        let result = cache.get("key1").unwrap();
        assert_eq!(result, None);
    }
}
