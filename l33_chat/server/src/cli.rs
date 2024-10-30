use clap::{value_parser, Parser};
use std::net::Ipv4Addr;

#[derive(Parser)]
pub struct Cli {
    /// Listening IP
    #[clap(short, long, default_value = "0.0.0.0", env = "WBTECH_L33_IP")]
    pub ip: Ipv4Addr,

    /// Listening port
    #[clap(
        short,
        long,
        value_parser = value_parser!(u16).range(1..),
        default_value_t = 3000,
        env = "WBTECH_L33_PORT"
    )]
    pub port: u16,
}
