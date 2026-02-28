//! HNSW vector index using usearch.
//!
//! Provides approximate nearest neighbor search for embedding vectors.
//! Backed by mmap'd file for memory efficiency.

use std::path::Path;

use crate::error::OmniResult;

/// Vector index using usearch HNSW algorithm.
pub struct VectorIndex {
    dimensions: usize,
    // index: Option<usearch::Index>, // initialized when first vector is added
}

impl VectorIndex {
    /// Create or open a vector index at the given path.
    pub fn open(_index_path: &Path, dimensions: usize) -> OmniResult<Self> {
        // TODO: Initialize usearch index with mmap backing
        Ok(Self {
            dimensions,
        })
    }

    /// Add a vector to the index.
    pub fn add(&mut self, _id: u64, _vector: &[f32]) -> OmniResult<()> {
        // TODO: Insert into usearch index
        Ok(())
    }

    /// Search for the K nearest neighbors to the query vector.
    pub fn search(&self, _query: &[f32], k: usize) -> OmniResult<Vec<(u64, f32)>> {
        // TODO: usearch KNN search
        // Returns Vec<(vector_id, distance)>
        let _ = k;
        Ok(Vec::new())
    }

    /// Remove a vector by ID.
    pub fn remove(&mut self, _id: u64) -> OmniResult<()> {
        // TODO: Mark as deleted in usearch
        Ok(())
    }

    /// Returns the number of vectors in the index.
    pub fn len(&self) -> usize {
        0 // TODO
    }

    /// Returns true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the configured dimensions.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_index_creation() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let index = VectorIndex::open(&dir.path().join("vectors.usearch"), 384)
            .expect("create index");
        assert_eq!(index.dimensions(), 384);
        assert!(index.is_empty());
    }
}
