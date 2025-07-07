/*!
Compression adapters for snapshot data.

This module provides compression functionality to reduce snapshot file sizes.
The default implementation uses gzip compression, but the architecture allows
for plugging in different compression algorithms.
*/

use std::io::{Read, Write};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use crate::{PersistError, Result};

/// Compression abstraction for snapshot data
///
/// This trait defines the interface for all compression implementations.
/// It allows the core engine to work with different compression algorithms
/// without being coupled to any specific implementation.
pub trait CompressionAdapter {
    /// Compress the input data
    ///
    /// # Arguments
    /// * `data` - The data to compress
    ///
    /// # Returns
    /// The compressed data or an error
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Decompress the input data
    ///
    /// # Arguments
    /// * `compressed_data` - The compressed data to decompress
    ///
    /// # Returns
    /// The decompressed data or an error
    fn decompress(&self, compressed_data: &[u8]) -> Result<Vec<u8>>;

    /// Get the name of the compression algorithm
    fn algorithm_name(&self) -> &str;
}

/// Gzip compression adapter
///
/// This implementation uses the DEFLATE algorithm (gzip) to compress snapshot data.
/// It provides a good balance of compression ratio and speed for most use cases.
///
/// # Example
/// ```rust
/// use persist_core::GzipCompressor;
/// 
/// let compressor = GzipCompressor::new();
/// let data = b"some agent state data to compress";
/// let compressed = compressor.compress(data)?;
/// let decompressed = compressor.decompress(&compressed)?;
/// assert_eq!(data, &decompressed[..]);
/// ```
#[derive(Debug, Clone)]
pub struct GzipCompressor {
    compression_level: Compression,
}

impl GzipCompressor {
    /// Create a new gzip compressor with default compression level (6)
    pub fn new() -> Self {
        Self {
            compression_level: Compression::default(),
        }
    }

    /// Create a new gzip compressor with the specified compression level
    ///
    /// # Arguments
    /// * `level` - Compression level (0-9, where 0 is no compression and 9 is maximum)
    ///
    /// # Example
    /// ```rust
    /// use persist_core::GzipCompressor;
    /// 
    /// // Fast compression (less CPU, larger files)
    /// let fast_compressor = GzipCompressor::with_level(1);
    /// 
    /// // Maximum compression (more CPU, smaller files)
    /// let max_compressor = GzipCompressor::with_level(9);
    /// ```
    pub fn with_level(level: u32) -> Self {
        Self {
            compression_level: Compression::new(level),
        }
    }

    /// Create a compressor for fast compression (level 1)
    pub fn fast() -> Self {
        Self::with_level(1)
    }

    /// Create a compressor for maximum compression (level 9)
    pub fn max() -> Self {
        Self::with_level(9)
    }
}

impl Default for GzipCompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionAdapter for GzipCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), self.compression_level);
        
        encoder.write_all(data)
            .map_err(|e| PersistError::compression(format!("Failed to write data for compression: {}", e)))?;
        
        encoder.finish()
            .map_err(|e| PersistError::compression(format!("Failed to finish compression: {}", e)))
    }

    fn decompress(&self, compressed_data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = GzDecoder::new(compressed_data);
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| PersistError::compression(format!("Failed to decompress data: {}", e)))?;
        
        Ok(decompressed)
    }

    fn algorithm_name(&self) -> &str {
        "gzip"
    }
}

/// No-compression adapter for testing or when compression is not desired
///
/// This implementation simply passes data through without any compression.
/// Useful for testing or when the data is already compressed.
#[derive(Debug, Clone)]
pub struct NoCompression;

impl NoCompression {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoCompression {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionAdapter for NoCompression {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decompress(&self, compressed_data: &[u8]) -> Result<Vec<u8>> {
        Ok(compressed_data.to_vec())
    }

    fn algorithm_name(&self) -> &str {
        "none"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_compression_roundtrip() {
        let compressor = GzipCompressor::new();
        let original_data = b"This is some test data that should compress well because it has repetitive patterns. ".repeat(10);
        
        // Compress the data
        let compressed = compressor.compress(&original_data).unwrap();
        
        // Compressed data should be smaller for repetitive content
        assert!(compressed.len() < original_data.len());
        
        // Decompress and verify
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(original_data, decompressed);
    }

    #[test]
    fn test_gzip_compression_levels() {
        let test_data = b"Some test data to compress with different levels".repeat(20);
        
        let fast_compressor = GzipCompressor::fast();
        let default_compressor = GzipCompressor::new();
        let max_compressor = GzipCompressor::max();
        
        let fast_compressed = fast_compressor.compress(&test_data).unwrap();
        let default_compressed = default_compressor.compress(&test_data).unwrap();
        let max_compressed = max_compressor.compress(&test_data).unwrap();
        
        // Generally, higher compression levels should produce smaller output
        // (though this isn't guaranteed for all data)
        assert!(max_compressed.len() <= default_compressed.len());
        
        // All should decompress to the same original data
        assert_eq!(fast_compressor.decompress(&fast_compressed).unwrap(), test_data);
        assert_eq!(default_compressor.decompress(&default_compressed).unwrap(), test_data);
        assert_eq!(max_compressor.decompress(&max_compressed).unwrap(), test_data);
    }

    #[test]
    fn test_no_compression() {
        let compressor = NoCompression::new();
        let test_data = b"test data";
        
        let compressed = compressor.compress(test_data).unwrap();
        assert_eq!(compressed, test_data);
        
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, test_data);
        
        assert_eq!(compressor.algorithm_name(), "none");
    }

    #[test]
    fn test_gzip_algorithm_name() {
        let compressor = GzipCompressor::new();
        assert_eq!(compressor.algorithm_name(), "gzip");
    }

    #[test]
    fn test_gzip_empty_data() {
        let compressor = GzipCompressor::new();
        let empty_data = b"";
        
        let compressed = compressor.compress(empty_data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        
        assert_eq!(decompressed, empty_data);
    }

    #[test]
    fn test_gzip_invalid_compressed_data() {
        let compressor = GzipCompressor::new();
        let invalid_data = b"this is not compressed gzip data";
        
        let result = compressor.decompress(invalid_data);
        assert!(result.is_err());
    }
}
