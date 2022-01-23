use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::utils::HashHex;

use super::Blockchain;

const REWARD_AMOUNT: u32 = 10;

#[derive(Debug, Clone)]
pub struct NotEnoughFundsError;

impl fmt::Display for NotEnoughFundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not enough funds")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutput {
    pub value: u32,
    pub script_pub_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXInput {
    pub tx_id: HashHex,
    pub output_index: i32,
    pub script_signature: String,
}

impl TXOutput {
    pub fn is_unlockable_with(&self, script_pub_key: &str) -> bool {
        self.script_pub_key == script_pub_key
    }
}

impl TXInput {
    pub fn can_unlock_output_with(&self, script_signature: &str) -> bool {
        self.script_signature == script_signature
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

    pub fn new_utxo(
        from: String,
        to: String,
        amount: u32,
        bc: &Blockchain,
    ) -> Result<Transaction, NotEnoughFundsError> {
        let (acc, spendable_outputs) = bc.find_spendable_outputs(&from, amount);

        if acc < amount {
            return Err(NotEnoughFundsError);
        }

        let inputs: Vec<TXInput> = spendable_outputs
            .iter()
            .map(|(tx_id, outputs)| {
                outputs.iter().map(|output_index| TXInput {
                    output_index: *output_index,
                    tx_id: tx_id.to_owned(),
                    script_signature: from.to_owned(),
                })
            })
            .flatten()
            .collect();

        let outputs = vec![
            TXOutput {
                script_pub_key: to,
                value: amount,
            },
            TXOutput {
                script_pub_key: from,
                value: acc - amount,
            },
        ];

        Ok(Transaction::new(inputs, outputs))
    }

    pub fn new_coinbase(to: String, signature: Option<String>) -> Self {
        let signature = signature.unwrap_or(format!("Reward to {}", &to));

        let tx_in = TXInput {
            tx_id: HashHex(vec![]),
            output_index: -1,
            script_signature: signature,
        };
        let tx_out = TXOutput {
            value: REWARD_AMOUNT,
            script_pub_key: to,
        };

        Transaction::new(vec![tx_in], vec![tx_out])
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1
            && self.inputs[0].tx_id.0.is_empty()
            && self.inputs[0].output_index == -1
    }
}
