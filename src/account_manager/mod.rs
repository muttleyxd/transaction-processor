use std::collections::HashMap;

use thiserror::Error;

use crate::{
    account_manager::account::Account,
    model::{ClientId, InputRecord},
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

    pub fn process_record(&mut self, record: InputRecord) -> Result<(), Error> {
        if self.accounts.contains_key(&record.client_id) {
            self.accounts.insert(
                record.client_id.clone(),
                Account::new(record.client_id.clone()),
            );
        }

        let account = self
            .accounts
            .get_mut(&record.client_id)
            .ok_or(Error::CannotRetrieveAccount(record.client_id.clone()))?;
        account.process_record(record)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot retrieve account for client: `{0}`")]
    CannotRetrieveAccount(ClientId),
    #[error("Record processing error: `{0}`")]
    RecordProcessing(#[from] account::ProcessingError),
}
