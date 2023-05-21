use std::fmt::Display;
use std::io;

pub fn to_max_len_buckets<T>(initial_vector: Vec<T>, max_bucket_size: usize) -> Vec<Vec<T>> {
    let mut buckets: Vec<Vec<T>> = vec![];
    if initial_vector.len() < 1 || max_bucket_size < 1 {
        return buckets;
    }
    let mut current_bucket: Vec<T> = vec![];

    for element in initial_vector {
        if current_bucket.len() >= max_bucket_size {
            buckets.push(current_bucket);
            current_bucket = vec![];
        }
        current_bucket.push(element);
    }
    buckets.push(current_bucket);
    buckets
}

/// Splits a vector evenly among a fixed number of vectors or "chunks".
/// The first chunk would have all elements with index `0 + n*amount_of_chunks``
/// and so on.
///
/// # Example
///
/// ```
/// let initial_vec = vec![1,2,3,4,5,6,7];
/// let chunks = to_n_chunks(initial_vec, 3);
/// assert_eq!(chunks, vec![vec![1,4,7], vec![2,5], vec![3,6]]);
/// ```
// impl<'a, T> Iterator for Iter<'a, T>
pub fn to_n_chunks<I, T>(initial_vector: I, amount_of_chunks: usize) -> Vec<Vec<T>> where I: std::iter::Iterator<Item = T> + ExactSizeIterator, T: Copy {
// pub fn to_n_chunks<T>(initial_vector: Vec<T>, amount_of_chunks: usize) -> Vec<Vec<T>> {
    let mut chunks: Vec<Vec<T>> = vec![];
    for _ in 0..amount_of_chunks {
        let chunk: Vec<T> = vec![];
        chunks.push(chunk);
    }
    if initial_vector.len() < 1 || amount_of_chunks < 1 {
        return chunks;
    }
    let mut chunk_idx = 0;
    for element in initial_vector {
        chunk_idx += 1;
        if chunk_idx >= amount_of_chunks {
            chunk_idx = 0;
        }
        chunks[chunk_idx].push(element)
    }
    chunks
}

pub fn to_io_err<E>(error: E) -> io::Error where E: Display {
    io::Error::new(io::ErrorKind::Other, error.to_string())
}
