use crate::interface::GtkMessage;
use crate::raw_transaction::TransactionOrigin;
use crate::raw_transaction::{tx_output::TxOutput, RawTransaction};
use crate::utility::to_io_err;
use crate::utility::{double_hash, encode_hex};
use gtk::glib::Sender;
use std::collections::HashMap;
use std::io::Cursor;
use std::io::{self, Read};

pub type Lock = Vec<u8>;
pub type UtxoId = [u8; 32];
type Address = String;
pub type Index = u32;

/// Struct that represents a UTXOs pending to be spent
#[derive(Debug, Clone)]
pub struct PendingUtxo {
    pub utxos: HashMap<UtxoId, UtxoTransaction>,
    pub spent: HashMap<UtxoId, Vec<Index>>,
}

impl PendingUtxo {
    /// Creates a new `PendingUtxo` with empty utxos and spent
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            spent: HashMap::new(), // is this really needed?
        }
    }
}

/// Wallet that stores the UTXOs of the user (pending and spent)
#[derive(Debug, Clone)]
pub struct WalletUtxo {
    pub utxos: HashMap<(UtxoId, Index), UtxoTransaction>,
    pub spent: HashMap<UtxoId, Vec<Index>>,
    pub pending: PendingUtxo,
}

impl WalletUtxo {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            spent: HashMap::new(),
            pending: PendingUtxo::new(),
        }
    }

    /// Returns the UTXOs that are available to be spent
    pub fn get_available_utxos(&self) -> Vec<(UtxoId, UtxoTransaction)> {
        let mut available_utxos: Vec<(UtxoId, UtxoTransaction)> = Vec::new();

        for ((utxo_id, index), utxo) in &self.utxos {
            if let Some(spent) = self.spent.get(utxo_id) {
                if !spent.contains(index) {
                    available_utxos.push((*utxo_id, utxo.clone()));
                }
            } else {
                available_utxos.push((*utxo_id, utxo.clone()));
            }
        }

        available_utxos
    }

    /// Returns the sum of the UTXOs that are available to be spent
    pub fn get_balance(&self) -> u64 {
        let mut balance = 0;

        for ((utxo_id, index), utxo) in &self.utxos {
            if let Some(spent) = self.spent.get(utxo_id) {
                if !spent.contains(index) {
                    balance += utxo.value;
                }
            } else {
                balance += utxo.value;
            }
        }

        balance
    }

    /// Returns the sum of the UTXOs that are pending
    pub fn get_pending_balance(&self) -> u64 {
        let mut balance = 0;

        for (utxo_id, utxo) in &self.pending.utxos {
            if !self.pending.spent.contains_key(utxo_id) {
                balance += utxo.value;
            }
        }

        balance
    }

    /// Adds a UTXO to the wallet
    pub fn add_utxo(
        &mut self,
        utxo_id: UtxoId,
        utxo: UtxoTransaction,
        origin: TransactionOrigin,
        index: u32,
        ui_sender: Option<&Sender<GtkMessage>>,
        active_addr: Option<&str>,
    ) -> io::Result<()> {
        if origin == TransactionOrigin::Pending {
            self.add_pending_utxo(utxo_id, utxo);
            return Ok(());
        }

        if let Some(_pending) = self.pending.utxos.remove(&utxo_id) {
            if let Some(addr) = active_addr {
                if addr == utxo.get_address()? {
                    println!("pending utxo is now confirmed!");
                    if let Some(sender) = ui_sender {
                        let msg = format!("Transaction {} is now confirmed", encode_hex(&utxo_id));
                        let _ui = sender
                            .send(GtkMessage::CreateNotification((
                                gtk::MessageType::Info,
                                "Confirmed".to_string(),
                                msg,
                            )))
                            .map_err(to_io_err);
                    }
                }
            }
        }
        self.utxos.insert((utxo_id, index), utxo);
        Ok(())
    }

    /// Adds a spent UTXO to the wallet
    pub fn add_spent(&mut self, utxo_id: UtxoId, index: Index, origin: TransactionOrigin) {
        if origin == TransactionOrigin::Pending {
            self.add_pending_spent(utxo_id, index);
            return;
        }

        self.pending.spent.remove(&utxo_id);
        if let Some(spent) = self.spent.get_mut(&utxo_id) {
            spent.push(index);
        } else {
            self.spent.insert(utxo_id, vec![index]);
        }
    }

    /// Adds a pending UTXO to the wallet
    fn add_pending_utxo(&mut self, utxo_id: UtxoId, utxo: UtxoTransaction) {
        self.pending.utxos.insert(utxo_id, utxo);
    }

    /// Adds a pending spent UTXO to the wallet
    fn add_pending_spent(&mut self, utxo_id: UtxoId, index: Index) {
        if let Some(spent) = self.pending.spent.get_mut(&utxo_id) {
            spent.push(index);
        } else {
            self.pending.spent.insert(utxo_id, vec![index]);
        }
    }
}

/// Struct that represents the UTXO set of the blockchain as a hashmap of wallets
#[derive(Debug, Clone)]
pub struct UtxoSet {
    pub set: HashMap<Address, WalletUtxo>,
}

impl UtxoSet {
    pub fn new() -> Self {
        Self {
            set: HashMap::new(),
        }
    }

    /// returns available utxos for a given address
    pub fn get_wallet_available_utxos(&self, address: &str) -> Vec<(UtxoId, UtxoTransaction)> {
        if let Some(wallet) = self.set.get(address) {
            return wallet.get_available_utxos();
        }

        // eval check that it's not on pending spent either

        Vec::new()
    }

    /// Gets the wallet balance for a given address (sum of available utxos)
    // Maybe we should combine this method with the one bellow
    pub fn get_wallet_balance(&self, address: &str) -> u64 {
        if let Some(wallet) = self.set.get(address) {
            return wallet.get_balance();
        }

        0
    }

    /// Gets the wallet pending balance for a given address (sum of pending utxos)
    // Maybe we should combine this method with the one above
    pub fn get_pending_wallet_balance(&self, address: &str) -> u64 {
        if let Some(wallet) = self.set.get(address) {
            return wallet.get_pending_balance();
        }

        0
    }
}

/// Struct that represents a UTXO transaction (index, value, lock)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtxoTransaction {
    pub index: u32,
    pub value: u64,
    pub lock: Lock,
}

/// Translate a P2PKH address to an address
pub fn p2pkh_to_address(p2pkh: [u8; 20]) -> String {
    let version_prefix: [u8; 1] = [0x6f];

    let hash = double_hash(&[&version_prefix[..], &p2pkh[..]].concat());

    let checksum = &hash[..4];

    let input = [&version_prefix[..], &p2pkh[..], checksum].concat();

    bs58::encode(input).into_string()
}

impl UtxoTransaction {
    /// Returns the address of the UTXO
    pub fn get_address(&self) -> io::Result<String> {
        // iterate lock one byte at a time until 0x14 is found
        let mut cursor = Cursor::new(self.lock.clone());

        let buf = &mut [0; 1];
        while buf[0] != 0x14 {
            cursor.read_exact(buf)?;
        }

        let mut pk_hash = [0; 20];
        cursor.read_exact(&mut pk_hash)?;

        Ok(p2pkh_to_address(pk_hash))
    }

    /// Returns the UTXO from a TxOutput
    pub fn from_tx_output(tx_output: &TxOutput, index: u32) -> io::Result<Self> {
        let value = tx_output.value;
        let lock = tx_output.pk_script.clone();
        Ok(Self { index, value, lock })
    }
}

/// Struct that represents a Utxo (list of UtxoTransactions)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    pub transactions: Vec<UtxoTransaction>,
}

impl Utxo {
    /// Returns the UTXO from a RawTransaction
    pub fn from_raw_transaction(raw_transaction: &RawTransaction) -> io::Result<Utxo> {
        let mut utxo = Utxo {
            transactions: Vec::new(),
        };

        for (index, tx_output) in raw_transaction.tx_out.iter().enumerate() {
            let utxo_transaction = UtxoTransaction::from_tx_output(tx_output, index as u32)?;
            utxo.transactions.push(utxo_transaction);
        }
        Ok(utxo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{raw_transaction::TransactionOrigin, utility::_decode_hex};

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

    #[test]
    fn test_get_wallet_balance_from_various_tx() {
        let mut utxo_set = UtxoSet::new();

        // read tx_a that generates utxoA
        let bytes = _decode_hex("020000000001011216d10ae3afe6119529c0a01abe7833641e0e9d37eb880ae5547cfb7c6c7bca0000000000fdffffff0246b31b00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac731f2001020000001976a914d617966c3f29cfe50f7d9278dd3e460e3f084b7b88ac02473044022059570681a773748425ddd56156f6af3a0a781a33ae3c42c74fafd6cc2bd0acbc02200c4512c250f88653fae4d73e0cab419fa2ead01d6ba1c54edee69e15c1618638012103e7d8e9b09533ae390d0db3ad53cc050a54f89a987094bffac260f25912885b834b2c2500").unwrap();
        let tx_a = RawTransaction::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        tx_a.generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        assert_eq!(
            utxo_set.get_wallet_balance("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"),
            1815366
        );

        // read tx_b that generates utxoB, but spends utxoA
        let bytes = _decode_hex("0100000001881468a1a95473ed788c8a13bcdb7e524eac4f1088b1e2606ffb95492e239b10000000006a473044022021dc538aab629f2be56304937e796884356d1e79499150f5df03e8b8a545d17702205b76bda9c238035c907cbf6a39fa723d65f800ebb8082bdbb62d016d7937d990012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff0210270000000000001976a9144a82aaa02eba3c31cd86ee83345c4f91986743fe88ac96051a00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac00000000").unwrap();
        let tx_b = RawTransaction::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        tx_b.generate_utxo(&mut utxo_set, TransactionOrigin::Block, None, None)
            .unwrap();

        assert_eq!(
            utxo_set.get_wallet_balance("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"),
            1705366
        );

        // read pending tx_c that generates utxoC
        let bytes = _decode_hex("020000000001011caf1fc6e053c6048b7108c3d22be0f57f95cc676ae688ef22e7793d853afd860100000000fdffffff02c81d1e00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ace6964cbe010000001976a914bbfb2d931dd19e1d3a503d0bfaba40cc2d3203fb88ac024730440220239f9521c30a2bb7df61011e0486712ab01a5fb43009ff872023051433f94a93022044f647c595894eba610b9089db5f34faea06aa0c58a684220c755fc38929f29201210394e3ae4d013b556c51d514a77ac5b5aae2f4e81edaedb192fc8b45b7a97d52ac9b2f2500").unwrap();
        let tx_c = RawTransaction::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        tx_c.generate_utxo(&mut utxo_set, TransactionOrigin::Pending, None, None)
            .unwrap(); // generate as pending

        assert_eq!(
            utxo_set.get_pending_wallet_balance("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"),
            1973704
        );
        // assure balance is not changed
        assert_eq!(
            utxo_set.get_wallet_balance("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"),
            1705366
        );

        // read txC that generated utxoC
        tx_c.generate_utxo(
            &mut utxo_set,
            TransactionOrigin::Block,
            None,
            Some("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"),
        )
        .unwrap();

        // pending balance should now be 0
        assert_eq!(
            utxo_set.get_pending_wallet_balance("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"),
            0
        );
        // balance should now be the same as before plus the pending balance now confirmed
        assert_eq!(
            utxo_set.get_wallet_balance("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"),
            1705366 + 1973704
        );
    }
}
