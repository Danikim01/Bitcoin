use rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::io;

use crate::{
    raw_transaction::{RawTransaction, TxInput, TxInputType, TxOutput},
    utxo::UtxoSet,
};

#[derive(PartialEq, Debug)]
pub struct Wallet {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
}

impl Wallet {
    pub fn new() -> Self {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        Self {
            secret_key,
            public_key,
        }
    }

    pub fn generate_transaction(
        &self,
        utxo_set: &UtxoSet,
        recv_addr: PublicKey,
        amount: u64,
    ) -> io::Result<RawTransaction> {

        let txin: Vec<TxInput> = Vec::new();
        //  we have to put the minimum number of utxos from
        //  our account needed to cover the amount we want to send

        let txout: Vec<TxOutput> = vec![TxOutput {
            value: 0 as i64,
            pk_script_bytes: 0 as u64,
            pk_script: vec![0; 1],
        }];

        //  the first txout is destined for the receiver
        //  the other txout is our "change"

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
    use super::*;

    #[test]
    fn create_wallet() {
        let my_wallet = Wallet::new();
        println!("Public key: {}", my_wallet.public_key);

        let new_public = PublicKey::from_secret_key(&Secp256k1::new(), &my_wallet.secret_key);
        println!("Public key: {}", new_public);
    }
}
