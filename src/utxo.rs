use crate::raw_transaction::{RawTransaction, TxOutput};
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
    transactions: Vec<UtxoTransaction>,
}

impl Utxo {
    pub fn _from_raw_transaction(raw_transaction: &RawTransaction) -> io::Result<Utxo> {

        let mut utxo = Utxo {
            transactions: Vec::new(),
        };

        for tx_output in &raw_transaction.tx_out {
            let utxo_transaction = UtxoTransaction::_from_tx_output(tx_output)?;
            utxo.transactions.push(utxo_transaction);
        }
        Ok(utxo)
    }

    /// Validate that the transaction of index in txid can be spent
    /// and mark it as spent
    pub fn _validate_spend(&self, index: usize) -> io::Result<()> {
        // first check that it exists
        if index >= self.transactions.len() {
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
