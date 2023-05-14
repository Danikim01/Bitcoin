use crate::io::Cursor;
use crate::messages::utility::*;
use std::io::Read;

#[derive(Debug)]
pub struct RawTransaction {
    pub version: i32, //Puede ser 1 o 2
    pub tx_in_count: usize,
    pub tx_in: Vec<TxInput>,
    pub tx_out_count: usize,
    pub tx_out: Vec<TxOutput>,
    pub lock_time: u32,
}
#[derive(Debug)]
pub struct Outpoint {
    pub hash: [u8; 32],
    pub index: u32,
}
#[derive(Debug)]
pub struct TxInput {
    pub previous_output: Outpoint,
    pub script_bytes: usize,
    pub signature_script: Vec<u8>,
    pub sequence: u32,
}
#[derive(Debug)]
pub struct TxOutput {
    pub value: i64,
    pub pk_script_bytes: usize,
    pub pk_script: Vec<u8>,
}

//Ver https://developer.bitcoin.org/reference/transactions.html#raw-transaction-format
impl RawTransaction {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<(), std::io::Error> {
        //Read RawTransaction
        let version = read_i32(cursor)?;
        println!("the version is {:?}", &version);
        let tx_in_count = read_from_varint(cursor)? as usize;
        println!("the txn in count is {:?}", &tx_in_count);

        let mut tx_in = Vec::with_capacity(tx_in_count);

        for _ in 0..tx_in_count {
            //read previous_output
            let prev_output = Outpoint {
                hash: read_hash(cursor)?,
                index: read_u32(cursor)?,
            };
            println!("El hash es {:?}", &prev_output.hash);
            println!("El index es {}", &prev_output.index);
            //read script bytes
            let script_bytes = read_from_varint(cursor)? as usize;
            println!("El script bytes es {}", &script_bytes);

            let mut signature_script = vec![0u8; script_bytes];
            cursor.read_exact(&mut signature_script)?;
            println!("El signature script bytes es {:?}", &signature_script);

            let sequence = read_u32(cursor)?;
            println!("El sequence es {}", &sequence);
            let input = TxInput {
                previous_output: prev_output,
                script_bytes: script_bytes,
                signature_script: signature_script,
                sequence: sequence,
            };
            tx_in.push(input);
        }

        let tx_out_count = read_from_varint(cursor)? as usize;
        println!("El tx out count es {}", &tx_out_count);

        let mut tx_out = Vec::with_capacity(tx_out_count);
        for _ in 0..tx_out_count {
            let value = read_i64(cursor)?;
            println!("El value es {}", &value);
            let pk_script_bytes = read_from_varint(cursor)? as usize;
            println!("El pk_script_bytes es {}", &pk_script_bytes);
            let mut pk_script = vec![0u8; pk_script_bytes];
            cursor.read_exact(&mut pk_script)?;
            println!("El pk_script es {:?}", &pk_script);

            let output = TxOutput {
                value: value,
                pk_script_bytes: pk_script_bytes,
                pk_script: pk_script,
            };
            tx_out.push(output);
        }

        let lock_time = read_u32(cursor)?;
        println!("El lock_time es {}", &lock_time);

        // let actual_raw_txn = RawTransaction {
        //     version: version,
        //     tx_in_count: tx_in_count,
        //     tx_in: tx_in,
        //     tx_out_count: tx_out_count,
        //     tx_out: tx_out,
        //     lock_time: lock_time,
        // };
        Ok(())
    }
}
