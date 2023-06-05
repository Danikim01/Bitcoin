pub mod header_constants {
    pub const HEADER_SIZE: usize = 24;
    pub const _START_STRING: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];
    pub const START_STRING_SIZE: usize = 4;
    pub const COMMAND_NAME_SIZE: usize = 12;
    pub const PAYLOAD_SIZE: usize = 4;
    pub const CHECKSUM_SIZE: usize = 4;
    pub const MAX_HEADER: usize = 2000;
}

pub mod messages {
    use super::super::HashId;
    pub const _MAX_INV_SIZE: usize = 50000;
    pub const MAX_PAYLOAD_SIZE: u32 = 500 * 1024 * 1024; // 500 MB
    pub const GENESIS_HASHID: HashId = [
        0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72, 0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63, 0xf7,
        0x4f, 0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c, 0x68, 0xd6, 0x19, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ];
}

pub mod commands {
    pub const GETHEADERS: &str = "getheaders\0\0";
    pub const GETDATA: &str = "getdata\0\0\0\0\0";
    pub const BLOCK: &str = "block\0\0\0\0\0\0\0";
    pub const VERSION: &str = "version\0\0\0\0\0";
    pub const VERACK: &str = "verack\0\0\0\0\0\0";
    pub const HEADERS: &str = "headers\0\0\0\0\0";
    pub const UNKNOWN: &str = "no_command\0\0";
    pub const SENDCMPCT: &str = "sendcmpct\0\0\0";
    pub const SENDHEADERS: &str = "sendheaders\0";
    pub const PING: &str = "ping\0\0\0\0\0\0\0\0";
    pub const FEEFILTER: &str = "feefilter\0\0\0";
    pub const ADDR: &str = "addr\0\0\0\0\0\0\0\0";
    pub const INV: &str = "inv\0\0\0\0\0\0\0\0\0";
}

pub mod version_constants {
    pub const LATEST_VERSION: i32 = 70015;
}

pub mod config {
    pub const PATH: &str = "src/initial_config.txt";
    pub const PORT: u16 = 8333;
    pub const VERBOSE: &str = "VERBOSE";
    pub const QUIET: &str = "QUIET";
}
