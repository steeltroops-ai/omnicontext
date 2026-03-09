//! Vector index for approximate nearest neighbor search.
//!
//! Supports three search strategies:
//! - **Flat**: Brute-force O(n) scan. Exact results. Best for <10k vectors.
//! - **IVF**: Inverted file with k-means clustering. O(n/k * n_probe).
//!   Best for 10k-100k vectors.
//! - **HNSW**: Hierarchical navigable small world graph. O(log n).
//!   Best for >100k vectors. Pure Rust, no external deps.
//!
//! ## Automatic Strategy Selection
//!
//! Call `build_optimal_index()` to automatically select the best strategy
//! based on index size. Or build explicitly with `build_ivf()` / `build_hnsw()`.
//!
//! ## Performance (384 dimensions)
//!
//! | Strategy | 10k vectors | 100k vectors | 1M vectors |
//! |----------|-------------|--------------|------------|
//! | Flat     | ~0.5ms      | ~5ms         | ~50ms      |
//! | IVF      | ~0.3ms      | ~1ms         | ~5ms       |
//! | HNSW     | ~0.1ms      | ~0.5ms       | ~1ms       |
#![allow(
    clippy::manual_let_else,
    clippy::missing_errors_doc,
    clippy::must_use_candidate
)]

pub mod hnsw;

use std::collections::HashMap;
use std::path::Path;

use crate::error::{OmniError, OmniResult};

/// Distance metric used for nearest neighbor search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum DistanceMetric {
    /// Cosine similarity (dot product of L2-normalized vectors).
    /// Range: [-1, 1], higher is more similar.
    /// Default for most embedding models.
    #[default]
    Cosine,
    /// Euclidean distance (L2 norm of difference vector).
    /// Range: [0, inf), lower is more similar.
    /// Note: search returns negative distance so that sorting by descending
    /// value still gives the most similar results first.
    Euclidean,
    /// Raw dot product (no normalization assumed).
    /// Range: (-inf, inf), higher is more similar.
    DotProduct,
}

/// Vector index for nearest neighbor search.
///
/// Stores vectors in memory with optional disk persistence.
/// Supports flat, IVF, and HNSW search strategies.
pub struct VectorIndex {
    dimensions: usize,
    metric: DistanceMetric,
    vectors: HashMap<u64, Vec<f32>>,
    index_path: Option<std::path::PathBuf>,
    /// Optional IVF index for medium vector sets (10k-100k).
    ivf: Option<IvfIndex>,
    /// Optional HNSW index for large vector sets (>100k).
    hnsw_index: Option<hnsw::HnswIndex>,
}

impl VectorIndex {
    /// Create or open a vector index at the given path.
    pub fn open(index_path: &Path, dimensions: usize) -> OmniResult<Self> {
        Self::open_with_metric(index_path, dimensions, DistanceMetric::default())
    }

    /// Create or open a vector index with a specific distance metric.
    pub fn open_with_metric(
        index_path: &Path,
        dimensions: usize,
        metric: DistanceMetric,
    ) -> OmniResult<Self> {
        let mut index = Self {
            dimensions,
            metric,
            vectors: HashMap::new(),
            index_path: Some(index_path.to_path_buf()),
            ivf: None,
            hnsw_index: None,
        };

        // Try loading existing index from disk
        if index_path.exists() {
            match index.load_from_disk() {
                Ok(()) => {
                    tracing::info!(
                        vectors = index.vectors.len(),
                        dimensions,
                        metric = ?metric,
                        "loaded vector index from disk"
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to load vector index, starting fresh");
                    index.vectors.clear();
                }
            }
        }

        Ok(index)
    }

    /// Create an in-memory-only vector index (for tests).
    pub fn in_memory(dimensions: usize) -> Self {
        Self {
            dimensions,
            metric: DistanceMetric::default(),
            vectors: HashMap::new(),
            index_path: None,
            ivf: None,
            hnsw_index: None,
        }
    }

    /// Create an in-memory vector index with a specific distance metric.
    pub fn in_memory_with_metric(dimensions: usize, metric: DistanceMetric) -> Self {
        Self {
            dimensions,
            metric,
            vectors: HashMap::new(),
            index_path: None,
            ivf: None,
            hnsw_index: None,
        }
    }

    /// Add a vector to the index.
    ///
    /// The vector must have exactly `dimensions` elements and should be
    /// L2-normalized for cosine similarity to work correctly.
    pub fn add(&mut self, id: u64, vector: &[f32]) -> OmniResult<()> {
        if vector.len() != self.dimensions {
            return Err(OmniError::Internal(format!(
                "vector dimension mismatch: expected {}, got {}",
                self.dimensions,
                vector.len()
            )));
        }

        self.vectors.insert(id, vector.to_vec());
        Ok(())
    }

    /// Add multiple vectors in a batch.
    pub fn add_batch(&mut self, entries: &[(u64, Vec<f32>)]) -> OmniResult<()> {
        for (id, vector) in entries {
            self.add(*id, vector)?;
        }
        Ok(())
    }

    /// Search for the K nearest neighbors to the query vector.
    ///
    /// Returns `Vec<(id, score)>` sorted by descending score.
    /// For Cosine/DotProduct: higher = more similar.
    /// For Euclidean: score is negative distance (higher = closer).
    pub fn search(&self, query: &[f32], k: usize) -> OmniResult<Vec<(u64, f32)>> {
        if query.len() != self.dimensions {
            return Err(OmniError::Internal(format!(
                "query dimension mismatch: expected {}, got {}",
                self.dimensions,
                query.len()
            )));
        }

        if self.vectors.is_empty() {
            return Ok(Vec::new());
        }

        // Compute similarity/distance against all vectors using configured metric
        let mut scores: Vec<(u64, f32)> = self
            .vectors
            .iter()
            .map(|(&id, vec)| {
                let score = match self.metric {
                    DistanceMetric::Cosine | DistanceMetric::DotProduct => dot_product(query, vec),
                    DistanceMetric::Euclidean => {
                        // Negate so that smaller distance = higher score
                        -euclidean_distance_sq(query, vec)
                    }
                };
                (id, score)
            })
            .collect();

        // Sort by score descending (highest = most similar/closest)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k
        scores.truncate(k);
        Ok(scores)
    }

    /// Remove a vector by ID.
    pub fn remove(&mut self, id: u64) -> OmniResult<bool> {
        Ok(self.vectors.remove(&id).is_some())
    }

    /// Remove multiple vectors by ID.
    pub fn remove_batch(&mut self, ids: &[u64]) -> OmniResult<usize> {
        let mut removed = 0;
        for &id in ids {
            if self.vectors.remove(&id).is_some() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Returns the number of vectors in the index.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Returns true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Returns the configured dimensions.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Estimate heap memory usage in bytes for stored vectors.
    ///
    /// Does not include HashMap overhead or index structures (IVF/HNSW),
    /// only the raw vector data.
    pub fn memory_usage_bytes(&self) -> usize {
        // Each vector: dimensions * sizeof(f32) + Vec overhead (~24 bytes on 64-bit)
        // HashMap entry overhead: ~64 bytes per entry (key + hash + pointers)
        let per_vector = self.dimensions * std::mem::size_of::<f32>() + 24;
        let per_entry = per_vector + 64;
        self.vectors.len() * per_entry
    }

    /// Persist the index to disk atomically.
    ///
    /// Writes to a temporary file first, then renames to the target path.
    /// This prevents corruption if the process is interrupted mid-write.
    pub fn save(&self) -> OmniResult<()> {
        let path = match &self.index_path {
            Some(p) => p,
            None => return Ok(()), // in-memory mode, nothing to save
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let data = VectorData {
            dimensions: self.dimensions,
            entries: self
                .vectors
                .iter()
                .map(|(&id, vec)| (id, vec.clone()))
                .collect(),
        };

        let encoded = bincode::serialize(&data)
            .map_err(|e| OmniError::Internal(format!("failed to serialize vector index: {e}")))?;

        // Write to temp file alongside target, then atomic rename
        let tmp_path = path.with_extension("bin.tmp");
        std::fs::write(&tmp_path, encoded)?;
        std::fs::rename(&tmp_path, path).map_err(|e| {
            // Clean up temp file on rename failure
            let _ = std::fs::remove_file(&tmp_path);
            OmniError::Io(e)
        })?;

        tracing::debug!(path = %path.display(), vectors = self.len(), "saved vector index (atomic)");

        Ok(())
    }

    /// Load the index from disk.
    fn load_from_disk(&mut self) -> OmniResult<()> {
        let path = match &self.index_path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        let data = std::fs::read(&path)?;
        let decoded: VectorData = bincode::deserialize(&data)
            .map_err(|e| OmniError::Internal(format!("failed to deserialize vector index: {e}")))?;

        if decoded.dimensions != self.dimensions {
            return Err(OmniError::Internal(format!(
                "vector index dimension mismatch: file has {}, config expects {}",
                decoded.dimensions, self.dimensions
            )));
        }

        self.vectors = decoded.entries.into_iter().collect();
        Ok(())
    }
}

/// Serializable vector data for disk persistence.
#[derive(serde::Serialize, serde::Deserialize)]
struct VectorData {
    dimensions: usize,
    entries: Vec<(u64, Vec<f32>)>,
}

// ---------------------------------------------------------------------------
// Math
// ---------------------------------------------------------------------------

/// Compute dot product of two vectors.
///
/// For L2-normalized vectors, this equals cosine similarity.
#[inline]
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    // Simple scalar implementation. LLVM auto-vectorizes this to SIMD.
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compute squared Euclidean distance between two vectors.
///
/// Uses squared distance to avoid the sqrt which is monotonic and thus
/// doesn't affect relative ordering of results.
#[inline]
fn euclidean_distance_sq(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum()
}

/// L2-normalize a vector in place.
pub fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_random_vector(dim: usize, seed: u64) -> Vec<f32> {
        // Deterministic pseudo-random for reproducible tests
        let mut vec = Vec::with_capacity(dim);
        let mut state = seed;
        for _ in 0..dim {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            vec.push(((state >> 33) as f32) / (u32::MAX as f32) - 0.5);
        }
        l2_normalize(&mut vec);
        vec
    }

    #[test]
    fn test_vector_index_creation() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let index = VectorIndex::open(&dir.path().join("vectors.bin"), 384).expect("create index");
        assert_eq!(index.dimensions(), 384);
        assert!(index.is_empty());
    }

    #[test]
    fn test_in_memory_index() {
        let index = VectorIndex::in_memory(3);
        assert_eq!(index.dimensions(), 3);
        assert!(index.is_empty());
    }

    #[test]
    fn test_add_and_search() {
        let mut index = VectorIndex::in_memory(3);

        let mut v1 = vec![1.0, 0.0, 0.0];
        l2_normalize(&mut v1);
        let mut v2 = vec![0.0, 1.0, 0.0];
        l2_normalize(&mut v2);
        let mut v3 = vec![0.9, 0.1, 0.0];
        l2_normalize(&mut v3);

        index.add(1, &v1).expect("add v1");
        index.add(2, &v2).expect("add v2");
        index.add(3, &v3).expect("add v3");

        assert_eq!(index.len(), 3);

        // Search for something close to v1
        let mut query = vec![1.0, 0.0, 0.0];
        l2_normalize(&mut query);

        let results = index.search(&query, 2).expect("search");
        assert_eq!(results.len(), 2);
        // v1 should be the closest, v3 second (most similar to [1,0,0])
        assert_eq!(results[0].0, 1);
        assert_eq!(results[1].0, 3);
    }

    #[test]
    fn test_dimension_mismatch_rejected() {
        let mut index = VectorIndex::in_memory(3);
        let bad_vec = vec![1.0, 0.0]; // wrong dimensions
        assert!(index.add(1, &bad_vec).is_err());

        // Search with wrong dimensions
        let index2 = VectorIndex::in_memory(3);
        assert!(index2.search(&[1.0, 0.0], 1).is_err());
    }

    #[test]
    fn test_remove() {
        let mut index = VectorIndex::in_memory(3);
        index.add(1, &[1.0, 0.0, 0.0]).expect("add");
        index.add(2, &[0.0, 1.0, 0.0]).expect("add");

        assert_eq!(index.len(), 2);

        let removed = index.remove(1).expect("remove");
        assert!(removed);
        assert_eq!(index.len(), 1);

        let not_found = index.remove(99).expect("remove");
        assert!(!not_found);
    }

    #[test]
    fn test_search_empty_index() {
        let index = VectorIndex::in_memory(3);
        let results = index.search(&[1.0, 0.0, 0.0], 5).expect("search");
        assert!(results.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("vectors.bin");

        // Create and save
        {
            let mut index = VectorIndex::open(&path, 3).expect("open");
            index.add(1, &[1.0, 0.0, 0.0]).expect("add");
            index.add(2, &[0.0, 1.0, 0.0]).expect("add");
            index.save().expect("save");
        }

        // Load and verify
        {
            let index = VectorIndex::open(&path, 3).expect("open");
            assert_eq!(index.len(), 2);

            let results = index.search(&[1.0, 0.0, 0.0], 1).expect("search");
            assert_eq!(results[0].0, 1);
        }
    }

    #[test]
    fn test_l2_normalize() {
        let mut vec = vec![3.0, 4.0];
        l2_normalize(&mut vec);

        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "should be unit length");
        assert!((vec[0] - 0.6).abs() < 1e-6);
        assert!((vec[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product_normalized() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((dot_product(&a, &b) - 1.0).abs() < 1e-6, "identical = 1.0");

        let c = vec![0.0, 1.0, 0.0];
        assert!((dot_product(&a, &c)).abs() < 1e-6, "orthogonal = 0.0");
    }

    #[test]
    fn test_larger_index() {
        let dim = 384;
        let mut index = VectorIndex::in_memory(dim);

        // Insert 1000 vectors
        for i in 0..1000 {
            let v = make_random_vector(dim, i);
            index.add(i, &v).expect("add");
        }

        assert_eq!(index.len(), 1000);

        // Search should return top-k
        let query = make_random_vector(dim, 42);
        let results = index.search(&query, 10).expect("search");
        assert_eq!(results.len(), 10);

        // Results should be sorted by descending score
        for i in 1..results.len() {
            assert!(results[i - 1].1 >= results[i].1, "should be descending");
        }

        // The vector most similar to query(seed=42) should be itself
        assert_eq!(results[0].0, 42, "self-similarity should be highest");
    }

    #[test]
    fn test_add_batch() {
        let mut index = VectorIndex::in_memory(3);
        let entries = vec![
            (1, vec![1.0, 0.0, 0.0]),
            (2, vec![0.0, 1.0, 0.0]),
            (3, vec![0.0, 0.0, 1.0]),
        ];
        index.add_batch(&entries).expect("batch add");
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn test_remove_batch() {
        let mut index = VectorIndex::in_memory(3);
        index.add(1, &[1.0, 0.0, 0.0]).expect("add");
        index.add(2, &[0.0, 1.0, 0.0]).expect("add");
        index.add(3, &[0.0, 0.0, 1.0]).expect("add");

        let removed = index.remove_batch(&[1, 3, 99]).expect("batch remove");
        assert_eq!(removed, 2);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_ivf_build_empty() {
        let mut index = VectorIndex::in_memory(3);
        let result = index.build_ivf(4, 2);
        assert!(result.is_ok());
        assert!(
            index.ivf.is_none(),
            "IVF should not be built for empty index"
        );
    }

    #[test]
    fn test_ivf_build_and_search() {
        let mut index = VectorIndex::in_memory(3);
        // Add enough vectors for clustering
        for i in 0..50 {
            let v = vec![(i as f32).cos(), (i as f32).sin(), (i as f32) * 0.1];
            index.add(i, &v).expect("add");
        }
        index.build_ivf(5, 2).expect("build IVF");
        assert!(index.ivf.is_some(), "IVF should be built");

        let query = vec![1.0_f32.cos(), 1.0_f32.sin(), 0.1];
        let results = index.search_ivf(&query, 5).expect("search IVF");
        assert!(!results.is_empty(), "IVF search should return results");
        assert!(results.len() <= 5);

        // Result with ID=1 should be the closest (same vector)
        assert_eq!(results[0].0, 1, "closest vector should match query ID");
    }

    #[test]
    fn test_ivf_fallback_to_flat() {
        let mut index = VectorIndex::in_memory(3);
        index.add(1, &[1.0, 0.0, 0.0]).expect("add");

        // No IVF built, should fall back to flat
        let results = index.search_ivf(&[1.0, 0.0, 0.0], 1).expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn test_hnsw_build_and_search() {
        let mut index = VectorIndex::in_memory(3);
        for i in 0..50 {
            let v = vec![(i as f32).cos(), (i as f32).sin(), 0.0];
            index.add(i, &v).expect("add");
        }
        index
            .build_hnsw(hnsw::HnswConfig::default())
            .expect("build HNSW");
        assert!(index.hnsw_index.is_some());

        let query = vec![1.0_f32.cos(), 1.0_f32.sin(), 0.0];
        let results = index.search_hnsw(&query, 5).expect("search HNSW");
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 1, "closest should be vector 1");
    }

    #[test]
    fn test_hnsw_fallback_to_flat() {
        let mut index = VectorIndex::in_memory(3);
        index.add(1, &[1.0, 0.0, 0.0]).expect("add");

        let results = index.search_hnsw(&[1.0, 0.0, 0.0], 1).expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn test_search_best_uses_flat_by_default() {
        let mut index = VectorIndex::in_memory(3);
        index.add(1, &[1.0, 0.0, 0.0]).expect("add");
        assert_eq!(index.active_strategy(), "flat");

        let results = index.search_best(&[1.0, 0.0, 0.0], 1).expect("search");
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn test_search_best_prefers_hnsw() {
        let mut index = VectorIndex::in_memory(3);
        for i in 0..20 {
            index.add(i, &make_random_vector(3, i)).expect("add");
        }
        index.build_ivf(3, 2).expect("IVF");
        assert_eq!(index.active_strategy(), "ivf");

        index.build_hnsw(hnsw::HnswConfig::default()).expect("HNSW");
        assert_eq!(index.active_strategy(), "hnsw");

        let results = index
            .search_best(&make_random_vector(3, 5), 3)
            .expect("search");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_build_optimal_flat() {
        let mut index = VectorIndex::in_memory(3);
        for i in 0..100 {
            index.add(i, &make_random_vector(3, i)).expect("add");
        }
        index.build_optimal_index().expect("optimal");
        assert_eq!(index.active_strategy(), "flat", "<5000 should stay flat");
    }

    #[test]
    fn test_hnsw_scores_are_similarities() {
        let mut index = VectorIndex::in_memory(3);
        index.add(1, &[1.0, 0.0, 0.0]).expect("add");
        index.add(2, &[0.0, 1.0, 0.0]).expect("add");
        index.add(3, &[-1.0, 0.0, 0.0]).expect("add");

        index.build_hnsw(hnsw::HnswConfig::default()).expect("HNSW");

        let results = index.search_hnsw(&[1.0, 0.0, 0.0], 3).expect("search");
        // Self should have score ~1.0
        assert!(
            results[0].1 > 0.95,
            "self-similarity should be ~1.0, got {}",
            results[0].1
        );
        // Orthogonal should have score ~0.0
        assert!((results[1].1).abs() < 0.1, "orthogonal should be ~0.0");
    }
}

/// Inverted File Index for sub-linear ANN search.
///
/// Partitions vectors into clusters using k-means centroids.
/// At query time, only `n_probe` nearest clusters are searched.
///
/// Complexity: O(n_clusters + n_probe * n/n_clusters) per query
/// vs O(n) for flat scan.
#[derive(Debug, Clone)]
pub struct IvfIndex {
    /// Centroid vectors for each cluster.
    centroids: Vec<Vec<f32>>,
    /// Mapping from cluster_id -> vec of (vector_id, vector) in that cluster.
    buckets: Vec<Vec<(u64, Vec<f32>)>>,
    /// Number of clusters to probe at query time.
    n_probe: usize,
}

impl VectorIndex {
    /// Build an IVF index over the current vectors.
    ///
    /// Uses k-means clustering with `n_clusters` centroids.
    /// At query time, `n_probe` nearest clusters are searched.
    ///
    /// Recommended: `n_clusters = sqrt(N)`, `n_probe = max(3, sqrt(n_clusters))`.
    ///
    /// Call this after all vectors are added (e.g., after indexing completes).
    /// The flat search (`search()`) remains available as a fallback.
    pub fn build_ivf(&mut self, n_clusters: usize, n_probe: usize) -> OmniResult<()> {
        let n = self.vectors.len();
        if n < n_clusters || n < 2 {
            self.ivf = None;
            return Ok(());
        }

        let dims = self.dimensions;
        let all_vecs: Vec<(&u64, &Vec<f32>)> = self.vectors.iter().collect();

        // Simple k-means++ initialization
        let mut centroids = Vec::with_capacity(n_clusters);
        // First centroid: pick the first vector
        centroids.push(all_vecs[0].1.clone());

        // Remaining centroids: pick vectors that are far from existing centroids
        for _ in 1..n_clusters {
            let mut best_dist = f32::NEG_INFINITY;
            let mut best_idx = 0;
            for (idx, (_, vec)) in all_vecs.iter().enumerate() {
                let min_d = centroids
                    .iter()
                    .map(|c| cosine_sim(vec, c))
                    .fold(f32::INFINITY, f32::min);
                let neg_sim = -min_d; // want to maximize distance = minimize similarity
                if neg_sim > best_dist {
                    best_dist = neg_sim;
                    best_idx = idx;
                }
            }
            centroids.push(all_vecs[best_idx].1.clone());
        }

        // Run k-means iterations (10 iterations is sufficient for code embeddings)
        for _ in 0..10 {
            // Assign each vector to nearest centroid
            let mut assignments: Vec<Vec<usize>> = vec![Vec::new(); n_clusters];
            for (idx, (_, vec)) in all_vecs.iter().enumerate() {
                let mut best_c = 0;
                let mut best_sim = f32::NEG_INFINITY;
                for (c, centroid) in centroids.iter().enumerate() {
                    let sim = cosine_sim(vec, centroid);
                    if sim > best_sim {
                        best_sim = sim;
                        best_c = c;
                    }
                }
                assignments[best_c].push(idx);
            }

            // Recompute centroids as mean of assigned vectors
            for (c, assigned) in assignments.iter().enumerate() {
                if assigned.is_empty() {
                    continue;
                }
                let mut new_centroid = vec![0.0f32; dims];
                for &idx in assigned {
                    for (d, val) in all_vecs[idx].1.iter().enumerate() {
                        if d < dims {
                            new_centroid[d] += val;
                        }
                    }
                }
                let n_assigned = assigned.len() as f32;
                for val in &mut new_centroid {
                    *val /= n_assigned;
                }
                // L2 normalize the centroid
                let norm = new_centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 1e-10 {
                    for val in &mut new_centroid {
                        *val /= norm;
                    }
                }
                centroids[c] = new_centroid;
            }
        }

        // Build final buckets
        let mut buckets: Vec<Vec<(u64, Vec<f32>)>> = vec![Vec::new(); n_clusters];
        for (&id, vec) in &self.vectors {
            let mut best_c = 0;
            let mut best_sim = f32::NEG_INFINITY;
            for (c, centroid) in centroids.iter().enumerate() {
                let sim = cosine_sim(vec, centroid);
                if sim > best_sim {
                    best_sim = sim;
                    best_c = c;
                }
            }
            buckets[best_c].push((id, vec.clone()));
        }

        tracing::info!(n_clusters, n_probe, total_vectors = n, "IVF index built");

        self.ivf = Some(IvfIndex {
            centroids,
            buckets,
            n_probe,
        });

        Ok(())
    }

    /// Search using the IVF index. Falls back to flat search if IVF not built.
    pub fn search_ivf(&self, query: &[f32], k: usize) -> OmniResult<Vec<(u64, f32)>> {
        let ivf = match &self.ivf {
            Some(ivf) => ivf,
            None => return self.search(query, k),
        };

        // Find the n_probe nearest centroids
        let mut centroid_sims: Vec<(usize, f32)> = ivf
            .centroids
            .iter()
            .enumerate()
            .map(|(i, c)| (i, cosine_sim(query, c)))
            .collect();
        centroid_sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        centroid_sims.truncate(ivf.n_probe);

        // Search only the selected buckets
        let mut norm_query = query.to_vec();
        l2_normalize(&mut norm_query);
        let mut scores: Vec<(u64, f32)> = Vec::new();

        for (cluster_id, _) in &centroid_sims {
            for (id, vec) in &ivf.buckets[*cluster_id] {
                let score = match self.metric {
                    DistanceMetric::Cosine => {
                        let mut nv = vec.clone();
                        l2_normalize(&mut nv);
                        dot_product(&norm_query, &nv)
                    }
                    DistanceMetric::DotProduct => dot_product(query, vec),
                    DistanceMetric::Euclidean => -euclidean_distance_sq(query, vec),
                };
                scores.push((*id, score));
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        Ok(scores)
    }

    /// Build an HNSW index over the current vectors.
    ///
    /// This is the preferred strategy for large indexes (>50k vectors).
    /// Provides O(log n) search time with >95% recall.
    ///
    /// The HNSW index is built in-memory and not persisted to disk.
    /// Call this after all vectors are added.
    pub fn build_hnsw(&mut self, config: hnsw::HnswConfig) -> OmniResult<()> {
        let n = self.vectors.len();
        if n < 2 {
            self.hnsw_index = None;
            return Ok(());
        }

        let vectors: Vec<(u64, Vec<f32>)> = self
            .vectors
            .iter()
            .map(|(&id, vec)| (id, vec.clone()))
            .collect();
        let refs: Vec<(u64, &[f32])> = vectors.iter().map(|(id, v)| (*id, v.as_slice())).collect();

        let hnsw = hnsw::HnswIndex::build_batch(self.dimensions, config, &refs);

        tracing::info!(
            nodes = hnsw.len(),
            dimensions = self.dimensions,
            "HNSW index built"
        );

        self.hnsw_index = Some(hnsw);
        Ok(())
    }

    /// Search using the HNSW index. Falls back to flat search if HNSW not built.
    ///
    /// Returns results as `(id, score)` where score follows the same convention
    /// as `search()`: higher = more similar for Cosine/DotProduct.
    pub fn search_hnsw(&self, query: &[f32], k: usize) -> OmniResult<Vec<(u64, f32)>> {
        let hnsw = match &self.hnsw_index {
            Some(h) => h,
            None => return self.search(query, k),
        };

        let raw_results = hnsw.search(query, k);

        // Convert HNSW cosine distance to similarity score
        // HNSW returns (id, distance) where distance = 1 - cosine_similarity
        let results: Vec<(u64, f32)> = raw_results
            .into_iter()
            .map(|(id, dist)| (id, 1.0 - dist)) // Convert back to similarity
            .collect();

        Ok(results)
    }

    /// Search using the best available index strategy.
    ///
    /// Priority: HNSW > IVF > Flat.
    ///
    /// This is the recommended search API for production use.
    pub fn search_best(&self, query: &[f32], k: usize) -> OmniResult<Vec<(u64, f32)>> {
        if self.hnsw_index.is_some() {
            return self.search_hnsw(query, k);
        }
        if self.ivf.is_some() {
            return self.search_ivf(query, k);
        }
        self.search(query, k)
    }

    /// Automatically build the optimal index for the current vector count.
    ///
    /// Strategy selection:
    /// - <5,000 vectors: Flat (exact, fast enough)
    /// - 5,000-50,000 vectors: IVF (good recall, sub-linear)
    /// - >50,000 vectors: HNSW (best recall, O(log n))
    pub fn build_optimal_index(&mut self) -> OmniResult<()> {
        let n = self.vectors.len();

        if n < 5_000 {
            // Flat search is fast enough
            tracing::debug!(n, "flat search sufficient, skipping ANN index");
            return Ok(());
        }

        if n < 50_000 {
            // IVF is a good trade-off
            let n_clusters = (n as f64).sqrt() as usize;
            let n_probe = (n_clusters as f64).sqrt().max(3.0) as usize;
            tracing::info!(n, n_clusters, n_probe, "building IVF index");
            return self.build_ivf(n_clusters, n_probe);
        }

        // Large index: HNSW
        tracing::info!(n, "building HNSW index for large vector set");
        self.build_hnsw(hnsw::HnswConfig::for_code_search())
    }

    /// Returns the current search strategy as a string (for diagnostics).
    pub fn active_strategy(&self) -> &'static str {
        if self.hnsw_index.is_some() {
            "hnsw"
        } else if self.ivf.is_some() {
            "ivf"
        } else {
            "flat"
        }
    }
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let mut na = a.to_vec();
    let mut nb = b.to_vec();
    l2_normalize(&mut na);
    l2_normalize(&mut nb);
    dot_product(&na, &nb)
}
