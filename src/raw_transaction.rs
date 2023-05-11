#[derive(Debug)]
pub struct RawTransaction {
    version: i32, //Puede ser 1 o 2
    tx_in_count:usize,
    tx_in: Vec<TxInput>,
    tx_out_count:usize,
    tx_out: Vec<TxOutput>,
    lock_time: u32,
}
#[derive(Debug)]
struct Outpoint{
    hash:[u8;32],
    index:u32,
}
#[derive(Debug)]
struct TxInput {
    previous_output: Outpoint,
    script_bytes:usize,
    signature_script: Vec<u8>,
    sequence:u32,
}
#[derive(Debug)]
struct TxOutput {
    value: i64,
    pk_script_bytes: usize,
    pk_script:Vec<u8>,
}