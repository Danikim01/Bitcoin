pub fn bucket_vec<T>(initial_vector: Vec<T>, max_bucket_size: usize) -> Vec<Vec<T>> {
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
