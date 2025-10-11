use std::path::PathBuf;

use crate::parser::ast::Span;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("unexpected token: {0}")]
    Unexpected(&'static str),

    #[error("parse error")]
    Parse(#[from] ParseErrorBundle),

    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("other error: {0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub struct ParseErrorBundle {
    path: PathBuf,
    text: String,
    errors: Vec<ParseError>,
}

impl std::fmt::Display for ParseErrorBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in self.errors.iter() {
            let (line, column) = Self::offset_to_line_col(&self.text, error.span.start);
            writeln!(f, "{path}:{line}:{column}: {error_kind}", path = self.path.display(), error_kind = error.kind)?;
            if let Some((line_str, caret)) = Self::line_context(&self.text, error.span.start, error.span.end) {
                writeln!(f, "  {line_str}")?;
                writeln!(f, "  {caret}")?;
            }
        }

        Ok(())
    }
}

impl ParseErrorBundle {
    fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
        let clamped = offset.min(source.len());
        let line = source[..clamped].chars().filter(|&c| c == '\n').count() + 1;
        let line_start = source[..clamped].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let column = source[line_start..clamped].chars().count() + 1;
        (line, column)
    }

    fn line_context(source: &str, start: usize, end: usize) -> Option<(String, String)> {
        if source.is_empty() {
            return None;
        }
        let start = start.min(source.len());
        let end = end.min(source.len());

        let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = source[end..].find('\n').map(|i| end + i).unwrap_or_else(|| source.len());

        let line_slice = &source[line_start..line_end];
        let prefix = &source[line_start..start];
        let highlight = &source[start..end];

        let prefix_cols = prefix.chars().count();
        let highlight_cols = highlight.chars().count().max(1);

        let mut caret = String::new();
        caret.push_str(&" ".repeat(prefix_cols));
        caret.push('^');
        if highlight_cols > 1 {
            caret.push_str(&"~".repeat(highlight_cols - 1));
        }

        Some((line_slice.to_string(), caret))
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, start: usize, end: usize) -> Self {
        Self { kind, span: Span { start, end } }
    }
}

#[derive(Error, Debug)]
pub enum ParseErrorKind {
    #[error("unexpected token: {0}")]
    Unexpected(&'static str),

    #[error("expected {expected}, found {found}")]
    Expected { expected: &'static str, found: &'static str },

    #[error("invalid number literal")]
    InvalidNumber,

    #[error("duplicate attribute or item")]
    Duplicate,

    #[error("unterminated block or structure")]
    Unterminated,

    #[error("other error: {0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("設定ファイルを読み込めませんでした: {0}")]
    Io(#[from] std::io::Error),
    #[error("設定ファイルを解析できませんでした: {0}")]
    Parse(#[from] serde_yaml_ng::Error),
}
