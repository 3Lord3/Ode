use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// File cache of JSON responses keyed by URL with expiry
pub struct Cache {
    dir: PathBuf,
    ttl_secs: u64,
    max_bytes: u64,
}

impl Cache {
    pub fn new(ttl_secs: u64, max_mb: u64) -> Self {
        let dir = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".cache")
            })
            .join("genius");
        Self {
            dir,
            ttl_secs,
            max_bytes: max_mb * 1024 * 1024,
        }
    }

    fn path_for(&self, url: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        url.hash(&mut h);
        self.dir.join(format!("{:016x}.json", h.finish()))
    }

    pub fn get(&self, url: &str) -> Option<String> {
        let p = self.path_for(url);
        let meta = std::fs::metadata(&p).ok()?;
        let age = now().saturating_sub(
            meta.modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_secs(),
        );
        if age > self.ttl_secs {
            return None;
        }
        std::fs::read_to_string(&p).ok()
    }

    pub fn put(&self, url: &str, body: &str) {
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(self.path_for(url), body);
        self.enforce_limit();
    }

    /// Drops the least recently modified entries until the cache fits the limit
    fn enforce_limit(&self) {
        let mut entries: Vec<(std::time::SystemTime, PathBuf, u64)> = Vec::new();
        let Ok(dir) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for e in dir.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            let Ok(mtime) = meta.modified() else { continue };
            entries.push((mtime, e.path(), meta.len()));
        }
        let mut total: u64 = entries.iter().map(|(_, _, s)| s).sum();
        if total <= self.max_bytes {
            return;
        }
        // Oldest first, so the recently requested lyrics survive
        entries.sort_by_key(|(mtime, _, _)| *mtime);
        for (_, path, size) in entries {
            if total <= self.max_bytes {
                break;
            }
            if std::fs::remove_file(&path).is_ok() {
                total = total.saturating_sub(size);
            }
        }
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir)?;
        }
        Ok(())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn put_get() {
        let c = Cache::new(60, 50);
        let u = "https://example.test/1";
        c.put(u, "body");
        assert_eq!(c.get(u).as_deref(), Some("body"));
        c.clear().unwrap();
        assert!(c.get(u).is_none());
    }

    #[test]
    fn expired_entry_is_dropped() {
        let c = Cache::new(0, 50);
        let u = "https://example.test/2";
        c.put(u, "body");
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(c.get(u).is_none());
        c.clear().unwrap();
    }

    #[test]
    fn limit_drops_oldest_first() {
        // 1 MB limit: the second put evicts the first entry
        let c = Cache::new(3600, 1);
        let a = "https://example.test/a";
        let b = "https://example.test/b";
        let big = "x".repeat(700 * 1024);
        c.put(a, &big);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        c.put(b, &big);
        assert!(c.get(a).is_none(), "oldest entry should be evicted");
        assert_eq!(c.get(b).as_deref(), Some(big.as_str()));
        c.clear().unwrap();
    }
}
