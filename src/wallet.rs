use crate::config::Config;
use crate::interface::components::overview_panel::TransactionDisplayInfo;
use crate::interface::components::send_panel::TransactionInfo;
use crate::interface::GtkMessage;
use crate::messages::HashId;
use crate::raw_transaction::TransactionOrigin;
use crate::raw_transaction::{
    tx_input::{Outpoint, TxInput, TxInputType},
    tx_output::TxOutput,
    RawTransaction,
};
use crate::utility::{double_hash, to_io_err};
use crate::utxo::{Lock, UtxoSet, UtxoTransaction};
use bitcoin_hashes::{hash160, Hash};
use gtk::glib::SyncSender;
use rand::rngs::OsRng;
use secp256k1::{Secp256k1, SecretKey};
use std::collections::HashMap;
use std::io;
use std::str::FromStr;

fn hash_address(address: &str) -> io::Result<Vec<u8>> {
    let bytes = bs58::decode(address).into_vec().map_err(to_io_err)?;
    Ok(bytes)
}

fn build_p2pkh_script(hashed_pk: Vec<u8>) -> io::Result<Vec<u8>> {
    if hashed_pk.len() < 21 {
        return Err(io::Error::new(io::ErrorKind::Other, "Invalid address"));
    }

    let hpk = hashed_pk[1..21].to_vec();
    let mut pk_script = Vec::new();
    pk_script.push(0x76); // OP_DUP
    pk_script.push(0xa9); // OP_HASH160
    pk_script.push(0x14); // Push 20 bytes
    pk_script.extend_from_slice(&hpk);
    pk_script.push(0x88); // OP_EQUALVERIFY
    pk_script.push(0xac); // OP_CHECKSIG
    Ok(pk_script)
}

/// The Wallet struct is responsible for managing the wallet's secret key and address can be used to send transactions.
#[derive(PartialEq, Debug, Clone)]
pub struct Wallet {
    pub secret_key: SecretKey,
    pub address: String,
    pub history: Vec<TransactionDisplayInfo>,
}

impl Wallet {
    pub fn get_last_n_transactions(&self, n: usize) -> Vec<TransactionDisplayInfo> {
        let mut history = self.history.clone();
        history.reverse();
        history.truncate(n);
        history.reverse();
        history
    }

    pub fn update_history(
        &mut self,
        transaction_info: TransactionDisplayInfo,
        origin: TransactionOrigin,
    ) {
        if origin == TransactionOrigin::Block {
            // try to remove the pending transaction from the history
            // completely not optimal as it could be a hashmap
            for (i, tx) in self.history.iter().enumerate() {
                if tx.hash == transaction_info.hash {
                    self.history[i] = transaction_info;
                    return;
                }
            }
        }
        self.history.push(transaction_info); // pending tx get here yet they don't show on the ui
    }

    fn get_address_from_secret_key(secret_key: &SecretKey) -> String {
        let secp = Secp256k1::new();
        let pubkey = secret_key.public_key(&secp).serialize();
        let h160 = hash160::Hash::hash(&pubkey).to_byte_array();
        let version_prefix: [u8; 1] = [0x6f];
        let hash = double_hash(&[&version_prefix[..], &h160[..]].concat());
        let checksum = &hash[..4];
        let input = [&version_prefix[..], &h160[..], checksum].concat();
        bs58::encode(input).into_string()
    }

    /// Show wallet address in the UI
    pub fn display_in_ui(wallet: &String, ui_sender: Option<&SyncSender<GtkMessage>>) {
        if let Some(sender) = ui_sender {
            let _ = sender
                .send(GtkMessage::UpdateLabel((
                    "active_address_label".to_string(),
                    wallet.clone(),
                )))
                .map_err(to_io_err);
        }
    }

    /// Creates a new Wallet with a random secret key and address.
    pub fn new() -> Self {
        let secp = Secp256k1::new();
        let (sk, _addr) = secp.generate_keypair(&mut OsRng);
        Self {
            secret_key: sk,
            address: Self::get_address_from_secret_key(&sk),
            history: Vec::new(),
        }
    }

    /// Iterates all files in the wallet directory
    /// returns the hashmap of wallets with their addresses as keys
    /// and the first wallet as the default wallet
    pub fn init_all(config: &Config) -> io::Result<(String, HashMap<String, Wallet>)> {
        let mut wallets: HashMap<String, Wallet> = HashMap::new();

        // read wallets from directory
        let wallets_dir = config.get_wallets_dir();
        let dir = std::fs::read_dir(wallets_dir)?;
        for entry in dir {
            if let Ok(file) = entry {
                let path_string = match file.path().to_str() {
                    Some(p) => p.to_string(),
                    None => continue,
                };
                let wallet = Config::wallet_from_file(path_string)?;
                if let Some(w) = wallet {
                    wallets.insert(w.address.clone(), w);
                }
            }
        }

        // if wallets is still empty, create a new wallet

        // set active address to default specified in config, or the first wallet found
        let active_wallet = match wallets.get_key_value(&config.get_default_wallet_addr()) {
            Some((k, _)) => k.clone(),
            None => {
                // make any wallet the active wallet
                let foo = wallets.iter().next();
                match foo {
                    Some((k, _)) => k.clone(),
                    None => {
                        // return error, this should never happen
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Could not find any wallets",
                        ));
                    }
                }
            }
        };

        Ok((active_wallet, wallets))
    }

    fn fill_needed(
        amount: u64,
        available_utxos: Vec<(HashId, UtxoTransaction)>,
    ) -> (Vec<(HashId, UtxoTransaction, Lock)>, u64) {
        let mut used_utxos: Vec<(HashId, UtxoTransaction, Lock)> = Vec::new();
        let mut used_balance: u64 = 0;
        for (utxo_id, utxo) in available_utxos {
            used_balance += utxo.value;
            let lock = utxo.lock.clone();
            used_utxos.push((utxo_id, utxo, lock));
            if used_balance >= amount {
                break;
            }
        }

        (used_utxos, used_balance)
    }

    fn fill_txins(
        &self,
        utxo_set: &mut UtxoSet,
        amount: u64,
    ) -> io::Result<(Vec<TxInput>, u64, Vec<Lock>)> {
        // get available utxos
        let available_utxos: Vec<(HashId, UtxoTransaction)> =
            utxo_set.get_wallet_available_utxos(&self.address);

        if available_utxos.is_empty() {
            return Err(io::Error::new(io::ErrorKind::Other, "No available utxos"));
        }

        let (used_utxos, used_balance) = Self::fill_needed(amount, available_utxos);

        // build txins
        let mut txins: Vec<TxInput> = Vec::new();
        let mut locks: Vec<Lock> = Vec::new();
        for (utxo_id, utxo, lock) in used_utxos {
            let txin = TxInput {
                previous_output: Outpoint {
                    hash: utxo_id,
                    index: utxo.index,
                },
                script_bytes: 0,
                script_sig: Vec::new(),
                sequence: 0xffffffff,
            };
            txins.push(txin);
            locks.push(lock);
        }

        // return used utxos and used balance
        Ok((txins, used_balance, locks))
    }

    fn fill_txouts(
        &self,
        amount: u64,
        used_balance: u64,
        transaction_info: TransactionInfo,
    ) -> io::Result<Vec<TxOutput>> {
        let mut txout: Vec<TxOutput> = Vec::new();

        //  the first txout is destined for the receiver
        for (recv_addr, _label, spec_amount) in transaction_info.recipients {
            let recv_hashed_pk = hash_address(&recv_addr)?;
            let first_pk_script = build_p2pkh_script(recv_hashed_pk)?;
            txout.push(TxOutput {
                value: spec_amount,
                pk_script_bytes: first_pk_script.len() as u64,
                pk_script: first_pk_script,
            });
        }
        //  the last txout is our "change"
        let self_hashed_pk = hash_address(&self.address)?;
        let second_pk_script = build_p2pkh_script(self_hashed_pk)?;
        let value: u64 = match used_balance > (amount + transaction_info.fee) {
            true => used_balance - amount - transaction_info.fee,
            false => 0,
        };
        txout.push(TxOutput {
            value,
            pk_script_bytes: second_pk_script.len() as u64,
            pk_script: second_pk_script,
        });

        Ok(txout)
    }

    /// Generates a transaction from the wallet's utxos, filling the transaction with the given transaction info.
    /// Removes the used utxos from the utxo set.
    /// If the wallet does not have enough funds, returns an error.
    pub fn generate_transaction(
        &self,
        utxo_set: &mut UtxoSet,
        transaction_info: TransactionInfo,
    ) -> io::Result<RawTransaction> {
        let secp = Secp256k1::new();
        let amount = transaction_info
            .recipients
            .iter()
            .fold(0, |acc, x| acc + x.2);

        if utxo_set.get_wallet_balance(&self.address) <= amount {
            return Err(io::Error::new(io::ErrorKind::Other, "Not enough funds"));
        }

        let (txin, used_balance, locks) =
            self.fill_txins(utxo_set, transaction_info.fee + amount)?;
        let txout = self.fill_txouts(amount, used_balance, transaction_info)?;
        let mut transaction = RawTransaction {
            version: 1,
            tx_in_count: txin.len() as u64,
            tx_in: TxInputType::TxInput(txin.clone()),
            tx_out_count: txout.len() as u64,
            tx_out: txout,
            lock_time: 0,
        };

        for (index, _) in locks.iter().enumerate().take(txin.len()) {
            let prev_pk_script = locks[index].clone();
            transaction.sign_input(&secp, &self.secret_key, prev_pk_script, index)?;
        }
        Ok(transaction)
    }
}

impl TryFrom<&str> for Wallet {
    type Error = io::Error;
    fn try_from(secret_key: &str) -> io::Result<Wallet> {
        let key = SecretKey::from_str(secret_key).map_err(to_io_err)?;
        Ok(Self {
            secret_key: key,
            address: Self::get_address_from_secret_key(&key),
            history: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        raw_transaction::{RawTransaction, TransactionOrigin},
        utility::{_decode_hex, _encode_hex},
    };

    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_login() {
        let res: io::Result<Wallet> =
            "E7C33EA70CF2DBB24AA71F0604D7956CCBC5FE8F8F20C51328A14AC8725BE0F5".try_into();
        println!("{:?}", res);
    }

    #[test]
    fn create_wallet() {
        let my_wallet = Wallet::new();
        println!("Wallet: {:?}", my_wallet);
    }

    #[test]
    fn test_hash_address() {
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";

        let res = hash_address(address).unwrap();
        let expected = _decode_hex("6fc9bc003bf72ebdc53a9572f7ea792ef49a2858d78fc12f84").unwrap();
        assert_eq!(res, expected);
    }

    #[test]
    fn test_read_wallet_balance() {
        let mut utxo_set: UtxoSet = UtxoSet::new();
        let my_wallet: Wallet = "E7C33EA70CF2DBB24AA71F0604D7956CCBC5FE8F8F20C51328A14AC8725BE0F5"
            .try_into()
            .unwrap();

        let transaction_bytes = _decode_hex(
            "020000000001011216d10ae3afe6119529c0a01abe7833641e0e9d37eb880ae5547cfb7c6c7bca0000000000fdffffff0246b31b00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac731f2001020000001976a914d617966c3f29cfe50f7d9278dd3e460e3f084b7b88ac02473044022059570681a773748425ddd56156f6af3a0a781a33ae3c42c74fafd6cc2bd0acbc02200c4512c250f88653fae4d73e0cab419fa2ead01d6ba1c54edee69e15c1618638012103e7d8e9b09533ae390d0db3ad53cc050a54f89a987094bffac260f25912885b834b2c2500"
        ).unwrap();
        let transaction = RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes)).unwrap();
        transaction
            .generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        let balance = utxo_set.get_wallet_balance(&my_wallet.address);
        assert_eq!(balance, 1815366)
    }

    #[test]
    fn test_read_wallet_balance_with_spent() {
        let mut utxo_set: UtxoSet = UtxoSet::new();
        let my_wallet: Wallet = "E7C33EA70CF2DBB24AA71F0604D7956CCBC5FE8F8F20C51328A14AC8725BE0F5"
            .try_into()
            .unwrap();

        let transaction_1_bytes = _decode_hex(
            "020000000001011216d10ae3afe6119529c0a01abe7833641e0e9d37eb880ae5547cfb7c6c7bca0000000000fdffffff0246b31b00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac731f2001020000001976a914d617966c3f29cfe50f7d9278dd3e460e3f084b7b88ac02473044022059570681a773748425ddd56156f6af3a0a781a33ae3c42c74fafd6cc2bd0acbc02200c4512c250f88653fae4d73e0cab419fa2ead01d6ba1c54edee69e15c1618638012103e7d8e9b09533ae390d0db3ad53cc050a54f89a987094bffac260f25912885b834b2c2500"
        ).unwrap();
        let transaction_1 =
            RawTransaction::from_bytes(&mut Cursor::new(&transaction_1_bytes)).unwrap();
        transaction_1
            .generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        let transaction_2_bytes = _decode_hex(
            "02000000000101536d525880fd48a734fddd39d46d8f800ebf255102768d8d890603683a7af0b90000000000fdffffff0249def687010000001976a914799b0bc4ad97fff4c2e030443e4594ad374fa12788acb7051e00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac02473044022053e5d615cad3ad5efe972e891d401a19b8659687f00cac8df2b140ec1e4b5ad802200fe1e8c05a32b3f5e26fd5b956948817d07f9e753002162f97c76ccee7c7eb36012103084f5b365524916f2974248f0430bf26d223dd3a5422bc6ce04d0c8b4af71563a3302500"
        ).unwrap();
        let transaction_2 =
            RawTransaction::from_bytes(&mut Cursor::new(&transaction_2_bytes)).unwrap();
        transaction_2
            .generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        let transaction_3_bytes = _decode_hex(
            "0100000001881468a1a95473ed788c8a13bcdb7e524eac4f1088b1e2606ffb95492e239b10000000006a473044022021dc538aab629f2be56304937e796884356d1e79499150f5df03e8b8a545d17702205b76bda9c238035c907cbf6a39fa723d65f800ebb8082bdbb62d016d7937d990012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff0210270000000000001976a9144a82aaa02eba3c31cd86ee83345c4f91986743fe88ac96051a00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac00000000"
        ).unwrap();
        let transaction_3 =
            RawTransaction::from_bytes(&mut Cursor::new(&transaction_3_bytes)).unwrap();
        transaction_3
            .generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        let balance = utxo_set.get_wallet_balance(&my_wallet.address);
        assert_eq!(balance, 1705366 + 1967543)
    }

    #[test]
    fn test_generate_raw_transaction() {
        let wallet: Wallet = "E7C33EA70CF2DBB24AA71F0604D7956CCBC5FE8F8F20C51328A14AC8725BE0F5"
            .try_into()
            .unwrap();
        let mut utxo_set: UtxoSet = UtxoSet::new();

        // this transactions should give enough balance to send 1 tBTC
        let transaction_bytes = _decode_hex(
            "020000000001011216d10ae3afe6119529c0a01abe7833641e0e9d37eb880ae5547cfb7c6c7bca0000000000fdffffff0246b31b00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac731f2001020000001976a914d617966c3f29cfe50f7d9278dd3e460e3f084b7b88ac02473044022059570681a773748425ddd56156f6af3a0a781a33ae3c42c74fafd6cc2bd0acbc02200c4512c250f88653fae4d73e0cab419fa2ead01d6ba1c54edee69e15c1618638012103e7d8e9b09533ae390d0db3ad53cc050a54f89a987094bffac260f25912885b834b2c2500"
        ).unwrap();
        let transaction = RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes)).unwrap();
        transaction
            .generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        let recvr_addr = "mnJvq7mbGiPNNhUne4FAqq27Q8xZrAsVun".to_string();
        let recipients = vec![(recvr_addr.clone(), "foo".to_string(), 10000)];
        let transaction_info = TransactionInfo {
            recipients,
            fee: 100000,
        };
        let raw_transaction = wallet
            .generate_transaction(&mut utxo_set, transaction_info)
            .unwrap();

        let bytes = raw_transaction.serialize();
        let res = RawTransaction::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(res.tx_in_count, 1);
        assert_eq!(res.tx_out_count, 2);
        assert_eq!(res.tx_out[0].value, 10000);
        assert_eq!(res.tx_out[1].value, 1705366); // deducted fee of 10000

        let expected = "0100000001881468a1a95473ed788c8a13bcdb7e524eac4f1088b1e2606ffb95492e239b10000000006a473044022021dc538aab629f2be56304937e796884356d1e79499150f5df03e8b8a545d17702205b76bda9c238035c907cbf6a39fa723d65f800ebb8082bdbb62d016d7937d990012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff0210270000000000001976a9144a82aaa02eba3c31cd86ee83345c4f91986743fe88ac96051a00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac00000000";
        assert_eq!(expected, _encode_hex(&bytes));
    }

    #[test]
    fn test_send_to_self() {
        let wallet: Wallet = "E7C33EA70CF2DBB24AA71F0604D7956CCBC5FE8F8F20C51328A14AC8725BE0F5"
            .try_into()
            .unwrap();
        let mut utxo_set: UtxoSet = UtxoSet::new();

        let transaction_bytes = _decode_hex(
            "01000000011ecd55d9f67f16ffdc7b572a1c8baa2b4acb5c45c672f74e498b792d09f856a4010000006b483045022100bb0a409aa0b0a276b5ec4473f5aa9d526eb2e9835916f6754f7f5a89725b7f0c02204d3b3b3fe8f8af9e8de983301dd6bb5637e03038d94cba670b40b1e9ca221b29012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff0210270000000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac54121d00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac00000000"
        ).unwrap();
        let transaction = RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes)).unwrap();
        transaction
            .generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        let balance = utxo_set.get_wallet_balance(&wallet.address);
        assert_eq!(balance, 1905236 + 10000);
    }
}
