use crate::raw_transaction::{RawTransaction, TxOutput};
use std::collections::HashMap;
use std::io;

pub type UtxoSet = HashMap<UtxoId, Utxo>;

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

    // TODO: add desired pk_adress as parameter
    pub fn _get_wallet_balance(&self) -> i64 {
        // if desired pk_adress is the same as the adress held
        // and the transaction is not spent, return the value
        0
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

    // TODO: add desired pk_adress as parameter
    pub fn _get_wallet_balance(&self) -> i64 {
        let mut balance = 0;
        for transaction in &self.transactions {
            balance += transaction._get_wallet_balance();
        }
        balance
    }
}

// ADD TESTING
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_get_pk_address_balance() {
//         // create mock utxo set

//         // create mock pk_address

//         // assert_eq!(utxo.get_pk_address_balance(pk_address), expected_balance);
//     }
// }
