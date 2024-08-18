use std::collections::HashMap;

use crate::{
    account_manager::account::Account,
    model::{ClientId, InputRecord, OutputRecord},
};

pub mod account;

pub struct AccountManager {
    accounts: HashMap<ClientId, Account>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn process_record(&mut self, record: &InputRecord) -> Result<(), account::ProcessingError> {
        let account = self
            .accounts
            .entry(record.client_id)
            .or_insert_with(|| Account::new(record.client_id));
        account.process_record(record)?;

        Ok(())
    }

    pub fn gather_output(&self) -> Vec<OutputRecord> {
        self.accounts.values().map(Account::to_output).collect()
    }
}
