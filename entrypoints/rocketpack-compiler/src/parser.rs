use crate::{
    error::{ParseError, ParseErrorKind},
    parser::{ast::*, lexer::*},
};

pub mod ast;
pub mod lexer;

pub fn parse(path: &Path) -> (File, Vec<ParseError>) {
    let (tokens, lex_errors) = lexer::lex("");

    if !lex_errors.is_empty() {
        return (File::default(), lex_errors);
    }

    let mut p = Parser::new(tokens);
    let file = p.parse_file();
    (file, p.errors)
}

pub struct Parser {
    tokens: Vec<SpannedToken>,
    i: usize,
    pub errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, errors: Vec::new(), i: 0 }
    }

    // ===== 基本ユーティリティ =====

    fn peek(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.i)
    }

    fn nth(&self, n: usize) -> Option<&SpannedToken> {
        self.tokens.get(self.i + n)
    }

    fn bump(&mut self) -> Option<SpannedToken> {
        let t = self.tokens.get(self.i).cloned();
        self.i += 1;
        t
    }

    fn at(&self, t: Token) -> bool {
        self.peek().map(|x| std::mem::discriminant(&x.token)) == Some(std::mem::discriminant(&t))
    }

    fn expect(&mut self, want: Token, name: &'static str) -> Option<SpannedToken> {
        if self.at(want.clone()) {
            self.bump()
        } else {
            let (s, e) = self.error_here(ParseErrorKind::Expected {
                expected: name,
                found: self.peek_name(),
            });
            Some(SpannedToken {
                token: want,
                span: Span { start: s, end: e },
            })
        }
    }

    fn peek_name(&self) -> &'static str {
        match self.peek().map(|t| &t.token) {
            Some(Token::Ident(_)) => "identifier",
            Some(Token::Int(_)) | Some(Token::Hex(_)) => "integer",
            Some(Token::Float(_)) => "float",
            Some(Token::Str(_)) => "string",
            Some(Token::Bytes(_)) => "bytes",
            Some(Token::At) => "@",
            Some(Token::Semi) => ";",
            Some(Token::Colon) => ":",
            Some(Token::Comma) => ",",
            Some(Token::Eq) => "=",
            Some(Token::LBrace) => "{",
            Some(Token::RBrace) => "}",
            Some(Token::LParen) => "(",
            Some(Token::RParen) => ")",
            Some(Token::LBracket) => "[",
            Some(Token::RBracket) => "]",
            Some(Token::Lt) => "<",
            Some(Token::Gt) => ">",
            Some(Token::Dots) => "..",
            Some(Token::PathSep) => "::",
            None => "EOF",
        }
    }

    fn error_here(&mut self, kind: ParseErrorKind) -> (usize, usize) {
        let (s, e) = if let Some(t) = self.peek() { (t.span.start, t.span.end) } else { (0, 0) };
        self.errors.push(ParseError::new(kind, s, e));
        (s, e)
    }

    // ===== エントリ =====

    pub fn parse_file(&mut self) -> File {
        let mut file = File::default();

        // 任意の順序でトップレベルを読み込む
        while let Some(_) = self.peek() {
            // キーワードは Ident で来るので先読みして判定
            if let Some(kw) = self.peek_keyword() {
                match kw.as_str() {
                    "version" => {
                        if file.version.is_some() {
                            let (s, e) = self.error_here(ParseErrorKind::Duplicate);
                            // 食い進めておく
                            self.parse_version();
                            // 破棄
                            let _ = (s, e);
                        } else {
                            file.version = Some(self.parse_version());
                        }
                    }
                    "package" => {
                        if file.package.is_some() {
                            let (s, e) = self.error_here(ParseErrorKind::Duplicate);
                            let _ = self.parse_package();
                            let _ = (s, e);
                        } else {
                            file.package = Some(self.parse_package());
                        }
                    }
                    "use" => {
                        file.uses.push(self.parse_use());
                    }
                    "struct" => {
                        file.items.push(Item::Struct(self.parse_struct()));
                    }
                    "enum" => {
                        file.items.push(Item::Enum(self.parse_enum()));
                    }
                    "type" => {
                        file.items.push(Item::TypeAlias(self.parse_type_alias()));
                    }
                    "const" => {
                        file.items.push(Item::Const(self.parse_const()));
                    }
                    _ => {
                        // 未対応トップレベル
                        self.error_here(ParseErrorKind::Unexpected("unknown top-level"));
                        self.bump();
                    }
                }
            } else {
                // 何かしらのトークンを消費して前に進む
                self.error_here(ParseErrorKind::Unexpected("unexpected token at top-level"));
                self.bump();
            }
        }

        file
    }

    fn peek_keyword(&self) -> Option<String> {
        match self.peek()?.token.clone() {
            Token::Ident(s) => Some(s),
            _ => None,
        }
    }

    // ===== トップレベル要素 =====

    fn parse_version(&mut self) -> Spanned<u32> {
        let start = self.expect_ident_kw("version").span.start;
        let version = self.expect_int_u32();
        self.expect(Token::Semi, ";");
        Spanned::new(version, start, self.prev_end())
    }

    fn parse_package(&mut self) -> Spanned<Path> {
        let start = self.expect_ident_kw("package").span.start;
        let path = self.parse_path();
        self.expect(Token::Semi, ";");
        Spanned::new(path, start, self.prev_end())
    }

    fn parse_use(&mut self) -> Use {
        let _start = self.expect_ident_kw("use").span.start;
        let path = Spanned::new(self.parse_path(), self.prev_start(), self.prev_end());
        // MVP: `as Alias` のみ対応
        let alias = if self.is_ident_kw("as") {
            let a = self.expect_ident();
            Some(a)
        } else {
            None
        };
        self.expect(Token::Semi, ";");
        Use { path, alias }
    }

    fn parse_struct(&mut self) -> Struct {
        let _kw = self.expect_ident_kw("struct");
        let name = self.expect_ident();
        self.expect(Token::LBrace, "{");
        let mut fields = Vec::new();
        while !self.at(Token::RBrace) && self.peek().is_some() {
            if self.at(Token::At) {
                fields.push(self.parse_field());
            } else {
                self.error_here(ParseErrorKind::Unexpected("expected field or reserved"));
                self.bump();
            }
        }
        self.expect(Token::RBrace, "}");
        Struct { name, fields }
    }

    fn parse_enum(&mut self) -> Enum {
        let _kw = self.expect_ident_kw("enum");
        let name = self.expect_ident();
        self.expect(Token::LBrace, "{");
        let mut variants = Vec::new();
        while !self.at(Token::RBrace) && self.peek().is_some() {
            if self.at(Token::At) {
                variants.push(self.parse_variant());
            } else {
                self.error_here(ParseErrorKind::Unexpected("expected @tag for variant"));
                self.bump();
            }
        }
        self.expect(Token::RBrace, "}");
        Enum { name, variants }
    }

    fn parse_type_alias(&mut self) -> TypeAlias {
        let _kw = self.expect_ident_kw("type");
        let name = self.expect_ident();
        self.expect(Token::Eq, "=");
        let ty = self.expect_type();
        self.expect(Token::Semi, ";");
        TypeAlias { name, ty }
    }

    fn parse_const(&mut self) -> Const {
        let _kw = self.expect_ident_kw("const");
        let name = self.expect_ident();
        self.expect(Token::Colon, ":");
        let ty = self.expect_type();
        self.expect(Token::Eq, "=");
        let value = self.expect_literal();
        self.expect(Token::Semi, ";");
        Const { name, ty, value }
    }

    // ===== struct: field / reserved =====

    fn parse_field(&mut self) -> Field {
        let _at = self.expect(Token::At, "@").unwrap();
        let tag = self.expect_int_u32_spanned();
        let name = self.expect_ident();
        self.expect(Token::Colon, ":");
        let ty = self.expect_type();
        let default = if self.at(Token::Eq) {
            self.bump();
            Some(self.expect_literal())
        } else {
            None
        };
        self.expect(Token::Semi, ";");
        Field { tag, name, ty, default }
    }

    // ===== enum: variant =====

    fn parse_variant(&mut self) -> Variant {
        let _at = self.expect(Token::At, "@");
        let tag = self.expect_int_u32_spanned();
        let name = self.expect_ident();
        let kind = if self.at(Token::Semi) {
            self.bump();
            VariantKind::Unit
        } else if self.at(Token::LParen) {
            let params = self.parse_tuple_params();
            self.expect(Token::Semi, ";");
            VariantKind::Tuple(params)
        } else if self.at(Token::LBrace) {
            // レコード: 中身は struct の field と同じ書式（@番号付き）
            self.bump();
            let mut fields = Vec::new();
            while !self.at(Token::RBrace) && self.peek().is_some() {
                if self.at(Token::At) {
                    fields.push(self.parse_field());
                } else {
                    self.error_here(ParseErrorKind::Unexpected("expected @tag inside record variant"));
                    self.bump();
                }
            }
            self.expect(Token::RBrace, "}");
            self.expect(Token::Semi, ";");
            VariantKind::Record(fields)
        } else {
            self.error_here(ParseErrorKind::Unexpected("expected ';', '(' or '{' after variant name"));
            VariantKind::Unit
        };
        Variant { tag, name, kind }
    }

    fn parse_tuple_params(&mut self) -> Vec<(Spanned<String>, Spanned<Type>)> {
        self.expect(Token::LParen, "(");
        let mut v = Vec::new();
        if !self.at(Token::RParen) {
            loop {
                let name = self.expect_ident();
                self.expect(Token::Colon, ":");
                let ty = self.expect_type();
                v.push((name, ty));
                if self.at(Token::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen, ")");
        v
    }

    // ===== 型 =====

    fn expect_type(&mut self) -> Spanned<Type> {
        let start = self.curr_start();
        let ty = self.parse_type_inner();
        let end = self.prev_end();
        Spanned::new(ty, start, end)
    }

    fn parse_type_inner(&mut self) -> Type {
        // Option<T> / Vec<T> / Map<K,V> / [T;N] / Path
        if self.is_ident_kw("Option") {
            self.expect(Token::Lt, "<");
            let inner = self.expect_type();
            self.expect(Token::Gt, ">");
            Type::Option(Box::new(inner.value))
        } else if self.is_ident_kw("Vec") {
            self.expect(Token::Lt, "<");
            let inner = self.expect_type();
            self.expect(Token::Gt, ">");
            Type::Vec(Box::new(inner.value))
        } else if matches!(self.peek_keyword().as_deref(), Some("map") | Some("Map")) {
            let _ = self.expect_ident();
            self.expect(Token::Lt, "<");
            let k = self.expect_type();
            self.expect(Token::Comma, ",");
            let v = self.expect_type();
            self.expect(Token::Gt, ">");
            Type::Map(Box::new(k.value), Box::new(v.value))
        } else if self.at(Token::LBracket) {
            self.bump();
            let inner = self.expect_type();
            self.expect(Token::Semi, ";");
            let n = self.expect_int_u64();
            self.expect(Token::RBracket, "]");
            Type::Array(Box::new(inner.value), n)
        } else {
            Type::Path(self.parse_path())
        }
    }

    fn parse_path(&mut self) -> Path {
        let mut segs = Vec::new();
        segs.push(self.expect_ident());
        while self.at(Token::PathSep) {
            self.bump();
            segs.push(self.expect_ident());
        }
        Path { segments: segs }
    }

    // ===== リテラル =====

    fn expect_literal(&mut self) -> Spanned<Literal> {
        let (start, val, end) = match self.peek().cloned() {
            Some(SpannedToken { token: Token::Ident(s), span }) if s == "true" || s == "false" => {
                self.bump();
                (span.start, Literal::Bool(s == "true"), span.end)
            }
            Some(SpannedToken { token: Token::Int(n), span }) => {
                self.bump();
                (span.start, Literal::Int(n), span.end)
            }
            Some(SpannedToken { token: Token::Hex(n), span }) => {
                self.bump();
                (span.start, Literal::Int(n), span.end)
            }
            Some(SpannedToken { token: Token::Float(f), span }) => {
                self.bump();
                (span.start, Literal::Float(f), span.end)
            }
            Some(SpannedToken { token: Token::Str(s), span }) => {
                self.bump();
                (span.start, Literal::String(s), span.end)
            }
            Some(SpannedToken { token: Token::Bytes(b), span }) => {
                self.bump();
                (span.start, Literal::Bytes(b), span.end)
            }
            _ => {
                let (s, e) = self.error_here(ParseErrorKind::Expected {
                    expected: "literal",
                    found: self.peek_name(),
                });
                (s, Literal::Int(0), e)
            }
        };
        Spanned::new(val, start, end)
    }

    // ===== 諸々の expect/状態 =====

    fn expect_ident(&mut self) -> Spanned<String> {
        match self.bump() {
            Some(t) => match t.token {
                Token::Ident(s) => Spanned::new(s, t.span.start, t.span.end),
                _ => {
                    let (s, e) = self.error_here(ParseErrorKind::Expected {
                        expected: "identifier",
                        found: self.peek_name(),
                    });
                    Spanned::new("_".to_string(), s, e)
                }
            },
            None => {
                let (s, e) = self.error_here(ParseErrorKind::Expected {
                    expected: "identifier",
                    found: "EOF",
                });
                Spanned::new("_".to_string(), s, e)
            }
        }
    }

    fn is_ident_kw(&mut self, kw: &str) -> bool {
        if let Some(SpannedToken { token: Token::Ident(s), .. }) = self.peek()
            && s == kw
        {
            self.bump();
            return true;
        }
        false
    }
    fn expect_ident_kw(&mut self, kw: &'static str) -> Spanned<String> {
        let t = self.expect_ident();
        if t.value != kw {
            self.errors.push(ParseError::new(
                ParseErrorKind::Expected {
                    expected: kw,
                    found: "identifier",
                },
                t.span.start,
                t.span.end,
            ));
        }
        t
    }

    fn expect_int_u32(&mut self) -> u32 {
        match self.bump() {
            Some(t) => match t.token {
                Token::Int(n) | Token::Hex(n) => n as u32,
                _ => {
                    self.error_here(ParseErrorKind::Expected {
                        expected: "integer",
                        found: self.peek_name(),
                    });
                    0
                }
            },
            None => {
                self.error_here(ParseErrorKind::Expected {
                    expected: "integer",
                    found: "EOF",
                });
                0
            }
        }
    }
    fn expect_int_u32_spanned(&mut self) -> Spanned<u32> {
        match self.bump() {
            Some(t) => match t.token {
                Token::Int(n) | Token::Hex(n) => Spanned::new(n as u32, t.span.start, t.span.end),
                _ => {
                    let (s, e) = self.error_here(ParseErrorKind::Expected {
                        expected: "integer",
                        found: self.peek_name(),
                    });
                    Spanned::new(0, s, e)
                }
            },
            None => {
                let (s, e) = self.error_here(ParseErrorKind::Expected {
                    expected: "integer",
                    found: "EOF",
                });
                Spanned::new(0, s, e)
            }
        }
    }
    fn expect_int_u64(&mut self) -> u64 {
        match self.bump() {
            Some(t) => match t.token {
                Token::Int(n) | Token::Hex(n) => n as u64,
                _ => {
                    self.error_here(ParseErrorKind::Expected {
                        expected: "integer",
                        found: self.peek_name(),
                    });
                    0
                }
            },
            None => {
                self.error_here(ParseErrorKind::Expected {
                    expected: "integer",
                    found: "EOF",
                });
                0
            }
        }
    }

    fn curr_start(&self) -> usize {
        self.peek().map(|t| t.span.start).unwrap_or(0)
    }
    fn prev_start(&self) -> usize {
        if self.i == 0 { 0 } else { self.tokens[self.i - 1].span.start }
    }
    fn prev_end(&self) -> usize {
        if self.i == 0 { 0 } else { self.tokens[self.i - 1].span.end }
    }
}
