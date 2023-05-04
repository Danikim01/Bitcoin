#[derive(Debug)]
pub struct GetBlocks {
    version: u32,
    hash_count: u8,
    block_header_hashes: Vec<[u8;32]>,
    stop_hash: [u8;32],
}

//Default for genesis block
impl Default for GetBlocks {
    fn default() -> Self {
        let version = 70015;

        GetBlocks::new(
            70015,
            1,
            vec![[0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63, 0xf7, 0x4f,
                0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00]], //genesis hash
            [0_u8; 32], //til max block hashes (500 is MAX for response)
        )

    }
}