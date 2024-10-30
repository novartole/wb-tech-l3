use crate::var::Var;
use anyhow::anyhow;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(multicall = true)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Cd {
        path: Option<PathBuf>,
    },
    Ls {
        path: Option<PathBuf>,
    },
    Echo {
        s: Option<String>,
    },
    Pwd,
    Exec {
        program: String,
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(skip)]
    External {
        program: String,
        args: Vec<String>,
    },
    Export {
        #[arg(short)]
        n: bool,
        #[arg(required_if_eq("n", "true"))]
        vars: Vec<Var>,
    },
    Exit,
}

impl TryFrom<&str> for Cli {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let args = shlex::split(value).ok_or(anyhow!("command not found"))?;
        let cmd = Self::try_parse_from(&args).map(|cli| cli.cmd).unwrap_or({
            let program = args[0].to_owned();
            let args = args[1..].iter().map(String::to_owned).collect();
            Command::External { program, args }
        });
        Ok(Self { cmd })
    }
}
