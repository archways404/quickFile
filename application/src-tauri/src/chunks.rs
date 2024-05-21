// src/chunks.rs

/// Splits the given data into chunks of the specified size.
///
/// # Arguments
///
/// * `data` - A vector of bytes to be split into chunks.
/// * `chunk_size` - The size of each chunk in bytes.
///
/// # Returns
///
/// * A vector of vectors, where each inner vector is a chunk of the original data.
pub fn split_into_chunks(data: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
    data.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_into_chunks() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let chunk_size = 3;
        let chunks = split_into_chunks(&data, chunk_size);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![1, 2, 3]);
        assert_eq!(chunks[1], vec![4, 5, 6]);
        assert_eq!(chunks[2], vec![7, 8, 9]);
    }
}
