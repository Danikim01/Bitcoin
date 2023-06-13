use rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::io;

use crate::{
    raw_transaction::{Outpoint, RawTransaction, TxInput, TxInputType, TxOutput},
    utxo::UtxoSet,
};

#[derive(PartialEq, Debug)]
pub struct Wallet {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
}

fn public_key_to_string(public_key: &PublicKey) -> String {
    let serialized_key = public_key.serialize();
    let hex_chars: Vec<String> = serialized_key
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect();
    hex_chars.join("")
}

impl Wallet {
    pub fn new() -> Self {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        Self {
            secret_key,
            public_key,
        }
    }

    fn read_wallet_balance(utxo_set: &UtxoSet) -> u64 {
        let mut balance = 0;

        for (_, inner_map) in utxo_set.iter() {
            for (_, transaction) in inner_map.iter() {
                balance += transaction._value;
            }
        }

        balance as u64
    }

    fn get_index(utxo_id: &[u8; 32], vec_transactions: &Vec<RawTransaction>) -> Option<u32> {
        for transaction in vec_transactions {
            match &transaction.tx_in {
                TxInputType::TxInput(tx_inputs) => {
                    for tx_input in tx_inputs.iter() {
                        if tx_input.previous_output._hash == *utxo_id {
                            return Some(tx_input.previous_output._index);
                        }
                    }
                }
                _ => continue,
            }
        }
        None
    }

    fn get_script_sig(
        utxo_id: &[u8; 32],
        vec_transactions: &Vec<RawTransaction>,
    ) -> Option<Vec<u8>> {
        for transaction in vec_transactions {
            match &transaction.tx_in {
                TxInputType::TxInput(tx_inputs) => {
                    for tx_input in tx_inputs.iter() {
                        if tx_input.previous_output._hash == *utxo_id {
                            return Some(tx_input.script_sig.clone());
                        }
                    }
                }
                _ => continue,
            }
        }
        None
    }

    pub fn fill_txins(
        utxo_set: &mut UtxoSet,
        amount: u64,
        balance: &mut u64,
        txin: &mut Vec<TxInput>,
        vec_transactions: &Vec<RawTransaction>,
    ) {
        for (address, inner_map) in utxo_set.iter_mut() {
            for (utxo_id, transaction) in inner_map.iter_mut() {
                if *balance >= amount {
                    break;
                }
                let mut _index = 0;
                let mut _script_sig = Vec::new();
                match Self::get_index(utxo_id, vec_transactions) {
                    Some(index) => {
                        let _index = index;
                    }
                    None => {
                        continue;
                    }
                }

                match Self::get_script_sig(utxo_id, vec_transactions) {
                    Some(script_sig) => {
                        let _script_sig = script_sig;
                    }
                    None => {
                        continue;
                    }
                }

                if !transaction._spent {
                    let tx_input = TxInput {
                        previous_output: Outpoint {
                            _hash: *utxo_id,
                            _index: _index, // Set the appropriate index value
                        },
                        script_sig: _script_sig.clone(), // Set the appropriate script signature
                        script_bytes: _script_sig.len() as u64, // Set the appropriate value
                        sequence: 0,                     // Set the appropriate value
                    };

                    txin.push(tx_input);
                    *balance += transaction._value as u64;
                }
                //mark utxo transaction as spent
                transaction._spent = true;
            }
        }
    }

    pub fn generate_transaction(
        &self,
        utxo_set: &mut UtxoSet,
        recv_addr: PublicKey,
        amount: u64,
        vec_transactions: &Vec<RawTransaction>,
    ) -> io::Result<RawTransaction> {
        if Self::read_wallet_balance(&utxo_set) < amount {
            return Err(io::Error::new(io::ErrorKind::Other, "Not enough funds"));
        }

        let mut txin: Vec<TxInput> = Vec::new();
        let mut balance = 0;

        // Iterar sobre el UtxoSet
        Self::fill_txins(utxo_set, amount, &mut balance, &mut txin, vec_transactions);

        let txout: Vec<TxOutput> = vec![TxOutput {
            value: 0 as i64,
            pk_script_bytes: 0 as u64,
            pk_script: vec![0; 1],
        }];

        //  the first txout is destined for the receiver
        //  the other txout is our "change"

        Ok(RawTransaction {
            version: 1,
            tx_in_count: txin.len() as u64,
            tx_in: TxInputType::TxInput(txin),
            tx_out_count: txout.len() as u64,
            tx_out: txout,
            lock_time: 0 as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_wallet() {
        let my_wallet = Wallet::new();
        println!("Public key: {}", my_wallet.public_key);

        let new_public = PublicKey::from_secret_key(&Secp256k1::new(), &my_wallet.secret_key);
        println!("Public key: {}", new_public);
    }
}
