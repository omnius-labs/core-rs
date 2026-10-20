#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, start: usize, end: usize) -> Self {
        Self { value, span: Span { start, end } }
    }
}

// ===== Top level =====

#[derive(Debug, Clone, Default)]
pub struct File {
    pub version: Option<Spanned<u32>>,
    pub package: Option<Spanned<Path>>,
    pub uses: Vec<Use>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Use {
    pub path: Spanned<Path>,
    pub alias: Option<Spanned<String>>,
}

// ===== attribute =====
//
// `#[namespace::name(arg1, arg2, ...)]` is written right before a struct/enum.
// The namespace names a code generation target (for example `rust`), and each generator
// ignores attributes whose namespace does not match its own.

#[derive(Debug, Clone)]
pub struct Attribute {
    pub path: Spanned<Path>,
    pub args: Vec<Spanned<String>>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Struct(Struct),
    Enum(Enum),
    TypeAlias(TypeAlias),
    Const(Const),
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Item::Struct(s) => write!(f, "struct {}", s.name.value),
            Item::Enum(e) => write!(f, "enum {}", e.name.value),
            Item::TypeAlias(t) => write!(f, "type {}", t.name.value),
            Item::Const(c) => write!(f, "const {}", c.name.value),
        }
    }
}

// ===== Type expressions =====

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<Spanned<String>>, // a::b::C
}

#[derive(Debug, Clone)]
pub enum Type {
    Path(Path), // bool, u32, MyType, pkg::T
    Option(Box<Type>),
    Vec(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Array(Box<Type>, u64), // [T; N]
    Constrained(Box<Type>, LengthRange),
}

#[derive(Debug, Clone)]
pub struct LengthRange {
    pub min: Option<Spanned<LengthBound>>,
    pub max: Spanned<LengthBound>,
}

#[derive(Debug, Clone)]
pub enum LengthBound {
    Literal(u128),
    Const(String),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Bool(bool),
    Int(u128),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Some(Box<Literal>),
    None,
}

// ===== struct =====

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: Spanned<String>,
    pub fields: Vec<Field>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub tag: Spanned<u32>, // @N
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
    pub default: Option<Spanned<Literal>>,
}

// ===== enum =====

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: Spanned<String>,
    pub variants: Vec<Variant>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub tag: Spanned<u32>,
    pub name: Spanned<String>,
    pub kind: VariantKind,
}

#[derive(Debug, Clone)]
pub enum VariantKind {
    Unit,                                         // @1 Foo;
    Tuple(Vec<(Spanned<String>, Spanned<Type>)>), // @2 Bar(x: T, y: U);
    Record(Vec<Field>),                           // @3 Baz { @1 x: T; ... }
}

// ===== type alias / const =====

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}

#[derive(Debug, Clone)]
pub struct Const {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
    pub value: Spanned<Literal>,
}
