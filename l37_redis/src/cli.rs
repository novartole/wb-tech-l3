use clap::{builder::NonEmptyStringValueParser, value_parser, Parser};
use std::net::Ipv4Addr;

#[derive(Parser)]
pub struct Cli {
    /// Listening IP
    #[clap(short, long, default_value = "0.0.0.0", env = "WBTECH_L37_IP")]
    pub ip: Ipv4Addr,

    /// Listening port
    #[clap(
        short,
        long,
        value_parser = value_parser!(u16).range(1..),
        default_value_t = 3000,
        env = "WBTECH_L37_PORT"
    )]
    pub port: u16,

    /// Database configuration string
    #[clap(
        long, 
        value_parser = NonEmptyStringValueParser::new(),
        env = "WBTECH_L37_DB_PARAMS",
    )]
    pub db_params: String,
}
