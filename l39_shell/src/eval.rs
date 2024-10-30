use crate::{
    cli::{Cli, Command},
    parse::{Expr, Redir},
    var::Vars,
};
use anyhow::{anyhow, bail, Result};
use std::{
    borrow::Cow,
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    process::{self, Stdio},
    thread,
};

pub enum Output {
    Norm,
    Exit,
}

const INPUTS: &str = "input";
const OUTPUTS: &str = "output";
const APPEDNDS: &str = "append";

fn cd(path: Option<PathBuf>) -> Result<()> {
    let path = path.as_ref().map(AsRef::as_ref).unwrap_or(Path::new("~"));

    let target = match path.to_str() {
        Some("~") => env::var("HOME").map(PathBuf::from).map(Cow::Owned)?,
        _ => path.into(),
    };

    if let Err(e) = env::set_current_dir(&target) {
        if let io::ErrorKind::NotFound = e.kind() {
            bail!("{}: no such file or directory", target.to_string_lossy());
        };

        return Err(anyhow!(e));
    }

    Ok(())
}

fn ls(path: Option<PathBuf>) -> Result<String> {
    let path = path.as_ref().map(AsRef::as_ref).unwrap_or(Path::new(""));

    let target = match path.to_str() {
        Some("") => env::current_dir()?,
        _ => path.into(),
    };

    let mut buf = String::new();
    for try_entry in fs::read_dir(target).unwrap() {
        let entry = try_entry.unwrap();
        let s = entry.file_name().to_string_lossy().to_string();
        buf.extend([s.as_str(), "\n"]);
    }

    Ok(buf)
}

fn echo(s: Option<String>) -> Result<String> {
    Ok(s.unwrap_or_default())
}

fn pwd() -> Result<String> {
    Ok(env::current_dir().map(|cur_dir| format!("{}", cur_dir.display()))?)
}

fn display(s: String) {
    println!("{}", s);
}

fn exec(program: String, args: Vec<String>, redirs: Vec<(Redir, Cow<'_, str>)>) -> Result<()> {
    let mut rds = HashMap::new();
    for (r, s) in redirs {
        let key = match r {
            Redir::Input => INPUTS,
            Redir::Output => OUTPUTS,
            Redir::Append => APPEDNDS,
        };
        match rds.get_mut(&key) {
            None => {
                rds.insert(key, vec![s]);
            }
            Some(vec) => vec.push(s),
        };
    }

    let mut cmd = process::Command::new(program);
    cmd.args(args).stdout(Stdio::piped());
    if rds.get(INPUTS).is_some_and(|vec| !vec.is_empty()) {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn()?;

    let i_reader = if let Some(mut child_input) = child.stdin.take() {
        Some(thread::spawn({
            let mut readers = vec![];
            for s in rds.remove(&INPUTS).unwrap() {
                let path = Path::new(s.as_ref());
                let reader = File::open(path).map(BufReader::new)?;
                readers.push(reader);
            }
            move || {
                for r in readers {
                    for try_line in r.lines() {
                        let mut line = try_line.unwrap();
                        line.push('\n');
                        child_input.write_all(line.as_bytes()).unwrap();
                    }
                }
            }
        }))
    } else {
        None
    };

    let mut o_writter = if let Some(vec) = rds.remove(&OUTPUTS) {
        let mut writters = vec![];
        for s in vec {
            let path = Path::new(s.as_ref());
            let w = File::create(path).map(BufWriter::new)?;
            writters.push(w);
        }
        Some(writters)
    } else {
        None
    };
    let mut a_writter = if let Some(vec) = rds.remove(&APPEDNDS) {
        let mut writters = vec![];
        for s in vec {
            let path = Path::new(s.as_ref());
            let w = File::options()
                .append(true)
                .open(path)
                .map(BufWriter::new)?;
            writters.push(w);
        }
        Some(writters)
    } else {
        None
    };

    let child_output = child.stdout.take().unwrap();
    let reader = BufReader::new(child_output);
    for try_line in reader.lines() {
        let line = try_line?;

        if let Some(writters) = o_writter.as_mut() {
            for w in writters {
                writeln!(w, "{}", line)?;
            }
        }
        if let Some(writters) = a_writter.as_mut() {
            for w in writters {
                writeln!(w, "{}", line)?;
            }
        }

        if o_writter.is_none() && a_writter.is_none() {
            display(line);
        }
    }
    if let Some(handler) = i_reader {
        handler.join().expect("failed joining handler");
    }
    Ok(())
}

pub fn eval(exprs: Vec<Expr<'_>>, session_vars: &mut Vars) -> Result<Output> {
    use Command::*;

    if exprs.is_empty() {
        bail!("no expression");
    } else if exprs.len() == 1 {
        // Safety: len == 1, so unwrap is OK.
        let Expr { cmd, redirs } = exprs.into_iter().next().unwrap();
        return match cmd {
            Some(s) => match Cli::try_from(s.as_ref())?.cmd {
                Cd { path } => cd(path),
                Ls { path } => ls(path).map(display),
                Echo { s } => echo(s).map(display),
                Pwd => pwd().map(display),
                Exec { program, args } => {
                    exec(program, args, redirs)?;
                    return Ok(Output::Exit);
                }
                External { program, args } => {
                    exec(program, args, redirs)?;
                    return Ok(Output::Norm);
                }
                Export { n, vars } => {
                    if n {
                        session_vars.remove(vars);
                    } else {
                        session_vars.append(vars);
                    };
                    Ok(())
                }
                Exit => return Ok(Output::Exit),
            },
            None => todo!("to allow usage of $<.. and $>.. to get/set STDIO/STDOU directly"),
        }
        .and(Ok(Output::Norm));
    }

    let mut children = vec![];
    let program = env::current_exe()?;
    let last = exprs.len() - 1;
    for (i, Expr { cmd, redirs }) in exprs.into_iter().enumerate() {
        let mut sub_cmd = process::Command::new(&program);
        let args = cmd.expect("empty command not supported yet");
        let mut sub_arg = vec!["exec".to_string()];
        let sub_args = shlex::split(args.as_ref()).ok_or(anyhow!("command not found"))?;
        let sub_redirs: Vec<_> = redirs
            .into_iter()
            .flat_map(|(r, s)| {
                [
                    match r {
                        Redir::Input => "<",
                        Redir::Output => ">",
                        Redir::Append => ">>",
                    }
                    .to_string(),
                    s.to_string(),
                ]
            })
            .collect();
        sub_arg.extend(sub_args);
        sub_arg.extend(sub_redirs);

        sub_cmd.stdin(Stdio::piped());
        if i != last {
            sub_cmd.stdout(Stdio::piped());
        }

        let child = sub_cmd.spawn()?;
        children.push((child, sub_arg.join(" ")));
    }

    let mut hs = vec![];
    let mut output = None;
    for (i, (child, args)) in children.iter_mut().enumerate() {
        child.stdin.as_mut().unwrap().write_all(args.as_bytes())?;

        if i != 0 {
            let mut cur_in = child.stdin.take().unwrap();
            let prev_out = output.take().unwrap();
            let h = thread::spawn(move || {
                let r = BufReader::new(prev_out);
                for try_line in r.lines() {
                    writeln!(cur_in, "{}", try_line.unwrap()).unwrap();
                }
            });
            hs.push(h);
        }
        if i != last {
            let cur_out = child.stdout.take().unwrap();
            output = Some(cur_out);
        }
    }

    for h in hs {
        h.join().unwrap();
    }

    Ok(Output::Norm)
}
