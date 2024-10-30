use crate::scan::{MetaToken, Token};
use core::str;
use std::borrow::Cow;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("missing '{}' while parsing 'cmd [<,>,>>] fd'", missing)]
    BadExpression { missing: &'static str },
    #[error("expected <, >, or >>")]
    MissRedir,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug)]
pub struct Expr<'a> {
    pub cmd: Option<Cow<'a, str>>,
    pub redirs: Vec<(Redir, Cow<'a, str>)>,
}

#[derive(Debug)]
pub enum Redir {
    Input,
    Output,
    Append,
}

impl std::fmt::Display for Redir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Input => "input",
                Self::Output => "output",
                Self::Append => "append",
            }
        )
    }
}

pub fn parse(tokens: Vec<MetaToken<'_>>) -> Result<Vec<Expr<'_>>, ParseError> {
    let mut res = vec![];
    for mut tokens in squash(tokens).split(|meta| matches!(meta.token, Token::Pipe)) {
        let cmd = match tokens
            .split_first()
            .map(|(meta, elmnts)| (&meta.token, elmnts))
        {
            Some((Token::Input | Token::Output | Token::Append, _)) => None,
            Some((Token::String { lex }, values)) => {
                let cmd = Some(match lex {
                    Cow::Borrowed(lex) => Cow::from(*lex),
                    Cow::Owned(lex) => Cow::from(lex.clone()),
                });
                tokens = values;
                cmd
            }
            _ => return Err(ParseError::BadExpression { missing: "cmd+" }),
        };
        let mut redirs = vec![];
        for meta in tokens.chunks_exact(2) {
            let redir = match meta[0].token {
                Token::Input => Redir::Input,
                Token::Output => Redir::Output,
                Token::Append => Redir::Append,
                _ => return Err(ParseError::MissRedir),
            };
            if let Token::String { lex } = &meta[1].token {
                redirs.push((
                    redir,
                    match lex {
                        Cow::Borrowed(lex) => Cow::from(*lex),
                        Cow::Owned(lex) => Cow::from(lex.clone()),
                    },
                ));
            } else {
                return Err(ParseError::BadExpression { missing: "fd" });
            }
        }
        res.push(Expr { cmd, redirs });
    }
    Ok(res)
}

/// Squash neighbor [Token::String] into one instance.
fn squash(tokens: Vec<MetaToken<'_>>) -> Vec<MetaToken<'_>> {
    let mut iter = tokens.into_iter();
    let mut res = vec![];
    let mut acc = None;

    loop {
        match iter.by_ref().next() {
            Some(meta) => match meta.token {
                Token::String { lex } => match acc.as_mut() {
                    None => acc = Some((lex, meta.pos)),
                    Some((acc, _)) => acc.to_mut().extend([" ", lex.as_ref()]),
                },
                _ => {
                    if let Some((lex, pos)) = acc.take() {
                        res.push(MetaToken {
                            token: Token::String { lex },
                            pos,
                        });
                    }
                    res.push(meta);
                }
            },
            None => {
                if let Some((lex, pos)) = acc.take() {
                    res.push(MetaToken {
                        token: Token::String { lex },
                        pos,
                    });
                }
                break res;
            }
        }
    }
}
