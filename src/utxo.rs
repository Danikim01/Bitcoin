use crate::raw_transaction::generate_txid_vout_bytes;
use crate::raw_transaction::{tx_output::TxOutput, RawTransaction};
use crate::utility::double_hash;
use std::collections::HashMap;
use std::io::Cursor;
use std::io::{self, Read};

pub type UtxoId = [u8; 32];
pub type WalletUtxo = HashMap<UtxoId, UtxoTransaction>;
type Address = String;
pub type UtxoSpent = Vec<[u8; 36]>;

#[derive(Debug, Clone)]
pub struct UtxoSet {
    pub set: HashMap<Address, WalletUtxo>,
    pub spent: UtxoSpent,
}

impl UtxoSet {
    pub fn new() -> Self {
        Self {
            set: HashMap::new(),
            spent: vec![],
        }
    }

    fn utxo_spent(&self, txid: &[u8; 32], utxo: &UtxoTransaction) -> bool {
        let vout = utxo.index.to_le_bytes();
        self.spent.contains(&generate_txid_vout_bytes(txid.clone(), vout))
    }

    /// returns available utxos for a given address
    pub fn get_wallet_available_utxos(&self, address: &str) -> io::Result<Vec<(UtxoId, UtxoTransaction)>> {
        let mut available_utxos: Vec<(UtxoId, UtxoTransaction)> = Vec::new();

        if let Some(utxos) = self.set.get(address) {
            for (txid, utxo_transaction) in utxos {
                if !self.utxo_spent(txid, utxo_transaction) {
                    available_utxos.push((txid.clone(), utxo_transaction.clone()));
                }
            }
        }

        Ok(available_utxos)
    }

    pub fn get_wallet_balance(&self, address: &str) -> io::Result<u64> {
        let mut balance = 0;

        if let Some(utxos) = self.set.get(address) {
            for (txid, utxo_transaction) in utxos {
                if !self.utxo_spent(txid, utxo_transaction) {
                    balance += utxo_transaction.value as u64;
                }
            }
        }

        Ok(balance)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoTransaction {
    pub index: u32,
    pub value: u64,
    _lock: Vec<u8>,
}

pub fn p2pkh_to_address(p2pkh: [u8; 20]) -> String {
    let version_prefix: [u8; 1] = [0x6f];

    let hash = double_hash(&[&version_prefix[..], &p2pkh[..]].concat());

    let checksum = &hash[..4];

    let input = [&version_prefix[..], &p2pkh[..], checksum].concat();

    bs58::encode(input).into_string()
}

impl UtxoTransaction {
    fn _has_wallet(&self, address: &str) -> io::Result<bool> {
        // iterate lock one byte at a time until 0x14 is found
        let mut cursor = Cursor::new(self._lock.clone());

        let buf = &mut [0; 1];
        while buf[0] != 0x14 {
            cursor.read_exact(buf)?;
        }

        let mut pk_hash = [0; 20];
        cursor.read_exact(&mut pk_hash)?;

        let pk2addr = p2pkh_to_address(pk_hash);
        Ok(pk2addr == address)
    }

    pub fn get_address(&self) -> io::Result<String> {
        // iterate lock one byte at a time until 0x14 is found
        let mut cursor = Cursor::new(self._lock.clone());

        let buf = &mut [0; 1];
        while buf[0] != 0x14 {
            cursor.read_exact(buf)?;
        }

        let mut pk_hash = [0; 20];
        cursor.read_exact(&mut pk_hash)?;

        Ok(p2pkh_to_address(pk_hash))
    }

    pub fn from_tx_output(tx_output: &TxOutput, index: u32) -> io::Result<Self> {
        let value = tx_output.value;
        let lock = tx_output.pk_script.clone();
        Ok(Self {
            index,
            value,
            _lock: lock,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    pub transactions: Vec<UtxoTransaction>,
}

impl Utxo {
    pub fn from_raw_transaction(raw_transaction: &RawTransaction) -> io::Result<Utxo> {
        let mut utxo = Utxo {
            transactions: Vec::new(),
        };

        let mut index = 0;
        for tx_output in &raw_transaction.tx_out {
            let utxo_transaction = UtxoTransaction::from_tx_output(tx_output, index)?;
            utxo.transactions.push(utxo_transaction);
            index += 1;
        }
        Ok(utxo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_address_test_from_p2pkh() {
        let p2pkh: [u8; 20] = [
            0x7a, 0xa8, 0x18, 0x46, 0x85, 0xca, 0x1f, 0x06, 0xf5, 0x43, 0xb6, 0x4a, 0x50, 0x2e,
            0xb3, 0xb6, 0x13, 0x5d, 0x67, 0x20,
        ];
        let actual = p2pkh_to_address(p2pkh);
        let expected = "mrhW6tcF2LDetj3kJvaDTvatrVxNK64NXk".to_string();
        assert_eq!(actual, expected)
    }
}
