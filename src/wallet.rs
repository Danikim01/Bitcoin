use crate::{
    raw_transaction::{
        generate_txid_vout_bytes,
        tx_input::{Outpoint, TxInput, TxInputType},
        tx_output::TxOutput,
        RawTransaction,
    },
    utxo::UtxoTransaction,
};
use crate::{
    utility::{decode_hex, to_io_err},
    utxo::{UtxoSet, WalletUtxo},
};
use bitcoin_hashes::{hash160, ripemd160, sha256, Hash};
use rand::rngs::OsRng;
use secp256k1::Secp256k1;
use std::{io, str::FromStr};

#[derive(PartialEq, Debug)]
pub struct Wallet {
    pub secret_key: String,
    pub address: String,
}

// fn address_to_string(address: &PublicKey) -> String {
//     let serialized_key = address.serialize();
//     let hex_chars: Vec<String> = serialized_key
//         .iter()
//         .map(|byte| format!("{:02x}", byte))
//         .collect();
//     hex_chars.join("")
// }

//ver https://developer.bitcoin.org/devguide/wallets.html#public-key-formats
fn hash_address(address: String) -> io::Result<Vec<u8>> {
    let serialized_key = decode_hex(&address).map_err(to_io_err)?;
    let raw_bytes = if serialized_key[0] == 0x04 {
        // Uncompressed public key
        &serialized_key[1..]
    } else if serialized_key[0] == 0x03 || serialized_key[0] == 0x02 {
        // Compressed public key
        &serialized_key[0..]
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Invalid public key format",
        ));
    };

    // First hash with SHA256
    let sha_hash = sha256::Hash::hash(&raw_bytes).to_byte_array();

    // Second hash with RIPEMD160
    let ripemd_hash = ripemd160::Hash::hash(&sha_hash);

    Ok(ripemd_hash.to_byte_array().to_vec())
}

fn build_p2pkh_script(hashed_pk: Vec<u8>) -> Vec<u8> {
    let mut pk_script = Vec::new();
    pk_script.push(0x76); // OP_DUP
    pk_script.push(0xa9); // OP_HASH160
    pk_script.push(0x14); // Push 20 bytes
    pk_script.extend_from_slice(&hashed_pk);
    pk_script.push(0x88); // OP_EQUALVERIFY
    pk_script.push(0xac); // OP_CHECKSIG
    pk_script
}

impl Wallet {
    pub fn login() -> Self {
        let secret_key = "cVMDbb3HdL5Bo8hirbAjNnKgKPCcdU9vFmnKasQX3zSvXgCkbbFi".to_string();
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX".to_string();
        // let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";
        // let address = "mpTmaREX6juSwdcVGPyVx74GxWJ4AKQX3u";
        Self {
            secret_key,
            address,
        }
    }

    pub fn new() -> Self {
        let secp = Secp256k1::new();
        let (sk, addr) = secp.generate_keypair(&mut OsRng);
        Self {
            secret_key: format!("{}", sk.display_secret()),
            address: format!("{}", addr),
        }
    }

    /*fn get_index(utxo_id: &[u8; 32], vec_transactions: &Vec<RawTransaction>) -> Option<u32> {
        for transaction in vec_transactions {
            match &transaction.tx_in {
                TxInputType::TxInput(tx_inputs) => {
                    for tx_input in tx_inputs.iter() {
                        if tx_input.previous_output.hash == *utxo_id {
                            return Some(tx_input.previous_output._index);
                        }
                    }
                }
                _ => continue,
            }
        }
        None
    }*/

    /*fn get_script_sig(
        utxo_id: &[u8; 32],
        vec_transactions: &Vec<RawTransaction>,
    ) -> Option<Vec<u8>> {
        for transaction in vec_transactions {
            match &transaction.tx_in {
                TxInputType::TxInput(tx_inputs) => {
                    for tx_input in tx_inputs.iter() {
                        if tx_input.previous_output.hash == *utxo_id {
                            return Some(tx_input.script_sig.clone());
                        }
                    }
                }
                _ => continue,
            }
        }
        None
    }*/

    pub fn fill_txins(
        &self,
        utxo_set: &mut UtxoSet,
        amount: u64,
    ) -> io::Result<(Vec<TxInput>, u64)> {
        let mut used_utxos: Vec<UtxoTransaction> = Vec::new();
        let mut used_balance: u64 = 0;

        // get available utxos
        let available_utxos = utxo_set.get_wallet_available_utxos(&self.address)?;

        // iterate over them until used balance is enough
        for utxo in available_utxos {
            used_utxos.push(utxo.clone());
            used_balance += utxo.value;
            if used_balance >= amount {
                break;
            }
        }

        // mark used utxos as spent
        for utxo in used_utxos.iter() {
            let txid = [0_u8; 32]; // TODO: COMPLETE THIS
            let vout = utxo.index.to_le_bytes();
            utxo_set.spent.push(generate_txid_vout_bytes(txid, vout))
        }

        // build txins
        let mut txin: Vec<TxInput> = Vec::new();
        // TODO: COMPLETE THIS

        // return used utxos and used balance
        Ok((txin, used_balance))
    }

    // WARNINNNNNNNNNNNNNNNNNNNNNNNNNG: GENERATE UTXO SNAPSHOT AND USE IT INSTEAD OF THE UTXOSET
    // CHANGE UTXOSET TO UTXOSNAPSHOT ONCE EVERYTHING IS OK 😊
    pub fn generate_transaction(
        &self,
        utxo_set: &mut UtxoSet,
        recv_addr: String,
        amount: u64,
    ) -> io::Result<RawTransaction> {
        if utxo_set.get_wallet_balance(&self.address)? < amount {
            return Err(io::Error::new(io::ErrorKind::Other, "Not enough funds"));
        }

        let (txin, used_balance) = self.fill_txins(utxo_set, amount)?;

        let mut txout: Vec<TxOutput> = Vec::new();

        //  the first txout is destined for the receiver
        //  build the P2PKH locking script
        let pk_script = build_p2pkh_script(hash_address(recv_addr)?);
        txout.push(TxOutput {
            value: amount,
            pk_script_bytes: pk_script.len() as u64,
            pk_script: pk_script.clone(),
        });

        //  the other txout is our "change"
        txout.push(TxOutput {
            value: used_balance - amount,
            pk_script_bytes: pk_script.len() as u64,
            pk_script,
        });

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
    use crate::raw_transaction::RawTransaction;

    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_login() {
        let res = Wallet::login();
        println!("{:?}", res);
    }

    #[test]
    fn create_wallet() {
        let my_wallet = Wallet::new();
        println!("Wallet: {:?}", my_wallet);
    }

    #[test]
    fn test_read_wallet_balance() {
        let mut utxo_set: UtxoSet = UtxoSet::new();
        let my_wallet = Wallet::login();

        let transaction_bytes = decode_hex(
            "020000000001011216d10ae3afe6119529c0a01abe7833641e0e9d37eb880ae5547cfb7c6c7bca0000000000fdffffff0246b31b00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac731f2001020000001976a914d617966c3f29cfe50f7d9278dd3e460e3f084b7b88ac02473044022059570681a773748425ddd56156f6af3a0a781a33ae3c42c74fafd6cc2bd0acbc02200c4512c250f88653fae4d73e0cab419fa2ead01d6ba1c54edee69e15c1618638012103e7d8e9b09533ae390d0db3ad53cc050a54f89a987094bffac260f25912885b834b2c2500"
        ).unwrap();
        let transaction = RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes)).unwrap();
        transaction.generate_utxo(&mut utxo_set).unwrap();

        let balance = utxo_set.get_wallet_balance(&my_wallet.address).unwrap();
        assert_eq!(balance, 1815366)
    }

    #[test]
    fn test_generate_raw_transaction() {
        let wallet = Wallet::login();
        let mut utxo_set: UtxoSet = UtxoSet::new();

        // this transactions should give enough balance to send 1 tBTC
        let transaction_bytes = decode_hex(
            "020000000001011216d10ae3afe6119529c0a01abe7833641e0e9d37eb880ae5547cfb7c6c7bca0000000000fdffffff0246b31b00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac731f2001020000001976a914d617966c3f29cfe50f7d9278dd3e460e3f084b7b88ac02473044022059570681a773748425ddd56156f6af3a0a781a33ae3c42c74fafd6cc2bd0acbc02200c4512c250f88653fae4d73e0cab419fa2ead01d6ba1c54edee69e15c1618638012103e7d8e9b09533ae390d0db3ad53cc050a54f89a987094bffac260f25912885b834b2c2500"
        ).unwrap();
        let transaction = RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes)).unwrap();
        transaction.generate_utxo(&mut utxo_set).unwrap();

        let recvr_addr = "1JQheacLPdM5ySCkrZkV66G2ApAXe1mqLj".to_string();
        let raw_transaction = wallet
            .generate_transaction(&mut utxo_set, recvr_addr, 1)
            .unwrap();

        let bytes = raw_transaction.serialize();
        let res = RawTransaction::from_bytes(&mut Cursor::new(&bytes));
        assert!(res.is_ok());
        // EL TEST NO ES SOLO QUE PASE ESTO, LUEGO HAY QUE DESEAREALIZARLA Y VER QUE ESTE TODO BIEN
    }
}
