#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum FieldType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
    Bytes,
    String,
    Array,
    Map,
    Unknown { major: u8, info: u8 },
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Bool => write!(f, "bool"),
            FieldType::U8 => write!(f, "u8"),
            FieldType::U16 => write!(f, "u16"),
            FieldType::U32 => write!(f, "u32"),
            FieldType::U64 => write!(f, "u64"),
            FieldType::I8 => write!(f, "i8"),
            FieldType::I16 => write!(f, "i16"),
            FieldType::I32 => write!(f, "i32"),
            FieldType::I64 => write!(f, "i64"),
            FieldType::F16 => write!(f, "f16"),
            FieldType::F32 => write!(f, "f32"),
            FieldType::F64 => write!(f, "f64"),
            FieldType::Bytes => write!(f, "bytes"),
            FieldType::String => write!(f, "string"),
            FieldType::Array => write!(f, "array"),
            FieldType::Map => write!(f, "map"),
            FieldType::Unknown { major, info } => write!(f, "unknown(major={major}, info={info})"),
        }
    }
}
