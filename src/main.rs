use std::path::PathBuf;

use clap::Parser;
use csv::Trim;

use crate::account_manager::AccountManager;

mod account_manager;
mod model;

#[derive(Debug, Parser)]
#[command(version, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false, help = "Log errors to stderr")]
    log_errors: bool,
    /// Path of transaction file
    path: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(args.path)?;

    let mut account_manager = AccountManager::new();

    for result in reader.deserialize() {
        let record: model::InputRecord = result?;
        match account_manager.process_record(&record) {
            Err(error) => {
                if args.log_errors {
                    eprintln!("Error processing record: `{record:?}`, reason: `{error}`)`");
                }
            }
            _ => {}
        }
    }

    let mut writer = csv::Writer::from_writer(std::io::stdout());
    account_manager
        .gather_output()
        .into_iter()
        .try_for_each(|record| writer.serialize(record))?;
    writer.flush()?;

    Ok(())
}
