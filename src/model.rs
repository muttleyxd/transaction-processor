use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub type ClientId = u16;
pub type TransactionId = u32;

// Allowing dead code for now, as debug print output is used
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct InputRecord {
    pub r#type: InputRecordType,
    #[serde(rename = "client")]
    pub client_id: ClientId,
    #[serde(rename = "tx")]
    pub transaction_id: TransactionId,

    // Decimal used here, floats are not safe for calculating money
    #[serde(default)]
    pub amount: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputRecordType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Serialize)]
pub struct OutputRecord {
    #[serde(rename = "client")]
    pub client_id: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}
