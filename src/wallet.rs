use crate::utxo::UtxoId;
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
    utility::{decode_hex, double_hash, to_io_err},
    utxo::{UtxoSet, WalletUtxo},
};
use bitcoin_hashes::{hash160, ripemd160, sha256, Hash};
use rand::rngs::OsRng;
use secp256k1::{Message, Secp256k1, SecretKey};
use std::{io, str::FromStr};

//ver https://developer.bitcoin.org/devguide/wallets.html#public-key-formats
fn hash_address(address: String) -> io::Result<Vec<u8>> {
    let serialized_key = address.as_bytes();

    let version_prefix: [u8; 1] = [0x6f];
    let hash = double_hash(&[&version_prefix[..], &serialized_key[..]].concat());
    let checksum = &hash[..4];

    let input = [&version_prefix[..], &serialized_key[..], checksum].concat();
    let base58 = bs58::encode(input).into_string().as_bytes().to_vec();
    Ok(base58)
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

#[derive(PartialEq, Debug)]
pub struct Wallet {
    pub secret_key: String,
    pub address: String,
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

    // https://learnmeabitcoin.com/technical/p2sh
    fn build_script_sig(&self, recv_addr: String) -> io::Result<Vec<u8>> {
        let mut script_sig = Vec::new();

        Ok(script_sig)
    }

    fn fill_txins(
        &self,
        utxo_set: &mut UtxoSet,
        amount: u64,
        recv_addr: String,
    ) -> io::Result<(Vec<TxInput>, u64)> {
        // get available utxos
        let available_utxos: Vec<(UtxoId, UtxoTransaction)> =
            utxo_set.get_wallet_available_utxos(&self.address)?;

        // iterate over them until used balance is enough
        let mut used_utxos: Vec<(UtxoId, UtxoTransaction)> = Vec::new();
        let mut used_balance: u64 = 0;
        for (utxo_id, utxo) in available_utxos {
            used_balance += utxo.value;
            used_utxos.push((utxo_id, utxo));
            if used_balance >= amount {
                break;
            }
        }

        // build txins and mark utxos as spent
        let mut txins: Vec<TxInput> = Vec::new();
        for (utxo_id, utxo) in used_utxos {
            let vout = utxo.index.to_le_bytes();
            utxo_set.spent.push(generate_txid_vout_bytes(utxo_id, vout));

            // let hashed_pk = hash_address(utxo.get_address()?)?;
            let script_sig = self.build_script_sig(recv_addr.clone())?;
            let txin = TxInput {
                previous_output: Outpoint {
                    hash: utxo_id,
                    index: utxo.index,
                },
                script_bytes: script_sig.len() as u64,
                script_sig,
                sequence: 0xffffffff,
            };
            txins.push(txin);
        }

        // return used utxos and used balance
        Ok((txins, used_balance))
    }

    fn fill_txouts(
        &self,
        amount: u64,
        used_balance: u64,
        recv_addr: String,
    ) -> io::Result<Vec<TxOutput>> {
        let mut txout: Vec<TxOutput> = Vec::new();

        //  the first txout is destined for the receiver
        let recv_hashed_pk = hash_address(recv_addr.clone())?;
        let pk_script = build_p2pkh_script(recv_hashed_pk);
        txout.push(TxOutput {
            value: amount,
            pk_script_bytes: pk_script.len() as u64,
            pk_script,
        });

        //  the other txout is our "change"
        let self_hashed_pk = hash_address(self.address.clone())?;
        let pk_script = build_p2pkh_script(self_hashed_pk);
        txout.push(TxOutput {
            value: used_balance - amount,
            pk_script_bytes: pk_script.len() as u64,
            pk_script,
        });

        Ok(txout)
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

        let (txin, used_balance) = self.fill_txins(utxo_set, amount, recv_addr.clone())?;
        let txout = self.fill_txouts(amount, used_balance, recv_addr)?;

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
    use crate::{raw_transaction::RawTransaction, utility::_encode_hex};

    use super::*;
    use crate::messages::utility::read_hash;
    use bs58::decode;
    use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
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
        let wallet: Wallet = Wallet::login();
        let mut utxo_set: UtxoSet = UtxoSet::new();

        // this transactions should give enough balance to send 1 tBTC
        let transaction_bytes = decode_hex(
            "020000000001011216d10ae3afe6119529c0a01abe7833641e0e9d37eb880ae5547cfb7c6c7bca0000000000fdffffff0246b31b00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac731f2001020000001976a914d617966c3f29cfe50f7d9278dd3e460e3f084b7b88ac02473044022059570681a773748425ddd56156f6af3a0a781a33ae3c42c74fafd6cc2bd0acbc02200c4512c250f88653fae4d73e0cab419fa2ead01d6ba1c54edee69e15c1618638012103e7d8e9b09533ae390d0db3ad53cc050a54f89a987094bffac260f25912885b834b2c2500"
        ).unwrap();
        let transaction = RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes)).unwrap();
        transaction.generate_utxo(&mut utxo_set).unwrap();

        let recvr_addr = "mnJvq7mbGiPNNhUne4FAqq27Q8xZrAsVun".to_string();
        let raw_transaction = wallet
            .generate_transaction(&mut utxo_set, recvr_addr, 1)
            .unwrap();

        let bytes = raw_transaction.serialize();
        let res = RawTransaction::from_bytes(&mut Cursor::new(&bytes)).unwrap();

        assert_eq!(res.tx_in_count, 1);
        assert_eq!(res.tx_out_count, 2);
        assert_eq!(res.tx_out[0].value, 1);
        assert_eq!(res.tx_out[1].value, 1815365);
        // EL TEST NO ES SOLO QUE PASE ESTO, LUEGO HAY QUE DESEAREALIZARLA Y VER QUE ESTE TODO BIEN

        println!("{}", _encode_hex(&bytes))
    }

    fn decode_base58(s: &str) -> Result<Vec<u8>, bs58::decode::Error> {
        let bytes = decode(s);
        Ok(bytes.into_vec().unwrap())
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

    fn sign_transaction(
        transaction: &mut RawTransaction,
        private_key: &SecretKey,
        public_key: &PublicKey,
    ) {
        let secp: Secp256k1<secp256k1::All> = Secp256k1::gen_new();
        //this should be the same as z = transaction.sig_hash(0)??
        let z = transaction.serialize();
        let hash = double_hash(&z);

        // Sign the hash with the private key
        let message = hash.to_byte_array();
        let mut message_slice: &[u8] = &message;
        let message_slice = Message::from_slice(message_slice).unwrap();

        let mut signature = secp.sign_ecdsa(&message_slice, &private_key);
        assert_eq!(
            secp.verify_ecdsa(&message_slice, &signature, &public_key),
            Ok(())
        );

        // Convert the DER-encoded signature to bytes
        let der = signature.serialize_der().to_vec();
        let sig = [&der[..], &[0x01]].concat(); // Append SIGHASH_ALL (0x01) byte

        // Get the SEC format public key
        let public_key = PublicKey::from_secret_key(&secp, private_key);
        let sec = public_key.serialize();

        // Create the script_sig using the signature and public key
        let script_sig = [&sig[..], &sec[..]].concat();

        // Set the script_sig of the transaction input
        if let TxInputType::TxInput(inputs) = &mut transaction.tx_in {
            if let Some(input) = inputs.first_mut() {
                input.script_bytes = script_sig.len() as u64;
                input.script_sig = script_sig;
            }
        }
    }

    #[test]
    fn test_generate_raw_transaction2() {
        let bytes =
            decode_hex("0d6fe5213c0b3291f208cba8bfb59b7476dffacc4e5cb66f6eb20a080843a299").unwrap();

        let previous_output_hash = read_hash(&mut Cursor::new(&bytes)).unwrap();
        let prev_index = 13;
        let prev_output = Outpoint {
            hash: previous_output_hash,
            index: prev_index,
        };

        let tx_in = TxInput {
            previous_output: prev_output,
            script_bytes: 0,
            script_sig: Vec::new(),
            sequence: 0xffffffff,
        };

        let my_addr = "mzx5YhAH9kNHtcN481u6WkjeHjYtVeKVh2".to_string();
        let change_h160 = decode_base58(&my_addr).unwrap();
        let change_h160_hash = &change_h160[1..21]; // Extract the 20-byte hash

        let change_amount = 0.33 * 100_000_000.0;
        let change_script = build_p2pkh_script(change_h160_hash.to_vec());
        let change_output = TxOutput {
            value: change_amount as u64,
            pk_script_bytes: change_script.len() as u64,
            pk_script: change_script,
        };

        let recvr_addr = "mnrVtF8DWjMu839VW3rBfgYaAfKk8983Xf".to_string();
        let decoded_address = decode_base58(&recvr_addr).unwrap();
        let hash_bytes = &decoded_address[1..21]; // Extract the 20-byte hash

        let target_amount = 0.1 * 100_000_000.0;
        let target_h160 = decoded_address.clone();
        let target_script = build_p2pkh_script(hash_bytes.to_vec());
        let target_output = TxOutput {
            value: target_amount as u64,
            pk_script_bytes: target_script.len() as u64,
            pk_script: target_script,
        };

        let mut tx_obj = RawTransaction {
            version: 1,
            tx_in_count: 1,
            tx_in: TxInputType::TxInput(vec![tx_in]),
            tx_out_count: 2,
            tx_out: vec![change_output, target_output],
            lock_time: 0,
        };

        let private_key_bytes: [u8; 32] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x21, 0x34, 0x12, 0x22,
            0x3C, 0x11, 0x00, 0x23, 0x12, 0x15, 0x01, 0x00, 0x13, 0x20, 0x00, 0x01, 0x00, 0x32,
            0x21, 0x31, 0x11, 0x01,
        ];

        let private_key = SecretKey::from_slice(&private_key_bytes).unwrap();
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &private_key);

        sign_transaction(&mut tx_obj, &private_key, &public_key);

        //println!("{:?}", tx_obj);
        let tx_bytes = tx_obj.serialize();
        println!("{}", _encode_hex(&tx_bytes));
    }
}
