#[macro_use]
extern crate log;

use std::io::BufRead;
use std::{env::set_var, error::Error};
use structopt::StructOpt;

mod opt;
mod parser;
mod reader;

use crate::opt::{LogLevel, Opt};
use crate::parser::Expr;

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    match &opt.log_level {
        Some(log_level) => match log_level {
            LogLevel::DEBUG => set_var("RUST_LOG", "debug"),
            LogLevel::INFO => set_var("RUST_LOG", "info"),
            LogLevel::WARN => set_var("RUST_LOG", "warn"),
            LogLevel::ERROR => set_var("RUST_LOG", "error"),
        },
        None => set_var("RUST_LOG", "warn"),
    };

    pretty_env_logger::init_timed();
    debug!("{:?}", opt);

    let input = opt.input;
    let bufreader: Box<dyn BufRead> = reader::read_with_gz(&input)?;

    let reader = reader::InterproGffReader::new(bufreader)
        .with_comment(opt.comment)
        .with_max_length(opt.max_length)
        .with_min_length(opt.min_length)
        .with_id_expr(
            opt.id_expr
                .map(|s| Expr::from_string(&s).expect("Invalid id expr")),
        )
        .with_domain_expr(
            opt.domain_expr
                .map(|s| Expr::from_string(&s).expect("Invalid domain expr")),
        )
        .with_source_expr(
            opt.source_expr
                .clone()
                .map(|s| Expr::from_string(&s).expect("Invalid source expr")),
        );

    let records = reader.finish()?;

    let outformat = opt.out_format.unwrap_or(opt::OutputFormat::ID);

    match outformat {
        opt::OutputFormat::ID => {
            for record in records {
                println!("{}", record.id)
            }
        }
        opt::OutputFormat::ALL => {
            for record in records {
                println!("{}", record)
            }
        }
        opt::OutputFormat::TSV => {
            let expr = opt
                .source_expr
                .map(|s| Expr::from_string(&s).expect("Invalid source expr"));
            for record in records {
                println!("{}", record.to_tsv_record())
            }
        }
    }
    Ok(())
}
