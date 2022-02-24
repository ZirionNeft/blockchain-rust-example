use std::fmt;

use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

use crate::{
    store::AppStore,
    utils::{HashHex, Result},
};

const VERSION: u16 = 1;

#[derive(Debug, Clone)]
pub struct WalletNotFoundError;

impl fmt::Display for WalletNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Wallet is not found")
    }
}

impl std::error::Error for WalletNotFoundError {}

#[derive(Debug, Clone)]
pub struct Wallet {
    private_key: SigningKey,
    pub public_key: VerifyingKey,
}

impl Wallet {
    pub fn new() -> Self {
        let private_key = SigningKey::random(&mut OsRng);
        let public_key = VerifyingKey::from(&private_key);

        Wallet {
            private_key,
            public_key,
        }
    }

    pub fn create() -> Result<HashHex> {
        let wallet = Wallet::new();

        let address = wallet.generate_address();

        let wallets = AppStore::get_wallets_bucket()?;

        wallets.set(address.to_vec(), wallet.private_key.to_bytes().to_vec())?;

        Ok(address)
    }

    pub fn from_bytes(private_key: &[u8], public_key: &[u8]) -> Result<Self> {
        let private_key_instance = SigningKey::from_bytes(private_key)?;
        let public_key_instance = VerifyingKey::try_from(public_key)?;

        Ok(Wallet {
            private_key: private_key_instance,
            public_key: public_key_instance,
        })
    }

    pub fn get_all_addresses() -> Result<Vec<HashHex>> {
        let wallets = AppStore::get_wallets_bucket()?;

        let addresses: Vec<HashHex> = wallets
            .iter()
            .map(|k| {
                k.expect("Wallets item error")
                    .key()
                    .expect("Wallets key error")
            })
            .collect();
        Ok(addresses)
    }

    pub fn generate_address(&self) -> HashHex {
        let pub_key_bytes = self.pub_key_bytes_vec();

        let pub_key_hash = Self::hash_pub_key(pub_key_bytes);

        let mut payload: Vec<u8> = VERSION.to_ne_bytes().to_vec();
        payload.extend(pub_key_hash.to_vec());

        let checksum = Self::checksum_hash(payload.clone())[..4].to_vec();

        payload.extend(checksum);

        let encoded = bs58::encode(payload);

        HashHex(encoded.into_vec())
    }

    pub fn get_by(address: &str) -> Option<Wallet> {
        let wallets = AppStore::get_wallets_bucket().expect("Get wallets bucket error");

        if wallets.is_empty() {
            return None;
        }

        let private_key = wallets.get(address).ok().flatten();

        let private_key = match SigningKey::from_bytes(private_key?.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                return None;
            }
        };

        let public_key = VerifyingKey::from(&private_key);

        Some(Wallet {
            private_key,
            public_key,
        })
    }

    pub fn pub_key_bytes_vec(&self) -> Vec<u8> {
        let pub_key = self.public_key.to_encoded_point(false);

        pub_key.as_bytes().to_vec()
    }

    pub fn hash_pub_key(key: Vec<u8>) -> HashHex {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let result = hasher.finalize();

        let mut hasher = Ripemd160::new();
        hasher.update(result);
        let result = hasher.finalize().to_vec();

        HashHex(result)
    }

    pub fn retrieve_pub_key_hash(address: &str) -> Result<HashHex> {
        let bytes = bs58::decode(address).into_vec()?;
        let pub_key_hash = bytes.as_slice()[1..bytes.len() - 4].to_vec();

        Ok(HashHex(pub_key_hash))
    }

    fn checksum_hash(payload: Vec<u8>) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let result = hasher.finalize();

        let mut hasher = Sha256::new();
        hasher.update(result);
        hasher.finalize().to_vec()
    }
}
