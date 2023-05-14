pub mod header_constants {
    pub const HEADER_SIZE: usize = 24;
    pub const START_STRING: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];
    pub const START_STRING_SIZE: usize = 4;
    pub const COMMAND_NAME_SIZE: usize = 12;
    pub const PAYLOAD_SIZE: usize = 4;
    pub const CHECKSUM_SIZE: usize = 4;
}

pub mod commands {
    pub const VERACK: &str = "verack\0\0\0\0\0\0";
    pub const HEADER: &str = "headers\0\0\0\0\0";
    pub const UNKNOWN: &str = "no_command\0\0";
}

pub mod version_constants {
    pub const LATEST_VERSION: i32 = 70015;
}

pub mod config {
    pub const PATH: &str = "src/initial_config.txt";
    pub const PORT: u16 = 8333;
}
