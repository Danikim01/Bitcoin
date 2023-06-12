use rand::rngs::OsRng;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

#[derive(PartialEq, Debug)]
pub struct Account {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
}

impl Account {
    pub fn new() -> Account {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
        Account {
            secret_key,
            public_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_account() {
        let my_account = Account::new();
        println!("Public key:{}", my_account.public_key);
        assert!(1 == 1);
    }
}
