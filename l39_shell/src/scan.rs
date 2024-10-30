use anyhow::{anyhow, Result};
use std::borrow::Cow;

#[derive(Debug)]
pub enum Token<'a> {
    Eos,
    Pipe,
    Input,
    Output,
    Append,
    String { lex: Cow<'a, str> },
}

#[derive(Debug)]
pub struct MetaToken<'a> {
    pub token: Token<'a>,
    pub pos: usize,
}

pub fn scan<'a>(mut content: &'a str, tokens: &mut Vec<Result<MetaToken<'a>>>) {
    let mut m_pos = 0;
    let mut rest;

    loop {
        if content.is_empty() {
            break tokens.push(Ok(MetaToken {
                token: Token::Eos,
                pos: m_pos,
            }));
        }

        if let Some(token) = {
            let (token, suf) = {
                let (ch, mut rest) = content.split_at(1);
                (
                    match ch {
                        " " | "\t" => None,
                        "|" => Some(Ok(Token::Pipe)),
                        "<" => Some(Ok(Token::Input)),
                        ">" => Some(Ok(match rest.strip_prefix(">") {
                            Some(suf) => {
                                rest = suf;
                                Token::Append
                            }
                            None => Token::Output,
                        })),
                        "'" | "\"" => Some({
                            let quote = ch.chars().next().unwrap();
                            let mut prev = quote;
                            let mut pos = None;

                            for (i, cur) in rest.char_indices() {
                                if cur == quote && prev != '\\' {
                                    pos = Some(i);
                                    break;
                                }
                                prev = cur;
                            }

                            match pos {
                                Some(mid) => {
                                    let lex = Cow::from(&content[..=mid + 1]);
                                    rest = &rest[mid + 1..];
                                    Ok(Token::String { lex })
                                }
                                None => {
                                    rest = "";
                                    Err(anyhow!("[pos {}]: unterminated string", m_pos))
                                }
                            }
                        }),
                        _ => Some({
                            let mut prev = ch.chars().next().unwrap();
                            let mut pos = None;

                            for (i, cur) in rest.char_indices() {
                                if ['|', '<', '>', '"', '\'', '(', ')', ' '].contains(&cur)
                                    && prev != '\\'
                                {
                                    pos = Some(i);
                                    break;
                                }
                                prev = cur;
                            }

                            let lex = Cow::from(match pos {
                                Some(mid) => {
                                    rest = &rest[mid..];
                                    &content[..mid + 1]
                                }
                                None => {
                                    rest = "";
                                    content
                                }
                            });
                            Ok(Token::String { lex })
                        }),
                    },
                    rest,
                )
            };
            rest = Some(suf);
            token
        } {
            tokens.push(token.map(|token| MetaToken { token, pos: m_pos }));
        }

        match rest.take() {
            Some(rest) => {
                m_pos += content.len() - rest.len();
                content = rest;
            }
            None => println!("unprocessed content left"),
        }
    }
}
