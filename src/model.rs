use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// Allowing dead code for now, as debug print output is used
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct InputRecord {
    pub r#type: InputRecordType,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub transaction_id: u32,

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
    pub client_id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}
