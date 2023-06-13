use bitcoin_hashes::{ripemd160, sha256, Hash};
use rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::{io, str::FromStr};

use crate::{
    raw_transaction::{Outpoint, RawTransaction, TxInput, TxInputType, TxOutput},
    utility::{_decode_hex, to_io_err, _encode_hex},
    utxo::{UtxoSet, WalletUtxo},
};

#[derive(PartialEq, Debug)]
pub struct Wallet {
    pub secret_key: SecretKey,
    pub address: PublicKey,
}

fn address_to_string(address: &PublicKey) -> String {
    let serialized_key = address.serialize();
    let hex_chars: Vec<String> = serialized_key
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect();
    hex_chars.join("")
}

impl Wallet {
    pub fn login() -> io::Result<Self> {
        let secret_key =
            SecretKey::from_str("cVMDbb3HdL5Bo8hirbAjNnKgKPCcdU9vFmnKasQX3zSvXgCkbbFi")
                .map_err(to_io_err)?;
        let address =
            PublicKey::from_str("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX").map_err(to_io_err)?;
        Ok(Self {
            secret_key,
            address,
        })
    }

    pub fn new() -> Self {
        let secp = Secp256k1::new();
        let (secret_key, address) = secp.generate_keypair(&mut OsRng);
        Self {
            secret_key,
            address,
        }
    }

    pub fn get_balance(&self, utxo_set: &UtxoSet) -> u64 {
        let mut balance = 0;

        let k = _encode_hex(&self.address.serialize());
        let wallet_utxo: WalletUtxo = utxo_set.0.get(&k).unwrap().clone();
        for utxo in wallet_utxo {
            if utxo_set.1.contains(&utxo.0) {
                continue;
            }
            balance += utxo.1._value as u64;
        }
        
        balance as u64
    }

    // fn get_index(utxo_id: &[u8; 32], vec_transactions: &Vec<RawTransaction>) -> Option<u32> {
    //     for transaction in vec_transactions {
    //         match &transaction.tx_in {
    //             TxInputType::TxInput(tx_inputs) => {
    //                 for tx_input in tx_inputs.iter() {
    //                     if tx_input.previous_output.hash == *utxo_id {
    //                         return Some(tx_input.previous_output._index);
    //                     }
    //                 }
    //             }
    //             _ => continue,
    //         }
    //     }
    //     None
    // }

    // fn get_script_sig(
    //     utxo_id: &[u8; 32],
    //     vec_transactions: &Vec<RawTransaction>,
    // ) -> Option<Vec<u8>> {
    //     for transaction in vec_transactions {
    //         match &transaction.tx_in {
    //             TxInputType::TxInput(tx_inputs) => {
    //                 for tx_input in tx_inputs.iter() {
    //                     if tx_input.previous_output.hash == *utxo_id {
    //                         return Some(tx_input.script_sig.clone());
    //                     }
    //                 }
    //             }
    //             _ => continue,
    //         }
    //     }
    //     None
    // }

    // pub fn fill_txins(
    //     utxo_set: &mut UtxoSet,
    //     amount: u64,
    //     balance: &mut u64,
    //     txin: &mut Vec<TxInput>,
    //     vec_transactions: &Vec<RawTransaction>,
    // ) {
    //     for (address, inner_map) in utxo_set.0.iter_mut() {
    //         for (utxo_id, transaction) in inner_map.iter_mut() {
    //             if *balance >= amount {
    //                 break;
    //             }
    //             let mut _index = 0;
    //             let mut _script_sig = Vec::new();
    //             match Self::get_index(utxo_id, vec_transactions) {
    //                 Some(index) => {
    //                     let _index = index;
    //                 }
    //                 None => {
    //                     continue;
    //                 }
    //             }

    //             match Self::get_script_sig(utxo_id, vec_transactions) {
    //                 Some(script_sig) => {
    //                     let _script_sig = script_sig;
    //                 }
    //                 None => {
    //                     continue;
    //                 }
    //             }

    //             if !transaction._spent {
    //                 let tx_input = TxInput {
    //                     previous_output: Outpoint {
    //                         hash: *utxo_id,
    //                         _index: _index, // Set the appropriate index value
    //                     },
    //                     script_sig: _script_sig.clone(), // Set the appropriate script signature
    //                     script_bytes: _script_sig.len() as u64, // Set the appropriate value
    //                     sequence: 0,                     // Set the appropriate value
    //                 };

    //                 txin.push(tx_input);
    //                 *balance += transaction._value as u64;
    //             }
    //             //mark utxo transaction as spent
    //             transaction._spent = true;
    //         }
    //     }
    // }

    // //ver https://developer.bitcoin.org/devguide/wallets.html#public-key-formats
    // fn hash_address(address: &PublicKey) -> io::Result<Vec<u8>> {
    //     let serialized_key = address.serialize();
    //     let raw_bytes = if serialized_key[0] == 0x04 {
    //         // Uncompressed public key
    //         &serialized_key[1..]
    //     } else if serialized_key[0] == 0x03 || serialized_key[0] == 0x02 {
    //         // Compressed public key
    //         &serialized_key[0..]
    //     } else {
    //         return Err(io::Error::new(
    //             io::ErrorKind::Other,
    //             "Invalid public key format",
    //         ));
    //     };

    //     // First hash with SHA256
    //     let sha_hash = sha256::Hash::hash(&raw_bytes).to_byte_array();

    //     // Second hash with RIPEMD160
    //     let ripemd_hash = ripemd160::Hash::hash(&sha_hash);

    //     Ok(ripemd_hash.to_byte_array().to_vec())
    // }

    // pub fn construct_p2pkh_script(hashed_pk: Vec<u8>) -> Vec<u8> {
    //     let mut pk_script = Vec::new();
    //     pk_script.push(0x76); // OP_DUP
    //     pk_script.push(0xa9); // OP_HASH160
    //     pk_script.push(0x14); // Push 20 bytes
    //     pk_script.extend_from_slice(&hashed_pk);
    //     pk_script.push(0x88); // OP_EQUALVERIFY
    //     pk_script.push(0xac); // OP_CHECKSIG
    //     pk_script
    // }

    // pub fn generate_transaction(
    //     &self,
    //     utxo_set: &mut UtxoSet,
    //     recv_addr: PublicKey,
    //     amount: u64,
    //     vec_transactions: &Vec<RawTransaction>,
    // ) -> io::Result<RawTransaction> {
    //     if Self::read_wallet_balance(&utxo_set) < amount {
    //         return Err(io::Error::new(io::ErrorKind::Other, "Not enough funds"));
    //     }

    //     let mut txin: Vec<TxInput> = Vec::new();
    //     let mut balance = 0;

    //     // Iterar sobre el UtxoSet
    //     Self::fill_txins(utxo_set, amount, &mut balance, &mut txin, vec_transactions);

    //     //Construct the P2PKH locking script
    //     let hashed_pk = Self::hash_address(&recv_addr)?;
    //     let pk_script = Self::construct_p2pkh_script(hashed_pk);
    //     let mut txout: Vec<TxOutput> = vec![TxOutput {
    //         value: amount as i64,
    //         pk_script_bytes: pk_script.len() as u64,
    //         pk_script: pk_script.clone(),
    //     }];

    //     let txout_change = TxOutput {
    //         value: (balance as u64 - amount) as i64,
    //         pk_script_bytes: pk_script.len() as u64,
    //         pk_script,
    //     };

    //     txout.push(txout_change);
    //     //  the first txout is destined for the receiver
    //     //  the other txout is our "change"

    //     Ok(RawTransaction {
    //         version: 1,
    //         tx_in_count: txin.len() as u64,
    //         tx_in: TxInputType::TxInput(txin),
    //         tx_out_count: txout.len() as u64,
    //         tx_out: txout,
    //         lock_time: 0 as u32,
    //     })
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_wallet() {
        let my_wallet = Wallet::new();
        println!("Public key: {}", my_wallet.address);

        let new_public = PublicKey::from_secret_key(&Secp256k1::new(), &my_wallet.secret_key);
        println!("Public key: {}", new_public);
    }
}
