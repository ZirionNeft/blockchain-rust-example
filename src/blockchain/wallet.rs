use std::sync::Mutex;

use kv::{Bucket, Codec, Json};
use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use ripemd::Ripemd160;
use serde::{
    de::{MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Serialize,
};
use sha2::{Digest, Sha256};

use crate::{
    store::{init_store, WALLETS_BUCKET},
    utils::{checksum_hash, HashHex},
};

const VERSION: u16 = 1;

lazy_static! {
    static ref WALLETS: Mutex<Bucket<'static, String, Json<Wallet>>> = {
        let store = init_store().expect("Wallets store init error");

        let mut bucket = store
            .bucket::<String, Json<Wallet>>(Some(WALLETS_BUCKET))
            .expect("Wallets bucket init error");

        Mutex::new(bucket)
    };
}

#[derive(Debug, Clone)]
pub struct Wallet {
    private_key: SigningKey,
    pub public_key: VerifyingKey,
}

impl Serialize for Wallet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Wallet", 2)?;

        let pub_key_bytes = self.pub_key_bytes_vec();
        state.serialize_field("public_key", pub_key_bytes.as_slice());

        let private_key_bytes = self.private_key.to_bytes();
        state.serialize_field("private_key", private_key_bytes.as_slice());

        state.end()
    }
}

impl<'de> Deserialize<'de> for Wallet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            PrivateKey,
            PublicKey,
        }

        struct WalletVisitor;

        impl<'de> Visitor<'de> for WalletVisitor {
            type Value = Wallet;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Wallet")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Wallet, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut private_key = None;
                let mut public_key = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::PrivateKey => {
                            if private_key.is_some() {
                                return Err(serde::de::Error::duplicate_field("private_key"));
                            }
                            private_key = Some(map.next_value()?);
                        }
                        Field::PublicKey => {
                            if public_key.is_some() {
                                return Err(serde::de::Error::duplicate_field("public_key"));
                            }
                            public_key = Some(map.next_value()?);
                        }
                    }
                }
                let private_key =
                    private_key.ok_or_else(|| serde::de::Error::missing_field("private_key"))?;
                let public_key =
                    public_key.ok_or_else(|| serde::de::Error::missing_field("public_key"))?;

                let wallet = Wallet::from_bytes(private_key, public_key)
                    .expect("Wallet from bytes creating error");
                Ok(wallet)
            }
        }

        const FIELDS: &[&str] = &["private_key", "public_key"];
        deserializer.deserialize_struct("Wallet", FIELDS, WalletVisitor)
    }
}

impl Wallet {
    pub fn new() -> Self {
        let (private_key, public_key) = Self::new_key_pair();

        Wallet {
            private_key,
            public_key,
        }
    }

    pub fn from_bytes(
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let private_key_instance = SigningKey::from_bytes(private_key)?;
        let public_key_instance = VerifyingKey::try_from(public_key)?;

        Ok(Wallet {
            private_key: private_key_instance,
            public_key: public_key_instance,
        })
    }

    pub fn get_address(&self) -> HashHex {
        let pub_key_bytes = self.pub_key_bytes_vec();

        let pub_key_hash = Self::hash_pub_key(pub_key_bytes);

        let mut payload: Vec<u8> = VERSION.to_ne_bytes().to_vec();
        payload.extend(pub_key_hash.to_vec());

        let checksum = checksum_hash(payload.clone())[..4].to_vec();

        payload.extend(checksum);

        let encoded = bs58::encode(payload);

        HashHex(encoded.into_vec())
    }

    pub fn get_by(address: &str) -> Option<Wallet> {
        let wallets = WALLETS.lock().expect("Wallets locking error");

        if wallets.len() == 0 {
            return None;
        }

        match wallets.get(address) {
            Ok(v) => v.and_then(|v| Some(v.into_inner())),
            Err(e) => {
                println!("{}", e);
                None
            }
        }
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

    fn new_key_pair() -> (SigningKey, VerifyingKey) {
        let private_key = SigningKey::random(&mut OsRng);
        let public_key = VerifyingKey::from(&private_key);

        (private_key, public_key)
    }
}
