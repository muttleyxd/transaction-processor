use std::collections::HashMap;

use rust_decimal::Decimal;
use strum::Display;
use thiserror::Error;

use crate::model::{ClientId, InputRecord, InputRecordType, OutputRecord, TransactionId};

#[derive(Debug)]
pub struct Account {
    client_id: ClientId,
    transactions: HashMap<TransactionId, Transaction>,

    available: Decimal,
    held: Decimal,

    locked: bool,
}

#[derive(Debug)]
struct Transaction {
    pub state: TransactionState,
    pub amount: Decimal,
    pub r#type: TransactionType,
}

#[derive(Clone, Debug, Eq, PartialEq, Display)]
pub enum TransactionState {
    Valid,
    Dispute,
    Resolved,
    ChargedBack,
}

#[derive(Debug, Eq, PartialEq)]
enum TransactionType {
    Deposit,
    Withdrawal,
}

impl Account {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            transactions: HashMap::default(),

            available: Decimal::ZERO,
            held: Decimal::ZERO,

            locked: false,
        }
    }

    pub fn process_record(&mut self, record: &InputRecord) -> Result<(), ProcessingError> {
        if self.locked {
            return Err(ProcessingError::AccountIsLocked);
        }

        match record.r#type {
            InputRecordType::Deposit => {
                if self.transactions.contains_key(&record.transaction_id) {
                    return Err(ProcessingError::TransactionAlreadyExists(
                        record.transaction_id,
                    ));
                }

                let amount = record.amount.ok_or(ProcessingError::AmountMissing)?;

                self.transactions.insert(
                    record.transaction_id,
                    Transaction {
                        state: TransactionState::Valid,
                        amount,
                        r#type: TransactionType::Deposit,
                    },
                );

                self.available += amount;
            }
            InputRecordType::Withdrawal => {
                if self.transactions.contains_key(&record.transaction_id) {
                    return Err(ProcessingError::TransactionAlreadyExists(
                        record.transaction_id,
                    ));
                }

                let amount = record.amount.ok_or(ProcessingError::AmountMissing)?;

                let new_available = self
                    .available
                    .checked_sub(amount)
                    .ok_or(ProcessingError::DecimalOverflow)?;
                if new_available < Decimal::ZERO {
                    return Err(ProcessingError::WithdrawalNotEnoughMoneyAvailable(
                        self.available,
                        amount,
                    ));
                }

                self.transactions.insert(
                    record.transaction_id,
                    Transaction {
                        state: TransactionState::Valid,
                        amount: -amount,
                        r#type: TransactionType::Withdrawal,
                    },
                );

                self.available = new_available;
            }
            InputRecordType::Dispute => {
                let transaction = self
                    .transactions
                    .get_mut(&record.transaction_id)
                    .ok_or(ProcessingError::TransactionMissing(record.transaction_id))?;
                check_if_state_eq(transaction, TransactionState::Valid)?;

                let new_available = self
                    .available
                    .checked_sub(transaction.amount)
                    .ok_or(ProcessingError::DecimalOverflow)?;
                let new_held = self
                    .held
                    .checked_add(transaction.amount)
                    .ok_or(ProcessingError::DecimalOverflow)?;

                transaction.state = TransactionState::Dispute;
                self.available = new_available;
                self.held = new_held;
            }
            InputRecordType::Resolve => {
                let transaction = self
                    .transactions
                    .get_mut(&record.transaction_id)
                    .ok_or(ProcessingError::TransactionMissing(record.transaction_id))?;
                check_if_state_eq(transaction, TransactionState::Dispute)?;

                let (new_available, new_held) =
                    calculate_transaction_revert(transaction, self.available, self.held)?;
                self.available = new_available;
                self.held = new_held;
                transaction.state = TransactionState::Resolved;
            }
            InputRecordType::Chargeback => {
                let transaction = self
                    .transactions
                    .get_mut(&record.transaction_id)
                    .ok_or(ProcessingError::TransactionMissing(record.transaction_id))?;
                check_if_state_eq(transaction, TransactionState::Dispute)?;

                let (new_available, new_held) =
                    calculate_transaction_revert(transaction, self.available, self.held)?;
                self.available = new_available;
                self.held = new_held;
                self.locked = true;
                transaction.state = TransactionState::ChargedBack;
            }
        }

        Ok(())
    }

    pub fn to_output(&self) -> OutputRecord {
        OutputRecord {
            client_id: self.client_id,
            available: self.available,
            held: self.held,
            total: self.available + self.held,
            locked: self.locked,
        }
    }
}

#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("Account is locked")]
    AccountIsLocked,
    #[error("Amount missing")]
    AmountMissing,
    #[error("Decimal overflow")]
    DecimalOverflow,

    #[error("Transaction already exists: `{0}`")]
    TransactionAlreadyExists(TransactionId),
    #[error("Transaction missing: `{0}`")]
    TransactionMissing(TransactionId),
    #[error("Transaction wrong state, expected: `{0}`, actual: `{0}`")]
    TransactionWrongState(TransactionState, TransactionState),

    #[error("Withdrawal: not enough money available, available: `{0}`, requested: `{1}`")]
    WithdrawalNotEnoughMoneyAvailable(Decimal, Decimal),
}

fn calculate_transaction_revert(
    transaction: &Transaction,
    available: Decimal,
    held: Decimal,
) -> Result<(Decimal, Decimal), ProcessingError> {
    match transaction.r#type {
        TransactionType::Deposit => {
            let new_held = held
                .checked_sub(transaction.amount)
                .ok_or(ProcessingError::DecimalOverflow)?;
            Ok((available, new_held))
        }
        TransactionType::Withdrawal => {
            let new_held = held
                .checked_sub(-transaction.amount)
                .ok_or(ProcessingError::DecimalOverflow)?;
            let new_available = available
                .checked_add(-transaction.amount)
                .ok_or(ProcessingError::DecimalOverflow)?;
            Ok((new_available, new_held))
        }
    }
}

fn check_if_state_eq(
    transaction: &Transaction,
    expected: TransactionState,
) -> Result<(), ProcessingError> {
    if transaction.state != expected {
        return Err(ProcessingError::TransactionWrongState(
            expected,
            transaction.state.clone(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use super::{Account, ProcessingError, Transaction, TransactionState, TransactionType};
    use crate::model::{InputRecord, InputRecordType};

    #[test]
    fn test_to_output() {
        let account = Account {
            client_id: 1234,
            transactions: HashMap::default(),
            available: dec!(10.0),
            held: dec!(15.0),
            locked: true,
        };

        let output = account.to_output();
        assert_eq!(1234, output.client_id);
        assert_eq!(dec!(10.0), output.available);
        assert_eq!(dec!(15.0), output.held);
        assert_eq!(dec!(25.0), output.total);
        assert!(output.locked);
    }

    #[test]
    fn test_process_deposit_success() {
        let mut account = Account::new(0);
        account
            .process_record(&InputRecord {
                r#type: InputRecordType::Deposit,
                client_id: 0,
                transaction_id: 0,
                amount: Some(dec!(1.0)),
            })
            .unwrap();

        assert_eq!(dec!(1.0), account.available);
    }

    #[test]
    fn test_process_deposit_fail_missing_amount() {
        let mut account = Account::new(0);
        let result = account.process_record(&InputRecord {
            r#type: InputRecordType::Deposit,
            client_id: 0,
            transaction_id: 0,
            amount: None,
        });

        assert!(matches!(result, Err(ProcessingError::AmountMissing)));
    }

    #[test]
    fn test_process_withdrawal_success() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::default(),
            available: dec!(10.0),
            held: Decimal::default(),
            locked: false,
        };
        account
            .process_record(&InputRecord {
                r#type: InputRecordType::Withdrawal,
                client_id: 0,
                transaction_id: 0,
                amount: Some(dec!(1.0)),
            })
            .unwrap();

        assert_eq!(dec!(9.0), account.available);
        assert_eq!(dec!(-1.0), account.transactions[&0].amount);
    }

    #[test]
    fn test_process_withdraw_fail_missing_amount() {
        let mut account = Account::new(0);
        let result = account.process_record(&InputRecord {
            r#type: InputRecordType::Withdrawal,
            client_id: 0,
            transaction_id: 0,
            amount: None,
        });

        assert!(matches!(result, Err(ProcessingError::AmountMissing)));
    }

    #[test]
    fn test_process_withdrawal_fail_not_enough_money() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::default(),
            available: dec!(10.0),
            held: Decimal::default(),
            locked: false,
        };
        let result = account.process_record(&InputRecord {
            r#type: InputRecordType::Withdrawal,
            client_id: 0,
            transaction_id: 0,
            amount: Some(dec!(11.0)),
        });

        assert!(matches!(
            result,
            Err(ProcessingError::WithdrawalNotEnoughMoneyAvailable(_, _))
        ));
    }

    #[test]
    fn test_process_dispute_deposit_success() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::Valid,
                    amount: dec!(10.0),
                    r#type: TransactionType::Deposit,
                },
            )]),
            available: dec!(10.0),
            held: dec!(0.0),
            locked: false,
        };
        account
            .process_record(&InputRecord {
                r#type: InputRecordType::Dispute,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            })
            .unwrap();

        assert_eq!(dec!(0.0), account.available);
        assert_eq!(dec!(10.0), account.held);
    }

    #[test]
    fn test_process_dispute_withdrawal_success() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::Valid,
                    amount: dec!(-10.0),
                    r#type: TransactionType::Withdrawal,
                },
            )]),
            available: dec!(0.0),
            held: dec!(10.0),
            locked: false,
        };
        account
            .process_record(&InputRecord {
                r#type: InputRecordType::Dispute,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            })
            .unwrap();

        assert_eq!(dec!(10.0), account.available);
        assert_eq!(dec!(0.0), account.held);
        assert_eq!(
            TransactionState::Dispute,
            account.transactions[&0].state
        );
    }

    #[test]
    fn test_process_dispute_fail_missing_transaction() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::ChargedBack,
                    amount: dec!(10.0),
                    r#type: TransactionType::Deposit,
                },
            )]),
            available: dec!(10.0),
            held: dec!(0.0),
            locked: false,
        };
        let result = account.process_record(&InputRecord {
            r#type: InputRecordType::Dispute,
            client_id: 0,
            transaction_id: 0,
            amount: None,
        });
        assert!(matches!(
            result,
            Err(ProcessingError::TransactionWrongState(_, _))
        ));
    }

    #[test]
    fn test_process_dispute_fail_wrong_state() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::ChargedBack,
                    amount: dec!(10.0),
                    r#type: TransactionType::Deposit,
                },
            )]),
            available: dec!(10.0),
            held: dec!(0.0),
            locked: false,
        };
        let result = account.process_record(&InputRecord {
            r#type: InputRecordType::Dispute,
            client_id: 0,
            transaction_id: 0,
            amount: None,
        });
        assert!(matches!(
            result,
            Err(ProcessingError::TransactionWrongState(_, _))
        ));
    }

    #[test]
    fn test_process_resolve_deposit_success() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::Dispute,
                    amount: dec!(10.0),
                    r#type: TransactionType::Deposit,
                },
            )]),
            available: dec!(0.0),
            held: dec!(10.0),
            locked: false,
        };
        account
            .process_record(&InputRecord {
                r#type: InputRecordType::Resolve,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            })
            .unwrap();

        assert_eq!(dec!(0.0), account.available);
        assert_eq!(dec!(0.0), account.held);
        assert_eq!(
            TransactionState::Resolved,
            account.transactions[&0].state
        );
    }

    #[test]
    fn test_process_resolve_withdrawal_success() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::Dispute,
                    amount: dec!(-10.0),
                    r#type: TransactionType::Withdrawal,
                },
            )]),
            available: dec!(0.0),
            held: dec!(10.0),
            locked: false,
        };
        account
            .process_record(&InputRecord {
                r#type: InputRecordType::Resolve,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            })
            .unwrap();

        assert_eq!(dec!(10.0), account.available);
        assert_eq!(dec!(0.0), account.held);
        assert_eq!(
            TransactionState::Resolved,
            account.transactions[&0].state
        );
    }

    #[test]
    fn test_process_chargeback_deposit_success() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::Dispute,
                    amount: dec!(10.0),
                    r#type: TransactionType::Deposit,
                },
            )]),
            available: dec!(0.0),
            held: dec!(10.0),
            locked: false,
        };
        account
            .process_record(&InputRecord {
                r#type: InputRecordType::Chargeback,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            })
            .unwrap();

        assert_eq!(dec!(0.0), account.available);
        assert_eq!(dec!(0.0), account.held);
        assert!(account.locked);
        assert_eq!(
            TransactionState::ChargedBack,
            account.transactions[&0].state
        );
    }

    #[test]
    fn test_process_locked_account_fails_everything() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::default(),
            available: dec!(0.0),
            held: dec!(10.0),
            locked: true,
        };
        assert!(matches!(
            account.process_record(&InputRecord {
                r#type: InputRecordType::Deposit,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            }),
            Err(ProcessingError::AccountIsLocked)
        ));
        assert!(matches!(
            account.process_record(&InputRecord {
                r#type: InputRecordType::Withdrawal,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            }),
            Err(ProcessingError::AccountIsLocked)
        ));
        assert!(matches!(
            account.process_record(&InputRecord {
                r#type: InputRecordType::Dispute,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            }),
            Err(ProcessingError::AccountIsLocked)
        ));
        assert!(matches!(
            account.process_record(&InputRecord {
                r#type: InputRecordType::Resolve,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            }),
            Err(ProcessingError::AccountIsLocked)
        ));
        assert!(matches!(
            account.process_record(&InputRecord {
                r#type: InputRecordType::Chargeback,
                client_id: 0,
                transaction_id: 0,
                amount: None,
            }),
            Err(ProcessingError::AccountIsLocked)
        ));
    }

    #[test]
    fn test_process_transaction_already_exists() {
        let mut account = Account {
            client_id: 0,
            transactions: HashMap::from([(
                0,
                Transaction {
                    state: TransactionState::Valid,
                    amount: Default::default(),
                    r#type: TransactionType::Deposit,
                },
            )]),
            available: dec!(0.0),
            held: dec!(0.0),
            locked: false,
        };

        assert!(matches!(
            account.process_record(&InputRecord {
                r#type: InputRecordType::Deposit,
                client_id: 0,
                transaction_id: 0,
                amount: Some(dec!(50.0)),
            }),
            Err(ProcessingError::TransactionAlreadyExists(_))
        ));
        assert!(matches!(
            account.process_record(&InputRecord {
                r#type: InputRecordType::Withdrawal,
                client_id: 0,
                transaction_id: 0,
                amount: Some(dec!(50.0)),
            }),
            Err(ProcessingError::TransactionAlreadyExists(_))
        ));
    }
}
