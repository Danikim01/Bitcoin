use crate::raw_transaction::{RawTransaction, TxOutput};
use bitcoin_hashes::{hash160, Hash};
use std::collections::HashMap;
use std::io::Cursor;
use std::io::{self, Read};

fn _hash_pk_address(pk_address: Vec<u8>) -> [u8; 20] {
    hash160::Hash::hash(&pk_address).to_byte_array()
}

pub type UtxoSet = HashMap<UtxoId, Utxo>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoTransaction {
    _value: i64,
    _lock: Vec<u8>,
    _spent: bool,
}

impl UtxoTransaction {
    fn _has_wallet(&self, pk_address: Vec<u8>) -> io::Result<bool> {
        // iterate lock one byte at a time until 0x14 is found
        let mut cursor = Cursor::new(self._lock.clone());

        let buf = &mut [0; 1];
        while buf[0] != 0x14 {
            cursor.read_exact(buf)?;
        }

        let mut pk_hash = [0; 20];
        cursor.read_exact(&mut pk_hash)?;

        Ok(pk_hash == _hash_pk_address(pk_address))
    }

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
    pub fn _get_wallet_balance(&self, pk_address: Vec<u8>) -> io::Result<i64> {
        // if desired pk_adress is the same as the adress held
        // and the transaction is not spent, return the value
        if self._has_wallet(pk_address)? && !self._spent {
            return Ok(self._value);
        }
        Ok(0)
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
    pub fn _get_wallet_balance(&self, pk_address: Vec<u8>) -> io::Result<i64> {
        let mut balance = 0;
        for transaction in &self.transactions {
            balance += transaction._get_wallet_balance(pk_address.clone())?;
        }
        Ok(balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_pk_address() {
        // https://learnmeabitcoin.com/technical/public-key-hash
        let pk_address: &[u8] = &[
            0x02, 0xb4, 0x63, 0x2d, 0x08, 0x48, 0x5f, 0xf1, 0xdf, 0x2d, 0xb5, 0x5b, 0x9d, 0xaf,
            0xd2, 0x33, 0x47, 0xd1, 0xc4, 0x7a, 0x45, 0x70, 0x72, 0xa1, 0xe8, 0x7b, 0xe2, 0x68,
            0x96, 0x54, 0x9a, 0x87, 0x37,
        ];

        let expected_pk_hash: &[u8] = &[
            0x93, 0xce, 0x48, 0x57, 0x0b, 0x55, 0xc4, 0x2c, 0x2a, 0xf8, 0x16, 0xae, 0xab, 0xa0,
            0x6c, 0xfe, 0xe1, 0x22, 0x4f, 0xae,
        ];

        let pk_hash = _hash_pk_address(pk_address.to_vec());
        assert_eq!(pk_hash, expected_pk_hash)
    }

    #[test]
    fn test_utxo_transaction_get_pk_address_balance() {
        let lock_bytes: &[u8] = &[
            0x14, // push 20 bytes as data
            0x93, 0xce, 0x48, 0x57, 0x0b, 0x55, 0xc4, 0x2c, 0x2a, 0xf8, 0x16, 0xae, 0xab, 0xa0,
            0x6c, 0xfe, 0xe1, 0x22, 0x4f, 0xae, // Public key hash
        ];

        let expected_value = 100;

        let utxo_transaction = UtxoTransaction {
            _value: expected_value,
            _lock: lock_bytes.to_vec(),
            _spent: false,
        };

        let pk_address: &[u8] = &[
            0x02, 0xb4, 0x63, 0x2d, 0x08, 0x48, 0x5f, 0xf1, 0xdf, 0x2d, 0xb5, 0x5b, 0x9d, 0xaf,
            0xd2, 0x33, 0x47, 0xd1, 0xc4, 0x7a, 0x45, 0x70, 0x72, 0xa1, 0xe8, 0x7b, 0xe2, 0x68,
            0x96, 0x54, 0x9a, 0x87, 0x37,
        ];

        let actual_value = utxo_transaction
            ._get_wallet_balance(pk_address.to_vec())
            .unwrap();
        assert_eq!(actual_value, expected_value);
    }

    #[test]
    fn test_utxo_get_pk_address_balance() {
        let lock_bytes: &[u8] = &[
            0x14, // push 20 bytes as data
            0x93, 0xce, 0x48, 0x57, 0x0b, 0x55, 0xc4, 0x2c, 0x2a, 0xf8, 0x16, 0xae, 0xab, 0xa0,
            0x6c, 0xfe, 0xe1, 0x22, 0x4f, 0xae, // Public key hash
        ];

        let lock_other_bytes: &[u8] = &[
            0x14, // push 20 bytes as data
            0xff, 0xff, 0x48, 0x57, 0x0b, 0x55, 0xc4, 0x2c, 0x2a, 0xf8, 0x16, 0xae, 0xab, 0xa0,
            0x6c, 0xfe, 0xe1, 0x22, 0x4f, 0xae, // Public key hash
        ];

        let val1 = 100;
        let val2 = 200;
        let expected_value = val1 + val2;

        let utxo_transaction1 = UtxoTransaction {
            _value: val1,
            _lock: lock_bytes.to_vec(),
            _spent: false,
        };

        let utxo_transaction2 = UtxoTransaction {
            _value: 150,
            _lock: lock_other_bytes.to_vec(),
            _spent: false,
        };

        let utxo_transaction3 = UtxoTransaction {
            _value: val2,
            _lock: lock_bytes.to_vec(),
            _spent: false,
        };

        let utxo_transaction4 = UtxoTransaction {
            _value: 150,
            _lock: lock_bytes.to_vec(),
            _spent: true,
        };

        let utxo = Utxo {
            transactions: vec![
                utxo_transaction1,
                utxo_transaction2,
                utxo_transaction3,
                utxo_transaction4,
            ],
        };

        let pk_address: &[u8] = &[
            0x02, 0xb4, 0x63, 0x2d, 0x08, 0x48, 0x5f, 0xf1, 0xdf, 0x2d, 0xb5, 0x5b, 0x9d, 0xaf,
            0xd2, 0x33, 0x47, 0xd1, 0xc4, 0x7a, 0x45, 0x70, 0x72, 0xa1, 0xe8, 0x7b, 0xe2, 0x68,
            0x96, 0x54, 0x9a, 0x87, 0x37,
        ];

        let actual_value = utxo._get_wallet_balance(pk_address.to_vec()).unwrap();
        assert_eq!(actual_value, expected_value);
    }
}
