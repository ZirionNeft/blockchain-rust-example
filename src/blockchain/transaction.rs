use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    store::AppStore,
    utils::{HashHex, Result},
};

use super::{
    wallet::{Wallet, WalletNotFoundError},
    Blockchain,
};

const REWARD_AMOUNT: u32 = 10;

#[derive(Debug, Clone)]
pub struct NotEnoughFundsError;

impl fmt::Display for NotEnoughFundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not enough funds")
    }
}

impl std::error::Error for NotEnoughFundsError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutput {
    pub value: u32,
    pub pub_key_hash: HashHex,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXInput {
    pub tx_id: HashHex,
    pub output_index: i32,
    pub signature: HashHex,
    pub pub_key: HashHex,
}

impl TXOutput {
    pub fn is_locked_with(&self, pub_key_hash: &HashHex) -> bool {
        self.pub_key_hash.0 == pub_key_hash.0
    }

    pub fn lock(&mut self, address: HashHex) {
        let mut pub_key_hash = bs58::decode(address.0)
            .into_vec()
            .expect("base58 decode to vec error");
        pub_key_hash = pub_key_hash[1..(&pub_key_hash.len() - 4)].to_vec();
        self.pub_key_hash = pub_key_hash.into();
    }
}

impl TXInput {
    pub fn uses_key(&self, pub_key_hash: &HashHex) -> bool {
        let locking_hash = Wallet::hash_pub_key(self.pub_key.0.clone());

        pub_key_hash.0 == locking_hash.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: HashHex,
    pub inputs: Vec<TXInput>,
    pub outputs: Vec<TXOutput>,
}

impl Transaction {
    pub fn new(inputs: Vec<TXInput>, outputs: Vec<TXOutput>) -> Self {
        let vin_raw = serde_json::to_string(&inputs).expect("TXInput deserialize error");
        let vout_raw = serde_json::to_string(&outputs).expect("TXOutput deserialize error");

        let data = [vin_raw.as_bytes(), vout_raw.as_bytes()].concat();

        let mut hasher = Sha256::new();
        hasher.update(data);

        let hash_bytes: [u8; 32] = hasher.finalize().into();

        Transaction {
            id: HashHex(hash_bytes.to_vec()),
            inputs,
            outputs,
        }
    }

    pub fn new_utxo(from: String, to: String, amount: u32, bc: &Blockchain) -> Result<Transaction> {
        let wallet =
            Wallet::get_by(&from, bc.store).expect("Wallet with this address is not found");

        let pub_key = wallet.pub_key_bytes_vec();
        let pub_key_hash = Wallet::hash_pub_key(pub_key.clone());

        let (acc, spendable_outputs) = bc.find_spendable_outputs(&pub_key_hash, amount);

        if acc < amount {
            return Err(NotEnoughFundsError).map_err(|e| e.into());
        }

        let inputs: Vec<TXInput> = spendable_outputs
            .iter()
            .map(|(tx_id, outputs)| {
                outputs.iter().map(|output_index| TXInput {
                    output_index: *output_index,
                    tx_id: tx_id.to_owned(),
                    signature: HashHex(vec![]), // TODO: signature creating?
                    pub_key: pub_key.clone().into(),
                })
            })
            .flatten()
            .collect();

        let recipient_pub_key = Wallet::retrieve_pub_key_hash(&to)?;

        println!("recipient");

        let outputs = vec![
            TXOutput {
                pub_key_hash: recipient_pub_key,
                value: amount,
            },
            TXOutput {
                pub_key_hash,
                value: acc - amount,
            },
        ];

        Ok(Transaction::new(inputs, outputs))
    }

    pub fn new_coinbase(
        address: String,
        signature: Option<String>,
        store: &AppStore,
    ) -> Result<Self> {
        let wallet = match Wallet::get_by(&address, store) {
            Some(v) => v,
            None => return Err(WalletNotFoundError).map_err(|e| e.into()),
        };

        let pub_key = wallet.pub_key_bytes_vec();
        let pub_key_hash = Wallet::hash_pub_key(pub_key.clone());

        let signature = signature.unwrap_or(format!("Reward to {}", &address));

        let tx_in = TXInput {
            tx_id: HashHex(vec![]),
            output_index: -1,
            pub_key: HashHex(pub_key),
            signature: signature.as_bytes().into(),
        };
        let tx_out = TXOutput {
            value: REWARD_AMOUNT,
            pub_key_hash,
        };

        Ok(Transaction::new(vec![tx_in], vec![tx_out]))
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1
            && self.inputs[0].tx_id.0.is_empty()
            && self.inputs[0].output_index == -1
    }
}
