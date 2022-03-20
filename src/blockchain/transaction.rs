use std::{collections::HashMap, fmt, vec};

use p256::{
    ecdsa::{
        signature::{Signature, Signer, Verifier},
        SigningKey, VerifyingKey,
    },
    EncodedPoint,
};
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
        let id = Self::calculate_hash(&inputs, &outputs).unwrap();

        Transaction {
            id,
            inputs,
            outputs,
        }
    }

    pub fn calculate_hash(inputs: &[TXInput], outputs: &[TXOutput]) -> Result<HashHex> {
        let vin_raw = serde_json::to_string(inputs)?;
        let vout_raw = serde_json::to_string(outputs)?;

        let data = [vin_raw.as_bytes(), vout_raw.as_bytes()].concat();

        let mut hasher = Sha256::new();
        hasher.update(data);

        let hash_bytes: [u8; 32] = hasher.finalize().into();

        Ok(HashHex(hash_bytes.to_vec()))
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
                    signature: HashHex(vec![]),
                    pub_key: pub_key.clone().into(),
                })
            })
            .flatten()
            .collect();

        let recipient_pub_key = Wallet::retrieve_pub_key_hash(&to)?;

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

        let mut tx = Transaction::new(inputs, outputs);

        bc.sign_transaction(&mut tx, &wallet.private_key);

        Ok(tx)
    }

    pub fn sign(
        &mut self,
        prev_transactions: &mut HashMap<HashHex, Transaction>,
        private_key: &SigningKey,
    ) {
        if self.is_coinbase() {
            println!("[!] Signing skip - ({:?}) is coinbase transaction", self.id);
            return;
        }

        let mut new_tx = self.trimmed_copy();

        let new_inputs = new_tx.inputs.clone();

        for (index, input) in new_inputs.iter().enumerate() {
            let prev_tx = prev_transactions.get(&input.tx_id).unwrap();

            let inputs = &mut new_tx.inputs;

            inputs[index].signature = vec![].into();
            inputs[index].pub_key = prev_tx.outputs[input.output_index as usize]
                .pub_key_hash
                .clone();

            new_tx.id = Self::calculate_hash(inputs, &new_tx.outputs).unwrap();

            // Clearing value to evade side-effects
            inputs[index].pub_key = vec![].into();

            let signature = private_key.sign(&new_tx.id.0);

            self.inputs[index].signature = signature.to_vec().into();
        }
    }

    pub fn verify(&mut self, prev_transactions: &HashMap<HashHex, Transaction>) -> bool {
        let mut new_tx = self.trimmed_copy();

        for (index, input) in self.inputs.iter().enumerate() {
            let prev_tx = prev_transactions.get(&input.tx_id).unwrap();

            let inputs = &mut new_tx.inputs;

            inputs[index].signature = vec![].into();
            inputs[index].pub_key = prev_tx.outputs[input.output_index as usize]
                .pub_key_hash
                .clone();

            new_tx.id = Self::calculate_hash(inputs, &new_tx.outputs).unwrap();

            inputs[index].pub_key = vec![].into();

            let encoded_point = EncodedPoint::from_bytes(&input.pub_key.0).unwrap();
            let verify_key = VerifyingKey::from_encoded_point(&encoded_point).unwrap();

            let signature = p256::ecdsa::Signature::from_bytes(&input.signature.0).unwrap();

            if verify_key.verify(&new_tx.id.0, &signature).is_err() {
                return false;
            }
        }

        true
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

    fn trimmed_copy(&self) -> Self {
        let inputs: Vec<TXInput> = self
            .inputs
            .iter()
            .map(|input| TXInput {
                tx_id: input.tx_id.clone(),
                output_index: input.output_index,
                pub_key: vec![].into(),
                signature: vec![].into(),
            })
            .collect();

        let outputs: Vec<TXOutput> = self.outputs.clone();

        Transaction {
            id: self.id.clone(),
            inputs,
            outputs,
        }
    }
}
