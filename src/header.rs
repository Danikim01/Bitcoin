//https://developer.bitcoin.org/reference/block_chain.html#block-headers
pub struct BlockHeader {
    version: i32,
    prev_blockhash: [u8; 32],
    merkle_root: [u8; 32],
    time: u32,
    nbits: u32,
    nonce: u32,
}

enum CompactSize {
    OneByte(u8),
    TwoBytes(u16),
    FourBytes(u32),
    EightBytes(u64),
}

//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
pub struct Header {
    count: CompactSize, //compactSize uint
    headers: BlockHeader,
}
