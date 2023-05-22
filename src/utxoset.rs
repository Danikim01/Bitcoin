use crate::raw_transaction::{PkScriptData, RawTransaction, TxOutput};

use std::io::Error;

pub struct _Utxo {
    pub id: [u8; 20],
    pk_address: String,
    satoshi_value: i64,
}

impl _Utxo {
    pub fn _new(id: [u8; 20], pk_address: String, satoshi_value: i64) -> Self {
        Self {
            id,
            pk_address,
            satoshi_value,
        }
    }

    fn _from_txoutput(tx_output: &TxOutput) -> Result<Self, Error> {
        let pk_script_data: PkScriptData = tx_output._get_pk_script_data()?;

        Ok(Self {
            id: pk_script_data.pk_hash,
            pk_address: "addr".to_string(),
            satoshi_value: tx_output.value,
        })
    }

    pub fn _from_raw_transaction(
        utxoset: &mut _UTXOset,
        raw_transaction: &RawTransaction,
    ) -> Result<(), Error> {
        for tx_output in &raw_transaction.tx_out {
            let utxo = _Utxo::_from_txoutput(tx_output)?;
            utxoset._append(utxo)?;
        }

        Ok(())
    }
}

pub struct _UTXOset {
    pub utxo_vector: Vec<_Utxo>,
}

impl _UTXOset {
    pub fn _new() -> Self {
        Self {
            utxo_vector: Vec::new(),
        }
    }

    pub fn _append(&mut self, transaction: _Utxo) -> Result<(), Error> {
        self.utxo_vector.push(transaction);
        Ok(())
    }

    pub fn _try_remove(&mut self, transaction_id: [u8; 20]) -> Result<(), Error> {
        // get index of transaction
        let index = self.utxo_vector.iter().position(|x| x.id == transaction_id);

        if let Some(index) = index {
            self.utxo_vector.remove(index);
            return Ok(());
        }

        Err(Error::new(
            std::io::ErrorKind::NotFound,
            "Transaction not found",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utxoset_append_transaction() {
        let mut utxo_set = _UTXOset::_new();

        let id1: [u8; 20] = vec![1; 20].try_into().unwrap();
        let transaction1 = _Utxo::_new(id1, "pk_adress1".to_string(), 100);
        utxo_set._append(transaction1).unwrap();

        let id2: [u8; 20] = vec![2; 20].try_into().unwrap();
        let transaction2 = _Utxo::_new(id2, "pk_adress2".to_string(), 150);
        utxo_set._append(transaction2).unwrap();

        let id3: [u8; 20] = vec![3; 20].try_into().unwrap();
        let transaction3 = _Utxo::_new(id3, "pk_adress3".to_string(), 200);
        utxo_set._append(transaction3).unwrap();

        assert_eq!(utxo_set.utxo_vector.len(), 3);
    }

    #[test]
    fn test_utxout_remove_transaction() {
        let mut utxo_set = _UTXOset::_new();

        let id1: [u8; 20] = vec![1; 20].try_into().unwrap();
        let transaction1 = _Utxo::_new(id1, "pk_adress1".to_string(), 100);
        utxo_set._append(transaction1).unwrap();

        let id2: [u8; 20] = vec![2; 20].try_into().unwrap();
        let transaction2 = _Utxo::_new(id2, "pk_adress2".to_string(), 150);
        utxo_set._append(transaction2).unwrap();

        let id3: [u8; 20] = vec![3; 20].try_into().unwrap();
        let transaction3 = _Utxo::_new(id3, "pk_adress3".to_string(), 200);
        utxo_set._append(transaction3).unwrap();

        utxo_set._try_remove(id2).unwrap();
        assert_eq!(utxo_set.utxo_vector.len(), 2);
    }

    #[test]
    fn test_utxout_remove_invalid_trasnsaction() {
        let mut utxo_set = _UTXOset::_new();

        let id1: [u8; 20] = vec![1; 20].try_into().unwrap();
        let transaction1 = _Utxo::_new(id1, "pk_adress1".to_string(), 100);
        utxo_set._append(transaction1).unwrap();

        let id2: [u8; 20] = vec![2; 20].try_into().unwrap();
        let transaction2 = _Utxo::_new(id2, "pk_adress2".to_string(), 150);
        utxo_set._append(transaction2).unwrap();

        let id3: [u8; 20] = vec![3; 20].try_into().unwrap();
        let transaction3 = _Utxo::_new(id3, "pk_adress3".to_string(), 200);
        utxo_set._append(transaction3).unwrap();

        let id4: [u8; 20] = vec![4; 20].try_into().unwrap();
        assert!(utxo_set._try_remove(id4).is_err());
    }
}
