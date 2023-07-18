/// Constants used in the headers messages module (e.g. message sizes, magic_bytes, etc.)
pub mod header_constants {
    pub const HEADER_SIZE: usize = 24;
    pub const _START_STRING: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];
    pub const START_STRING_SIZE: usize = 4;
    pub const COMMAND_NAME_SIZE: usize = 12;
    pub const PAYLOAD_SIZE: usize = 4;
    pub const CHECKSUM_SIZE: usize = 4;
    pub const MAX_HEADER: usize = 2000;
}

/// Constants used in messages module (e.g. getheaders message, gedata message, etc.)
pub mod messages {
    pub const _MAX_INV_SIZE: usize = 50000;
    pub const MAX_PAYLOAD_SIZE: u32 = 500 * 1024 * 1024; // 500 MB
}

/// Constants with all valid commands in the bitcoin protocol in str format
pub mod commands {
    pub const ADDR: &str = "addr\0\0\0\0\0\0\0\0";
    pub const BLOCK: &str = "block\0\0\0\0\0\0\0";
    pub const FEEFILTER: &str = "feefilter\0\0\0";
    pub const GETDATA: &str = "getdata\0\0\0\0\0";
    pub const GETHEADERS: &str = "getheaders\0\0";
    pub const HEADERS: &str = "headers\0\0\0\0\0";
    pub const INV: &str = "inv\0\0\0\0\0\0\0\0\0";
    pub const NO_COMMAND: &str = "no_command\0\0";
    pub const PING: &str = "ping\0\0\0\0\0\0\0\0";
    pub const PONG: &str = "pong\0\0\0\0\0\0\0\0";
    pub const SENDCMPCT: &str = "sendcmpct\0\0\0";
    pub const SENDHEADERS: &str = "sendheaders\0";
    pub const TX: &str = "tx\0\0\0\0\0\0\0\0\0\0";
    pub const VERACK: &str = "verack\0\0\0\0\0\0";
    pub const VERSION: &str = "version\0\0\0\0\0";
    pub const NOTFOUND: &str = "notfound\0\0\0\0";
}

/// Constants with accepted version which is latest version
pub mod version_constants {
    pub const LATEST_VERSION: i32 = 70015;
}

/// Constants with all config parameters
pub mod config {
    // set possible values for log verbosity level
    pub const QUIET: &str = "QUIET";
    pub const VERBOSE: &str = "VERBOSE";
    // set default values for config, overriden by config files
    pub const LOG_FILE: &str = "tmp/node.log";
    pub const HEADERS_FILE: &str = "tmp/headers_backup.dat";
    pub const BLOCKS_FILE: &str = "tmp/blocks_backup.dat";
    pub const TCP_TIMEOUT: u64 = 30;
    pub const START_TIMESTAMP: u32 = 1681095600;
    pub const LOCALHOST: &str = "127.0.0.1";
    pub const LOCALSERVER: &str = "127.0.0.1:8333";
    pub const PORT: u16 = 8333;
    pub const MAGIC: [u8; 4] = [0x0b, 0x11, 0x09, 0x07];
}
