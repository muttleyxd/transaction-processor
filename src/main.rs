use csv::Trim;

mod model;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_path("transactions.csv")
        .unwrap();

    for result in reader.deserialize() {
        let record: model::InputRecord = result?;
        dbg!(&record);
    }

    Ok(())
}
