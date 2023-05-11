
pub mod header_constants{
    pub const HEADER_SIZE: usize = 24;
    pub const START_STRING: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];
    pub const START_STRING_SIZE: usize = 4;
    pub const COMMAND_NAME_SIZE: usize = 12;
    pub const PAYLOAD_SIZE: usize = 4;
    pub const CHECKSUM_SIZE: usize = 4;
}

pub mod message_constants{

}