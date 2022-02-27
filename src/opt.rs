use std::path::PathBuf;
use structopt::{clap, clap::arg_enum, StructOpt};

#[derive(Debug, StructOpt)]
#[structopt(name = "ir")]
#[structopt(long_version(option_env!("LONG_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))))]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub struct Opt {
    #[structopt(long = "log-level", possible_values(&LogLevel::variants()))]
    pub log_level: Option<LogLevel>,
    #[structopt(long = "input", short = "i")]
    pub input: PathBuf,
    #[structopt(long = "id-expr")]
    pub id_expr: Option<String>,
    #[structopt(long = "domain-expr")]
    pub domain_expr: Option<String>,
    #[structopt(long = "source-expr")]
    pub source_expr: Option<String>,
    #[structopt(long = "comment", default_value = "#")]
    pub comment: char,
    #[structopt(long = "min-length")]
    pub min_length: Option<u64>,
    #[structopt(long = "max-length")]
    pub max_length: Option<u64>,
}

arg_enum! {
    #[derive(Debug)]
    pub enum LogLevel {
        DEBUG,
        INFO,
        WARN,
        ERROR,
    }
}

arg_enum! {
    #[derive(Debug)]
    pub enum OutputFormat {
        ID,
        ALL,
    }
}
