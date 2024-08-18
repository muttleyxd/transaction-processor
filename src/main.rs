use csv::Trim;

use crate::account_manager::AccountManager;

mod account_manager;
mod model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_path("transactions.csv")
        .unwrap();

    let mut account_manager = AccountManager::new();

    for result in reader.deserialize() {
        let record: model::InputRecord = result?;
        account_manager.process_record(record).unwrap(); // todo: handle errors
    }

    Ok(())
}
