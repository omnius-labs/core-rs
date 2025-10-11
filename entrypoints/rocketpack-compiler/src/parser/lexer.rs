use logos::Logos;

use crate::{
    error::{ParseError, ParseErrorKind},
    parser::ast::Span,
};

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*([^*]|\*[^/])*\*/")]
pub enum Token {
    // Separators / punctuation
    #[token("@")]
    At,
    #[token(";")]
    Semi,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("=")]
    Eq,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("..")]
    Dots,
    #[token("::")]
    PathSep,

    // Literals
    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    Str(String),

    #[regex(r#"b"([^"\\]|\\.)*""#, parse_bytes)]
    Bytes(Vec<u8>),

    #[regex(r"[0-9][_0-9]*", parse_int)]
    Int(u128),
    #[regex(r"0x[0-9A-Fa-f][_0-9A-Fa-f]*", parse_hex)]
    Hex(u128),
    #[regex(r"[0-9][_0-9]*\.[0-9][_0-9]*([eE][+-]?[0-9][_0-9]*)?", parse_float)]
    Float(f64),

    // Identifiers
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", parse_ident, priority = 2)]
    Ident(String),
}

fn parse_string(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let slice = lex.slice();
    let raw = &slice[1..slice.len() - 1];
    Some(parse_string_literal(raw))
}

fn parse_bytes(lex: &mut logos::Lexer<Token>) -> Option<Vec<u8>> {
    let slice = lex.slice();
    let raw = &slice[2..slice.len() - 1];
    Some(parse_bytes_literal(raw))
}

fn parse_string_literal(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('x') => {
                    let hi = it.next();
                    let lo = it.next();
                    if let (Some(hi), Some(lo)) = (hi, lo) {
                        if let Some(val) = hex_pair(hi, lo) {
                            if let Some(ch) = char::from_u32(val as u32) {
                                out.push(ch);
                                continue;
                            }
                        }
                        out.push('\\');
                        out.push('x');
                        out.push(hi);
                        out.push(lo);
                    } else {
                        out.push('\\');
                        out.push('x');
                        if let Some(hi) = hi {
                            out.push(hi);
                        }
                        if let Some(lo) = lo {
                            out.push(lo);
                        }
                    }
                }
                Some(x) => {
                    out.push('\\');
                    out.push(x);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_bytes_literal(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push(b'\n'),
                Some('r') => out.push(b'\r'),
                Some('t') => out.push(b'\t'),
                Some('"') => out.push(b'"'),
                Some('\\') => out.push(b'\\'),
                Some('x') => {
                    let hi = it.next();
                    let lo = it.next();
                    if let (Some(hi), Some(lo)) = (hi, lo) {
                        if let Some(val) = hex_pair(hi, lo) {
                            out.push(val);
                            continue;
                        }
                        out.push(b'\\');
                        out.push(b'x');
                        out.push(hi as u8);
                        out.push(lo as u8);
                    } else {
                        out.push(b'\\');
                        out.push(b'x');
                        if let Some(hi) = hi {
                            out.push(hi as u8);
                        }
                        if let Some(lo) = lo {
                            out.push(lo as u8);
                        }
                    }
                }
                Some(x) => {
                    out.push(b'\\');
                    out.push(x as u8);
                }
                None => out.push(b'\\'),
            }
        } else {
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            out.extend_from_slice(encoded.as_bytes());
        }
    }
    out
}

fn hex_pair(hi: char, lo: char) -> Option<u8> {
    let high = hex_digit(hi)?;
    let low = hex_digit(lo)?;
    Some((high << 4) | low)
}

fn hex_digit(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'a'..='f' => Some(10 + (c as u8 - b'a')),
        'A'..='F' => Some(10 + (c as u8 - b'A')),
        _ => None,
    }
}

fn parse_int(lex: &mut logos::Lexer<Token>) -> Option<u128> {
    let s = lex.slice().replace('_', "");
    s.parse().ok()
}
fn parse_hex(lex: &mut logos::Lexer<Token>) -> Option<u128> {
    let s = lex.slice().trim_start_matches("0x").replace('_', "");
    u128::from_str_radix(&s, 16).ok()
}
fn parse_float(lex: &mut logos::Lexer<Token>) -> Option<f64> {
    let s = lex.slice().replace('_', "");
    s.parse().ok()
}
fn parse_ident(lex: &mut logos::Lexer<Token>) -> Option<String> {
    Some(lex.slice().to_owned())
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub fn lex(input: &str) -> (Vec<SpannedToken>, Vec<ParseError>) {
    let mut errors = Vec::new();
    let mut lexer = Token::lexer(input);
    let mut out = Vec::new();
    while let Some(result) = lexer.next() {
        let range = lexer.span();
        match result {
            Ok(tok) => out.push(SpannedToken {
                token: tok,
                span: Span {
                    start: range.start,
                    end: range.end,
                },
            }),
            Err(_) => {
                errors.push(ParseError::new(ParseErrorKind::Unexpected("invalid token"), range.start, range.end));
            }
        }
    }
    (out, errors)
}
