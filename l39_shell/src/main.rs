mod cli;
mod eval;
mod line;
mod parse;
mod scan;
mod var;

use crate::line::Line;
use anyhow::{bail, Result};
use eval::{eval, Output};
use parse::Expr;
use scan::MetaToken;
use std::io::{self, Write};
use var::Vars;

fn main() {
    repl();
}

fn repl() {
    let buf = &mut String::new();
    let vars = &mut Vars::default();

    loop {
        if let Err(why) = run(buf, vars) {
            eprintln!("error: {}", why);
        }
    }
}

fn run(buf: &mut String, vars: &mut Vars) -> Result<()> {
    prelude()?;

    let line = &read_line(buf)?;

    if line.is_empty() {
        return Ok(());
    }

    if let Output::Exit = scan(line)
        .and_then(parse)
        .and_then(|exprs| eval(exprs, vars))?
    {
        std::process::exit(0);
    }

    Ok(())
}

fn prelude() -> Result<(), io::Error> {
    io::stdout().write_all("₽ ".as_bytes())?;
    io::stdout().flush()
}

fn read_line(buf: &mut String) -> Result<Line<'_>> {
    io::stdin().read_line(buf)?;
    Ok(Line::drain(buf))
}

fn scan(line: &str) -> Result<Vec<MetaToken>> {
    let mut res = vec![];
    scan::scan(line, &mut res);
    let (results, tokens) = (
        res.len(),
        Vec::from_iter(res.into_iter().filter_map(|r| match r {
            Ok(token) => Some(token),
            Err(why) => {
                eprintln!("{}", why);
                None
            }
        })),
    );
    if tokens.len() < results {
        bail!("failed while scanning");
    } else {
        Ok(tokens)
    }
}

fn parse(tokens: Vec<MetaToken>) -> Result<Vec<Expr>> {
    let exprs = parse::parse(tokens)?;
    if exprs.is_empty() {
        bail!("expression not found");
    } else {
        Ok(exprs)
    }
}
