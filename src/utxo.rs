use crate::raw_transaction::{PkScriptData, RawTransaction, TxOutput};
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    pub id: UtxoId,
    _pk_address: String,
    _satoshi_value: i64,
}

pub type UtxoId = [u8; 20];

impl Utxo {
    pub fn _new(id: [u8; 20], pk_address: String, satoshi_value: i64) -> Self {
        Self {
            id,
            _pk_address: pk_address,
            _satoshi_value: satoshi_value,
        }
    }

    pub fn from_txoutput(tx_output: &TxOutput) -> io::Result<Self> {
        let pk_script_data: PkScriptData = tx_output._get_pk_script_data()?;

        Ok(Self {
            id: pk_script_data.pk_hash,
            _pk_address: "addr".to_string(),
            _satoshi_value: tx_output.value,
        })
    }

    pub fn _from_raw_transaction(
        utxoset: &mut HashMap<UtxoId, Utxo>,
        raw_transaction: &RawTransaction,
    ) -> io::Result<()> {
        for tx_output in &raw_transaction.tx_out {
            let utxo = Utxo::from_txoutput(tx_output)?;
            utxoset.insert(utxo.id, utxo);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utxosetappend_transaction() {
        let mut utxo_set = HashMap::new();

        let id1: [u8; 20] = vec![1; 20].try_into().unwrap();
        let transaction1 = Utxo::_new(id1, "pk_adress1".to_string(), 100);
        utxo_set.insert(id1, transaction1);

        let id2: [u8; 20] = vec![2; 20].try_into().unwrap();
        let transaction2 = Utxo::_new(id2, "pk_adress2".to_string(), 150);
        utxo_set.insert(id2, transaction2);

        let id3: [u8; 20] = vec![3; 20].try_into().unwrap();
        let transaction3 = Utxo::_new(id3, "pk_adress3".to_string(), 200);
        utxo_set.insert(id3, transaction3);

        assert_eq!(utxo_set.len(), 3);
    }

    #[test]
    fn test_utxout_remove_transaction() {
        let mut utxo_set = HashMap::new();

        let id1: [u8; 20] = vec![1; 20].try_into().unwrap();
        let transaction1 = Utxo::_new(id1, "pk_adress1".to_string(), 100);
        utxo_set.insert(id1, transaction1);

        let id2: [u8; 20] = vec![2; 20].try_into().unwrap();
        let transaction2 = Utxo::_new(id2, "pk_adress2".to_string(), 150);
        utxo_set.insert(id2, transaction2);

        let id3: [u8; 20] = vec![3; 20].try_into().unwrap();
        let transaction3 = Utxo::_new(id3, "pk_adress3".to_string(), 200);
        utxo_set.insert(id3, transaction3);

        utxo_set.remove(&id2);
        assert_eq!(utxo_set.len(), 2);
    }

    #[test]
    fn test_utxout_remove_invalid_trasnsaction() {
        let mut utxo_set = HashMap::new();

        let id1: [u8; 20] = vec![1; 20].try_into().unwrap();
        let transaction1 = Utxo::_new(id1, "pk_adress1".to_string(), 100);
        utxo_set.insert(id1, transaction1);

        let id2: [u8; 20] = vec![2; 20].try_into().unwrap();
        let transaction2 = Utxo::_new(id2, "pk_adress2".to_string(), 150);
        utxo_set.insert(id2, transaction2);

        let id3: [u8; 20] = vec![3; 20].try_into().unwrap();
        let transaction3 = Utxo::_new(id3, "pk_adress3".to_string(), 200);
        utxo_set.insert(id3, transaction3);

        let id4: [u8; 20] = vec![4; 20].try_into().unwrap();
        assert_eq!(utxo_set.remove(&id4), None);
    }
}
