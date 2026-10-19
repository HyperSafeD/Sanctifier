//! Analysis result cache for `sanctifier-core`.
//!
//! Caches the output of the most expensive analysis passes keyed by a
//! content-hash of the source string.  Subsequent calls with identical source
//! return the cached value in O(1) without re-parsing or re-walking the AST.
//!
//! # Cache design
//!
//! | Property | Choice | Rationale |
//! |---|---|---|
//! | Key | `u64` (SipHash of source bytes) | Fast, collision-resistant for typical contract sizes |
//! | Eviction | LRU with configurable capacity | Prevents unbounded growth in workspace scans |
//! | Invalidation | Source hash mismatch, explicit [`AnalysisCache::invalidate`], or capacity eviction | Covers all practical cases |
//! | Thread safety | `&mut self` — caller holds the lock | Keeps the cache itself simple; the Analyzer is `Send + Sync` |
//!
//! # Usage
//!
//! ```rust,ignore
//! use sanctifier_core::analysis_cache::AnalysisCache;
//! use sanctifier_core::{Analyzer, SanctifyConfig};
//!
//! let analyzer = Analyzer::new(SanctifyConfig::default());
//! let mut cache = AnalysisCache::new(128);
//!
//! // First call — computes and stores.
//! let findings = cache.get_or_analyze("my/contract.rs", source, || {
//!     analyzer.scan_arithmetic_overflow(source)
//! });
//!
//! // Second call with same source — returns cached value, no re-analysis.
//! let findings2 = cache.get_or_analyze("my/contract.rs", source, || {
//!     analyzer.scan_arithmetic_overflow(source)
//! });
//! assert_eq!(findings, findings2);
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// ── Hash helper ────────────────────────────────────────────────────────────────

fn hash_source(source: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    source.hash(&mut h);
    h.finish()
}

// ── Cache entry ────────────────────────────────────────────────────────────────

struct Entry<V> {
    source_hash: u64,
    value: V,
    /// Logical access timestamp (monotonically increasing hit counter).
    last_used: u64,
}

// ── AnalysisCache ──────────────────────────────────────────────────────────────

/// LRU-capped cache for analysis pass results.
///
/// `V` is typically `Vec<SomeFinding>`.  Use a separate cache instance per
/// analysis pass, or store the pass name as part of the key if you need a
/// single cache for multiple passes.
pub struct AnalysisCache<V> {
    entries: HashMap<String, Entry<V>>,
    capacity: usize,
    clock: u64,
}

impl<V: Clone> AnalysisCache<V> {
    /// Create a new cache with the given maximum number of entries.
    ///
    /// `capacity` must be ≥ 1.  A capacity of `0` is treated as `1`.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity: capacity.max(1),
            clock: 0,
        }
    }

    /// Return the cached result for `key` if the stored source hash matches,
    /// otherwise call `compute`, store the result, and return it.
    ///
    /// `key` is a logical identifier (e.g. a file path).  Two different `key`
    /// strings are always treated as independent cache entries even if the
    /// source content is identical.
    pub fn get_or_analyze<F>(&mut self, key: &str, source: &str, compute: F) -> V
    where
        F: FnOnce() -> V,
    {
        self.clock += 1;
        let hash = hash_source(source);

        if let Some(entry) = self.entries.get_mut(key) {
            if entry.source_hash == hash {
                entry.last_used = self.clock;
                return entry.value.clone();
            }
            // Source changed — invalidate this entry and recompute.
            self.entries.remove(key);
        }

        let value = compute();
        self.maybe_evict();
        self.entries.insert(
            key.to_string(),
            Entry {
                source_hash: hash,
                value: value.clone(),
                last_used: self.clock,
            },
        );
        value
    }

    /// Explicitly remove the cached entry for `key`.
    ///
    /// Returns `true` if an entry was present and removed.
    pub fn invalidate(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Remove all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Return the number of entries currently in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return `true` if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return `true` if `key` has a live entry whose source hash matches `source`.
    pub fn is_cached(&self, key: &str, source: &str) -> bool {
        let hash = hash_source(source);
        self.entries
            .get(key)
            .map(|e| e.source_hash == hash)
            .unwrap_or(false)
    }

    // Evict the least-recently-used entry when the cache is at capacity.
    fn maybe_evict(&mut self) {
        if self.entries.len() < self.capacity {
            return;
        }
        let lru_key = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone());
        if let Some(key) = lru_key {
            self.entries.remove(&key);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── cache hit / miss ──────────────────────────────────────────────────────

    #[test]
    fn cache_miss_on_first_call() {
        let mut cache: AnalysisCache<Vec<String>> = AnalysisCache::new(8);
        let mut called = 0usize;
        cache.get_or_analyze("a.rs", "fn foo() {}", || {
            called += 1;
            vec!["finding".to_string()]
        });
        assert_eq!(called, 1, "compute must be called on first access");
    }

    #[test]
    fn cache_hit_on_second_call_with_same_source() {
        let mut cache: AnalysisCache<Vec<String>> = AnalysisCache::new(8);
        let mut called = 0usize;
        let source = "fn foo() {}";
        for _ in 0..3 {
            cache.get_or_analyze("a.rs", source, || {
                called += 1;
                vec![]
            });
        }
        assert_eq!(
            called, 1,
            "compute must be called only once for identical source"
        );
    }

    #[test]
    fn cache_miss_after_source_changes() {
        let mut cache: AnalysisCache<Vec<String>> = AnalysisCache::new(8);
        let mut called = 0usize;
        cache.get_or_analyze("a.rs", "fn foo() {}", || {
            called += 1;
            vec![]
        });
        cache.get_or_analyze("a.rs", "fn bar() {}", || {
            called += 1;
            vec![]
        });
        assert_eq!(called, 2, "changed source must trigger recompute");
    }

    #[test]
    fn cache_returns_same_value_on_hit() {
        let mut cache: AnalysisCache<Vec<i32>> = AnalysisCache::new(8);
        let source = "fn foo() {}";
        let first = cache.get_or_analyze("a.rs", source, || vec![1, 2, 3]);
        let second = cache.get_or_analyze("a.rs", source, || vec![99]);
        assert_eq!(first, second);
        assert_eq!(second, vec![1, 2, 3]);
    }

    // ── invalidation ──────────────────────────────────────────────────────────

    #[test]
    fn explicit_invalidate_causes_recompute() {
        let mut cache: AnalysisCache<Vec<String>> = AnalysisCache::new(8);
        let source = "fn foo() {}";
        let mut called = 0usize;
        cache.get_or_analyze("a.rs", source, || {
            called += 1;
            vec![]
        });
        assert!(
            cache.invalidate("a.rs"),
            "should return true when entry existed"
        );
        cache.get_or_analyze("a.rs", source, || {
            called += 1;
            vec![]
        });
        assert_eq!(called, 2);
    }

    #[test]
    fn invalidate_missing_key_returns_false() {
        let mut cache: AnalysisCache<Vec<String>> = AnalysisCache::new(8);
        assert!(!cache.invalidate("nonexistent.rs"));
    }

    #[test]
    fn clear_empties_all_entries() {
        let mut cache: AnalysisCache<Vec<String>> = AnalysisCache::new(8);
        cache.get_or_analyze("a.rs", "fn a() {}", Vec::new);
        cache.get_or_analyze("b.rs", "fn b() {}", Vec::new);
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    // ── LRU eviction ──────────────────────────────────────────────────────────

    #[test]
    fn eviction_keeps_cache_within_capacity() {
        let mut cache: AnalysisCache<i32> = AnalysisCache::new(3);
        for i in 0..10u32 {
            cache.get_or_analyze(&format!("{i}.rs"), "fn f() {}", || i as i32);
        }
        assert!(cache.len() <= 3, "cache must not exceed capacity");
    }

    #[test]
    fn is_cached_returns_true_for_live_entry() {
        let mut cache: AnalysisCache<i32> = AnalysisCache::new(8);
        let source = "fn foo() {}";
        cache.get_or_analyze("a.rs", source, || 42);
        assert!(cache.is_cached("a.rs", source));
    }

    #[test]
    fn is_cached_returns_false_after_source_change() {
        let mut cache: AnalysisCache<i32> = AnalysisCache::new(8);
        cache.get_or_analyze("a.rs", "fn foo() {}", || 1);
        assert!(!cache.is_cached("a.rs", "fn bar() {}"));
    }

    // ── capacity = 0 edge case ────────────────────────────────────────────────

    #[test]
    fn zero_capacity_treated_as_one() {
        let mut cache: AnalysisCache<i32> = AnalysisCache::new(0);
        cache.get_or_analyze("a.rs", "fn a() {}", || 1);
        cache.get_or_analyze("b.rs", "fn b() {}", || 2);
        assert!(cache.len() <= 1);
    }
}
