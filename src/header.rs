//https://developer.bitcoin.org/reference/block_chain.html#block-headers
pub struct BlockHeader {
    version: i32,
    prev_blockhash: [u8; 32],
    merkle_root: [u8; 32],
    time: u32,
    nbits: u32,
    nonce: u32,
}

//https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
//https://developer.bitcoin.org/reference/p2p_networking.html#getheaders
pub struct Header <T>{
    count: T, //compactSize uint
    header: BlockHeader,
}

impl BlockHeader{
    fn new(version:i32,prev_blockhash:[u8; 32],merkle_root:[u8; 32],time:u32,nbits:u32,nonce:u32)->Self{
        Self{
            version,
            prev_blockhash,
            merkle_root,
            time,
            nbits,
            nonce,
        }
    }
}

impl <T>Header<T>{
    fn new(count:T,header:BlockHeader)->Self{
        Self{
            count,
            header,
        }
    }
}
