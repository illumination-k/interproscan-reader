use std::{error::Error, fs::File, io::BufReader};

mod opt;
mod parser;
mod reader;

fn main() -> Result<(), Box<dyn Error>> {
    let input = "small.gff3";
    let bufreader = BufReader::new(File::open(input)?);

    let reader = reader::InterproGffReader::new(bufreader);
    let records = reader.finish()?;
    dbg!(&records);
    Ok(())
}
