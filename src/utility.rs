use crate::messages::{HashId, Hashable};
use std::collections::HashMap;
use std::fmt::Display;
use std::io;

/// Splits a vector evenly among a fixed number of vectors or "buckets".
/// The first bucket would have all elements with index `0 + n*amount_of_buckets``
/// and so on.
///
/// # Example
///
/// ```
/// let initial_vec = vec![1,2,3,4,5,6,7];
/// let buckets = to_buckets(initial_vec, 3);
/// assert_eq!(buckets, vec![vec![1,4,7], vec![2,5], vec![3,6]]);
/// ```
pub fn to_buckets<T>(initial_vector: Vec<T>, amount_of_buckets: usize) -> Vec<Vec<T>> {
    // pub fn to_buckets<T>(initial_vector: Vec<T>, amount_of_buckets: usize) -> Vec<Vec<T>> {
    let mut buckets: Vec<Vec<T>> = vec![];
    for _ in 0..amount_of_buckets {
        let bucket: Vec<T> = vec![];
        buckets.push(bucket);
    }
    if initial_vector.len() < 1 || amount_of_buckets < 1 {
        return buckets;
    }
    let mut bucket_idx = 0;
    for element in initial_vector {
        bucket_idx += 1;
        if bucket_idx >= amount_of_buckets {
            bucket_idx = 0;
        }
        buckets[bucket_idx].push(element)
    }
    buckets
}

pub fn to_io_err<E>(error: E) -> io::Error
where
    E: Display,
{
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

pub fn into_hashmap<T>(elements: Vec<T>) -> HashMap<HashId, T>
where
    T: Hashable,
{
    let hashmap: HashMap<HashId, T> = elements
        .into_iter()
        .map(|element| (element.hash(), element))
        .collect();
    hashmap
}
