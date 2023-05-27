use crate::raw_transaction::{RawTransaction, TxOutput};
use crate::utility::double_hash;
use bitcoin_hashes::Hash;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoTransaction {
    _value: i64,
    _lock: Vec<u8>,
    _spent: bool,
}

impl UtxoTransaction {
    pub fn _from_tx_output(tx_output: &TxOutput) -> io::Result<Self> {
        let value = tx_output.value;
        let lock = tx_output.pk_script.clone();
        Ok(Self {
            _value: value,
            _lock: lock,
            _spent: false,
        })
    }
}

pub type UtxoId = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    pub txid: UtxoId,
    transaction: Vec<UtxoTransaction>,
}

impl Utxo {
    pub fn _from_raw_transaction(raw_transaction: &RawTransaction) -> io::Result<Utxo> {
        let txid = double_hash(&raw_transaction.serialize()).to_byte_array();

        let mut utxo = Utxo {
            txid,
            transaction: Vec::new(),
        };

        for tx_output in &raw_transaction.tx_out {
            let utxo_transaction = UtxoTransaction::_from_tx_output(tx_output)?;
            utxo.transaction.push(utxo_transaction);
        }
        Ok(utxo)
    }

    /// Validate that the transaction of index in txid can be spent
    /// and mark it as spent
    pub fn _validate_spend(&self, index: usize) -> io::Result<()> {
        // first check that it exists
        if index >= self.transaction.len() {
            println!("Utxo index out of bounds!");
            // return Err(io::Error::new(
            //     io::ErrorKind::InvalidInput,
            //     "Index out of bounds",
            // ));
        }

        // then check the lock (research how to do this)

        Ok(())
    }
}

// ADD TESTING
// #[cfg(test)]
// mod tests {    
//     use super::*;
// }
