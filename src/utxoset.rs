use std::io::Error;
use bitcoin_hashes::{sha256, Hash};
use crate::raw_transaction::{RawTransaction, TxOutput};

pub struct UTXO {
    pub id: [u8; 32],
    pk_address: String,
    satoshi_value: i64,
}

impl UTXO {
    pub fn new(id: [u8; 32], pk_address: String, satoshi_value: i64) -> Self {
        Self {
            id,
            pk_address,
            satoshi_value,
        }
    }

    // TODO: complete
    fn from_txoutput(tx_output: &TxOutput) -> Self {
        Self {
            id: sha256::Hash::hash("foo".as_bytes()).to_byte_array(),
            pk_address: "addr".to_string(),
            satoshi_value: tx_output.value,
        }
    }

    pub fn from_raw_transaction(utxoset: &mut UTXOset, raw_transaction: &RawTransaction) -> Result<(), Error> {        
        for tx_output in &raw_transaction.tx_out {
            let utxo = UTXO::from_txoutput(tx_output);
            utxoset.append(utxo)?;
        }

        Ok(())
    }
}

pub struct UTXOset {
    pub utxo_vector: Vec<UTXO>,
}

impl UTXOset {
    pub fn new() -> Self {
        Self {
            utxo_vector: Vec::new(),
        }
    }

    pub fn append(&mut self, transaction: UTXO) -> Result<(), Error> {
        self.utxo_vector.push(transaction);
        Ok(())
    }

    pub fn try_remove(&mut self, transaction_id: [u8; 32]) -> Result<(), Error> {
        // get index of transaction
        let index = self.utxo_vector.iter().position(|x| x.id == transaction_id);

        if let Some(index) = index {
            self.utxo_vector.remove(index);
            return Ok(());
        }

        return Err(Error::new(
            std::io::ErrorKind::NotFound,
            "Transaction not found",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utxoset_append_transaction() {
        let mut utxo_set = UTXOset::new();

        let id1 = sha256::Hash::hash(b"transaction1").to_byte_array();
        let transaction1 = UTXO::new(id1, "pk_adress1".to_string(), 100);
        utxo_set.append(transaction1).unwrap();

        let id2 = sha256::Hash::hash(b"transaction2").to_byte_array();
        let transaction2 = UTXO::new(id2, "pk_adress2".to_string(), 150);
        utxo_set.append(transaction2).unwrap();

        let id3 = sha256::Hash::hash(b"transaction3").to_byte_array();
        let transaction3 = UTXO::new(id3, "pk_adress3".to_string(), 200);
        utxo_set.append(transaction3).unwrap();

        assert_eq!(utxo_set.utxo_vector.len(), 3);
    }

    #[test]
    fn test_utxout_remove_transaction() {
        let mut utxo_set = UTXOset::new();

        let id1 = sha256::Hash::hash(b"transaction1").to_byte_array();
        let transaction1 = UTXO::new(id1, "pk_adress1".to_string(), 100);
        utxo_set.append(transaction1).unwrap();

        let id2 = sha256::Hash::hash(b"transaction2").to_byte_array();
        let transaction2 = UTXO::new(id2, "pk_adress2".to_string(), 150);
        utxo_set.append(transaction2).unwrap();

        let id3 = sha256::Hash::hash(b"transaction3").to_byte_array();
        let transaction3 = UTXO::new(id3, "pk_adress3".to_string(), 200);
        utxo_set.append(transaction3).unwrap();

        utxo_set.try_remove(id2).unwrap();
        assert_eq!(utxo_set.utxo_vector.len(), 2);
    }

    #[test]
    fn test_utxout_remove_invalid_trasnsaction() {
        let mut utxo_set = UTXOset::new();

        let id1 = sha256::Hash::hash(b"transaction1").to_byte_array();
        let transaction1 = UTXO::new(id1, "pk_adress1".to_string(), 100);
        utxo_set.append(transaction1).unwrap();

        let id2 = sha256::Hash::hash(b"transaction2").to_byte_array();
        let transaction2 = UTXO::new(id2, "pk_adress2".to_string(), 150);
        utxo_set.append(transaction2).unwrap();

        let id3 = sha256::Hash::hash(b"transaction3").to_byte_array();
        let transaction3 = UTXO::new(id3, "pk_adress3".to_string(), 200);
        utxo_set.append(transaction3).unwrap();

        let id4 = sha256::Hash::hash(b"transaction4").to_byte_array();
        assert!(utxo_set.try_remove(id4).is_err());
    }
}
