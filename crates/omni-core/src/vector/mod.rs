//! Vector index for approximate nearest neighbor search.
//!
//! Phase 2 implementation uses a flat (brute-force) cosine similarity index
//! backed by an in-memory vector store with disk persistence via bincode.
//!
//! This is correct and sufficient for codebases up to ~100k chunks.
//! HNSW (usearch) integration is planned for Phase 2c when larger
//! indexes are needed.
//!
//! ## Performance
//!
//! Flat search: O(n) per query, but with SIMD-friendly dot products.
//! For 100k vectors of 384 dimensions: ~5ms per query on modern hardware.

use std::collections::HashMap;
use std::path::Path;

use crate::error::{OmniError, OmniResult};

/// Vector index for nearest neighbor search.
///
/// Stores vectors in memory with optional disk persistence.
pub struct VectorIndex {
    dimensions: usize,
    vectors: HashMap<u64, Vec<f32>>,
    index_path: Option<std::path::PathBuf>,
}

impl VectorIndex {
    /// Create or open a vector index at the given path.
    pub fn open(index_path: &Path, dimensions: usize) -> OmniResult<Self> {
        let mut index = Self {
            dimensions,
            vectors: HashMap::new(),
            index_path: Some(index_path.to_path_buf()),
        };

        // Try loading existing index from disk
        if index_path.exists() {
            match index.load_from_disk() {
                Ok(()) => {
                    tracing::info!(
                        vectors = index.vectors.len(),
                        dimensions,
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
            vectors: HashMap::new(),
            index_path: None,
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
    /// Returns `Vec<(id, similarity_score)>` sorted by descending similarity.
    /// Similarity is cosine similarity (dot product of normalized vectors).
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

        // Compute cosine similarity against all vectors
        let mut scores: Vec<(u64, f32)> = self.vectors.iter()
            .map(|(&id, vec)| (id, dot_product(query, vec)))
            .collect();

        // Sort by similarity (descending)
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
            entries: self.vectors.iter()
                .map(|(&id, vec)| (id, vec.clone()))
                .collect(),
        };

        let encoded = bincode::serialize(&data).map_err(|e| {
            OmniError::Internal(format!("failed to serialize vector index: {e}"))
        })?;

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
        let decoded: VectorData = bincode::deserialize(&data).map_err(|e| {
            OmniError::Internal(format!("failed to deserialize vector index: {e}"))
        })?;

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
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            vec.push(((state >> 33) as f32) / (u32::MAX as f32) - 0.5);
        }
        l2_normalize(&mut vec);
        vec
    }

    #[test]
    fn test_vector_index_creation() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let index = VectorIndex::open(&dir.path().join("vectors.bin"), 384)
            .expect("create index");
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
}
