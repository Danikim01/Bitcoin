pub struct GetBlocks {
    version: u32,
    hash_count: u8,
    block_header_hashes: Vec<[char;32]>,
    stop_hash: [char;32],
}