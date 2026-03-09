//! Message compression for IPC optimization.
//!
//! Compresses large JSON-RPC messages (>100KB) using LZ4 to reduce
//! transmission time and memory usage. Compression is transparent to
//! the protocol - messages are compressed before transmission and
//! decompressed on receipt.
//!
//! ## Strategy
//!
//! - Messages <100KB: No compression (overhead not worth it)
//! - Messages ≥100KB: LZ4 compression with header
//! - Header format: `LZ4:<original_size>:<compressed_data>`
//!
//! ## Performance
//!
//! - Compression: ~500 MB/s (LZ4 fast mode)
//! - Decompression: ~2 GB/s
//! - Typical reduction: 5-10x for JSON payloads
//!
//! ## Example
//!
//! ```rust
//! use omni_daemon::compression::{compress_if_beneficial, decompress_if_compressed};
//!
//! let data = "large JSON payload...".as_bytes();
//! let compressed = compress_if_beneficial(data);
//! let decompressed = decompress_if_compressed(&compressed).unwrap();
//! assert_eq!(data, decompressed.as_slice());
//! ```

use std::io::{self, Write};

/// Compression threshold in bytes (100KB).
/// Messages smaller than this are not compressed.
const COMPRESSION_THRESHOLD: usize = 100 * 1024;

/// Compression header prefix.
const COMPRESSION_HEADER: &str = "LZ4:";

/// Compress data if it exceeds the threshold.
///
/// Returns the compressed data with header if compression was beneficial,
/// otherwise returns the original data unchanged.
pub fn compress_if_beneficial(data: &[u8]) -> Vec<u8> {
    if data.len() < COMPRESSION_THRESHOLD {
        // Too small to benefit from compression
        return data.to_vec();
    }

    match lz4::block::compress(data, None, false) {
        Ok(compressed) => {
            // Check if compression actually reduced size
            if compressed.len() < data.len() {
                // Build header: LZ4:<original_size>:<compressed_data>
                let mut result = Vec::with_capacity(compressed.len() + 32);
                write!(&mut result, "{}{}:", COMPRESSION_HEADER, data.len()).expect("write to vec");
                result.extend_from_slice(&compressed);

                tracing::debug!(
                    original_size = data.len(),
                    compressed_size = result.len(),
                    ratio = (data.len() as f64 / result.len() as f64),
                    "compressed message"
                );

                result
            } else {
                // Compression didn't help, return original
                data.to_vec()
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "compression failed, using uncompressed");
            data.to_vec()
        }
    }
}

/// Decompress data if it has a compression header.
///
/// Returns the decompressed data if compressed, otherwise returns
/// the original data unchanged.
pub fn decompress_if_compressed(data: &[u8]) -> io::Result<Vec<u8>> {
    // Check for compression header
    if data.len() < COMPRESSION_HEADER.len() {
        return Ok(data.to_vec());
    }

    let header_bytes = &data[..COMPRESSION_HEADER.len()];
    if header_bytes != COMPRESSION_HEADER.as_bytes() {
        // Not compressed
        return Ok(data.to_vec());
    }

    // Parse header: LZ4:<original_size>:<compressed_data>
    let header_end = data
        .iter()
        .skip(COMPRESSION_HEADER.len())
        .position(|&b| b == b':')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid compression header"))?;

    let size_str =
        std::str::from_utf8(&data[COMPRESSION_HEADER.len()..COMPRESSION_HEADER.len() + header_end])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let original_size: usize = size_str
        .parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let compressed_data = &data[COMPRESSION_HEADER.len() + header_end + 1..];

    // Decompress
    let decompressed = lz4::block::decompress(compressed_data, Some(original_size as i32))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    tracing::debug!(
        compressed_size = data.len(),
        decompressed_size = decompressed.len(),
        ratio = (decompressed.len() as f64 / data.len() as f64),
        "decompressed message"
    );

    Ok(decompressed)
}

/// Get compression statistics for a message.
#[derive(Debug, Clone)]
pub struct CompressionStats {
    /// Original message size in bytes.
    pub original_size: usize,
    /// Compressed message size in bytes (including header).
    pub compressed_size: usize,
    /// Whether compression was applied.
    pub compressed: bool,
    /// Compression ratio (original / compressed).
    pub ratio: f64,
}

impl CompressionStats {
    /// Calculate compression stats for a message.
    pub fn calculate(original: &[u8], compressed: &[u8]) -> Self {
        let is_compressed = compressed.starts_with(COMPRESSION_HEADER.as_bytes());
        Self {
            original_size: original.len(),
            compressed_size: compressed.len(),
            compressed: is_compressed,
            ratio: if is_compressed {
                original.len() as f64 / compressed.len() as f64
            } else {
                1.0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_message_not_compressed() {
        let data = b"small message";
        let compressed = compress_if_beneficial(data);
        assert_eq!(compressed, data);
    }

    #[test]
    fn test_large_message_compressed() {
        // Create a large compressible message (repeated pattern)
        let data = "x".repeat(200_000);
        let compressed = compress_if_beneficial(data.as_bytes());

        // Should be compressed (starts with header)
        assert!(compressed.starts_with(COMPRESSION_HEADER.as_bytes()));
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_compression_roundtrip() {
        let data = "test data ".repeat(20_000);
        let compressed = compress_if_beneficial(data.as_bytes());
        let decompressed = decompress_if_compressed(&compressed).expect("decompress");

        assert_eq!(data.as_bytes(), decompressed.as_slice());
    }

    #[test]
    fn test_uncompressed_passthrough() {
        let data = b"uncompressed data";
        let result = decompress_if_compressed(data).expect("decompress");
        assert_eq!(data, result.as_slice());
    }

    #[test]
    fn test_compression_stats() {
        let original = "x".repeat(200_000);
        let compressed = compress_if_beneficial(original.as_bytes());

        let stats = CompressionStats::calculate(original.as_bytes(), &compressed);
        assert!(stats.compressed);
        assert!(stats.ratio > 1.0);
        assert_eq!(stats.original_size, original.len());
    }

    #[test]
    fn test_invalid_header() {
        let data = b"LZ4:invalid:data";
        let result = decompress_if_compressed(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_compression_threshold() {
        // Just below threshold
        let data = vec![b'x'; COMPRESSION_THRESHOLD - 1];
        let compressed = compress_if_beneficial(&data);
        assert_eq!(compressed, data);

        // At threshold
        let data = vec![b'x'; COMPRESSION_THRESHOLD];
        let compressed = compress_if_beneficial(&data);
        assert!(compressed.starts_with(COMPRESSION_HEADER.as_bytes()));
    }

    #[test]
    fn test_incompressible_data() {
        // Random data doesn't compress well
        let data: Vec<u8> = (0..200_000).map(|i| (i % 256) as u8).collect();
        let compressed = compress_if_beneficial(&data);

        // Should return original if compression didn't help
        // (or compressed if LZ4 still managed some reduction)
        assert!(compressed.len() <= data.len() * 2); // Sanity check
    }
}
