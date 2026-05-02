use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    path::{Path as FsPath, PathBuf},
};

use serde_yaml_ng::{Mapping, Value};
use tracing::info;

use crate::{
    config::{GeneratorConfig, GeneratorTargetConfig, SourceConfig},
    error::CodegenError,
    parser::{
        self,
        ast::{Const, Enum, Field, File, Item, Literal, Path as AstPath, Struct, Type, Use, VariantKind},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiscoveredSource {
    base_dir: PathBuf,
    absolute_path: PathBuf,
    relative_path: PathBuf,
}

#[derive(Debug, Clone)]
struct ParsedSource {
    source: DiscoveredSource,
    file: File,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GeneratedRustFile {
    source: DiscoveredSource,
    contents: String,
}

#[derive(Debug, Clone, Default)]
struct SchemaIndex {
    package: Vec<String>,
    uses: Vec<UseBinding>,
    imported_paths: BTreeMap<String, Vec<String>>,
    type_aliases: BTreeMap<String, Type>,
    user_types: BTreeMap<String, NamedTypeKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UseBinding {
    path: Vec<String>,
    alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NamedTypeKind {
    Struct,
    Enum,
    Alias,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BuiltinType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    String,
    Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NamedType {
    rust_path: Vec<String>,
    kind: NamedTypeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedType {
    Builtin(BuiltinType),
    Named(NamedType),
    Option(Box<ResolvedType>),
    Vec(Box<ResolvedType>),
    Map(Box<ResolvedType>, Box<ResolvedType>),
    Array(Box<ResolvedType>, u64),
}

pub async fn generate(root_dir: &FsPath, sources: &[SourceConfig], conf: &GeneratorConfig) -> Result<(), CodegenError> {
    let discovered_sources = discover_source_files(root_dir, sources)?;
    let parsed_sources = parse_sources(&discovered_sources)?;
    let generated_files = render_sources(&parsed_sources)?;
    let written_count = write_generated_files(root_dir, conf, &generated_files)?;

    info!(
        generator_id = %conf.id,
        plugin = %conf.plugin,
        source_count = parsed_sources.len(),
        rendered_count = generated_files.len(),
        written_count,
        "generated rocketpack rust files"
    );

    Ok(())
}

fn parse_sources(sources: &[DiscoveredSource]) -> Result<Vec<ParsedSource>, CodegenError> {
    let mut parsed_sources = Vec::with_capacity(sources.len());

    for source in sources {
        let file = parser::parse(&source.absolute_path)?;
        parsed_sources.push(ParsedSource { source: source.clone(), file });
    }

    Ok(parsed_sources)
}

fn render_sources(parsed_sources: &[ParsedSource]) -> Result<Vec<GeneratedRustFile>, CodegenError> {
    let mut generated_files = Vec::with_capacity(parsed_sources.len());

    for parsed_source in parsed_sources {
        generated_files.push(GeneratedRustFile {
            source: parsed_source.source.clone(),
            contents: render_rust_file(parsed_source)?,
        });
    }

    Ok(generated_files)
}

fn write_generated_files(root_dir: &FsPath, conf: &GeneratorConfig, generated_files: &[GeneratedRustFile]) -> Result<usize, CodegenError> {
    let mut written_count = 0usize;

    for generated_file in generated_files {
        let Some(output_path) = resolve_output_path(root_dir, conf, &generated_file.source)? else {
            info!(
                generator_id = %conf.id,
                source = normalize_path(&generated_file.source.relative_path),
                "skip rust source without matching target"
            );
            continue;
        };

        if let Some(parent_dir) = output_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        fs::write(&output_path, &generated_file.contents)?;
        written_count += 1;
    }

    Ok(written_count)
}

fn resolve_output_path(root_dir: &FsPath, conf: &GeneratorConfig, source: &DiscoveredSource) -> Result<Option<PathBuf>, CodegenError> {
    let relative_path = normalize_path(&source.relative_path);
    let Some(target) = conf.targets.iter().find(|target| glob_matches(&target.pattern, &relative_path)) else {
        return Ok(None);
    };

    let dir = target_option_dir(target)?.ok_or_else(|| CodegenError::Other(format!("missing dir option for rust target pattern: {}", target.pattern)))?;
    let file_stem = source
        .relative_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| CodegenError::Other(format!("invalid source file name: {}", source.relative_path.display())))?;

    Ok(Some(root_dir.join(dir).join(format!("{file_stem}.rs"))))
}

fn target_option_dir(target: &GeneratorTargetConfig) -> Result<Option<&str>, CodegenError> {
    match &target.options {
        Some(options) => mapping_string(options, "dir"),
        None => Ok(None),
    }
}

fn mapping_string<'a>(mapping: &'a Mapping, key: &str) -> Result<Option<&'a str>, CodegenError> {
    let Some(value) = mapping.get(Value::String(key.to_string())) else {
        return Ok(None);
    };

    match value {
        Value::String(value) => Ok(Some(value.as_str())),
        _ => Err(CodegenError::Other(format!("target option `{key}` must be a string"))),
    }
}

fn build_schema_index(file: &File) -> SchemaIndex {
    let mut index = SchemaIndex {
        package: file.package.as_ref().map(|package| path_segments(&package.value)).unwrap_or_default(),
        ..SchemaIndex::default()
    };

    for use_decl in &file.uses {
        let path = path_segments(&use_decl.path.value);
        let alias = use_decl.alias.as_ref().map(|alias| alias.value.clone());
        let import_name = alias.clone().or_else(|| path.last().cloned()).unwrap_or_default();

        index.uses.push(UseBinding { path: path.clone(), alias });
        index.imported_paths.insert(import_name, path);
    }

    for item in &file.items {
        match item {
            Item::Struct(item) => {
                index.user_types.insert(item.name.value.clone(), NamedTypeKind::Struct);
            }
            Item::Enum(item) => {
                index.user_types.insert(item.name.value.clone(), NamedTypeKind::Enum);
            }
            Item::TypeAlias(item) => {
                index.user_types.insert(item.name.value.clone(), NamedTypeKind::Alias);
                index.type_aliases.insert(item.name.value.clone(), item.ty.value.clone());
            }
            Item::Const(_) => {}
        }
    }

    index
}

fn render_rust_file(parsed_source: &ParsedSource) -> Result<String, CodegenError> {
    let index = build_schema_index(&parsed_source.file);
    let mut out = String::new();

    writeln!(&mut out, "// @generated by rocketpack-compiler").ok();
    writeln!(&mut out, "#[allow(dead_code)]").ok();
    writeln!(&mut out, "#[allow(clippy::all)]").ok();

    let mut depth = 0usize;
    for segment in &index.package {
        writeln!(&mut out, "{}pub mod {} {{", indent(depth), sanitize_ident(segment)).ok();
        depth += 1;
    }

    if !index.uses.is_empty() {
        for use_binding in &index.uses {
            let rendered_path = use_binding.path.iter().map(|segment| sanitize_ident(segment)).collect::<Vec<_>>().join("::");
            if let Some(alias) = &use_binding.alias {
                writeln!(&mut out, "{}use {} as {};", indent(depth), rendered_path, sanitize_ident(alias)).ok();
            } else {
                writeln!(&mut out, "{}use {};", indent(depth), rendered_path).ok();
            }
        }
        writeln!(&mut out).ok();
    }

    for item in &parsed_source.file.items {
        match item {
            Item::Struct(item) => {
                write_struct_declaration(&mut out, &index, item, depth);
                writeln!(&mut out).ok();
                write_struct_codec_impl(&mut out, &index, item, depth)?;
            }
            Item::Enum(item) => {
                write_enum_declaration(&mut out, &index, item, depth);
                writeln!(&mut out).ok();
                write_enum_codec_impl(&mut out, &index, item, depth)?;
            }
            Item::TypeAlias(item) => write_type_alias_declaration(&mut out, &index, item, depth),
            Item::Const(item) => write_const_declaration(&mut out, &index, item, depth)?,
        }
        writeln!(&mut out).ok();
    }

    for close_depth in (0..depth).rev() {
        writeln!(&mut out, "{}}}", indent(close_depth)).ok();
    }

    Ok(out)
}

fn write_struct_declaration(out: &mut String, index: &SchemaIndex, item: &Struct, depth: usize) {
    writeln!(out, "{}#[derive(Debug, Clone, PartialEq)]", indent(depth)).ok();
    writeln!(out, "{}pub struct {} {{", indent(depth), sanitize_ident(&item.name.value)).ok();

    for field in &item.fields {
        writeln!(
            out,
            "{}pub {}: {},",
            indent(depth + 1),
            sanitize_ident(&field.name.value),
            render_declaration_type(index, &field.ty.value)
        )
        .ok();
    }

    writeln!(out, "{}}}", indent(depth)).ok();
}

fn write_struct_codec_impl(out: &mut String, index: &SchemaIndex, item: &Struct, depth: usize) -> Result<(), CodegenError> {
    let struct_name = sanitize_ident(&item.name.value);
    let sorted_fields = resolve_sorted_struct_fields(index, item)?;

    writeln!(out, "{}impl omnius_core_rocketpack::RocketPackStruct for {} {{", indent(depth), struct_name).ok();
    write_struct_pack_fn(out, index, item, &sorted_fields, depth + 1)?;
    writeln!(out).ok();
    write_struct_unpack_fn(out, index, item, &sorted_fields, depth + 1)?;
    writeln!(out, "{}}}", indent(depth)).ok();

    Ok(())
}

fn resolve_sorted_struct_fields<'a>(index: &SchemaIndex, item: &'a Struct) -> Result<Vec<(&'a Field, ResolvedType)>, CodegenError> {
    let mut sorted_fields = Vec::with_capacity(item.fields.len());

    for field in &item.fields {
        sorted_fields.push((field, resolve_type(index, &field.ty.value)?));
    }

    sorted_fields.sort_by_key(|(field, _)| field.tag.value);
    Ok(sorted_fields)
}

fn write_struct_pack_fn(out: &mut String, index: &SchemaIndex, _item: &Struct, fields: &[(&Field, ResolvedType)], depth: usize) -> Result<(), CodegenError> {
    writeln!(out, "{}fn pack(", indent(depth)).ok();
    writeln!(out, "{}encoder: &mut impl omnius_core_rocketpack::RocketPackEncoder,", indent(depth + 1)).ok();
    writeln!(out, "{}value: &Self,", indent(depth + 1)).ok();
    writeln!(out, "{}) -> std::result::Result<(), omnius_core_rocketpack::RocketPackEncoderError> {{", indent(depth)).ok();

    let required_count = fields.iter().filter(|(_, resolved)| !matches!(resolved, ResolvedType::Option(_))).count();
    let has_optional = fields.iter().any(|(_, resolved)| matches!(resolved, ResolvedType::Option(_)));

    if has_optional {
        writeln!(out, "{}let mut count = {};", indent(depth + 1), required_count).ok();
        for (field, resolved) in fields {
            if matches!(resolved, ResolvedType::Option(_)) {
                writeln!(out, "{}if value.{}.is_some() {{", indent(depth + 1), sanitize_ident(&field.name.value)).ok();
                writeln!(out, "{}count += 1;", indent(depth + 2)).ok();
                writeln!(out, "{}}}", indent(depth + 1)).ok();
            }
        }
        writeln!(out, "{}encoder.write_map(count)?;", indent(depth + 1)).ok();
    } else {
        writeln!(out, "{}encoder.write_map({})?;", indent(depth + 1), required_count).ok();
    }

    for (field, resolved) in fields {
        let field_ident = sanitize_ident(&field.name.value);
        match resolved {
            ResolvedType::Option(inner) => {
                writeln!(out, "{}if let Some({}) = &value.{} {{", indent(depth + 1), field_ident, field_ident).ok();
                writeln!(out, "{}encoder.write_u64({})?;", indent(depth + 2), field.tag.value).ok();
                write_encode_value(out, inner, &field_ident, depth + 2)?;
                writeln!(out, "{}}}", indent(depth + 1)).ok();
            }
            _ => {
                writeln!(out, "{}encoder.write_u64({})?;", indent(depth + 1), field.tag.value).ok();
                write_encode_value(out, resolved, &format!("&value.{field_ident}"), depth + 1)?;
            }
        }
    }

    writeln!(out, "{}Ok(())", indent(depth + 1)).ok();
    writeln!(out, "{}}}", indent(depth)).ok();

    let _ = index;
    Ok(())
}

fn write_encode_value(out: &mut String, resolved: &ResolvedType, expr: &str, depth: usize) -> Result<(), CodegenError> {
    match resolved {
        ResolvedType::Builtin(builtin) => match builtin {
            BuiltinType::Bool => {
                writeln!(out, "{}encoder.write_bool(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::U8 => {
                writeln!(out, "{}encoder.write_u8(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::U16 => {
                writeln!(out, "{}encoder.write_u16(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::U32 => {
                writeln!(out, "{}encoder.write_u32(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::U64 => {
                writeln!(out, "{}encoder.write_u64(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::I8 => {
                writeln!(out, "{}encoder.write_i8(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::I16 => {
                writeln!(out, "{}encoder.write_i16(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::I32 => {
                writeln!(out, "{}encoder.write_i32(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::I64 => {
                writeln!(out, "{}encoder.write_i64(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::F32 => {
                writeln!(out, "{}encoder.write_f32(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::F64 => {
                writeln!(out, "{}encoder.write_f64(*({}))?;", indent(depth), expr).ok();
            }
            BuiltinType::String => {
                writeln!(out, "{}encoder.write_string(({}).as_str())?;", indent(depth), expr).ok();
            }
            BuiltinType::Bytes => {
                writeln!(out, "{}encoder.write_bytes(({}).as_slice())?;", indent(depth), expr).ok();
            }
            BuiltinType::U128 => {
                writeln!(
                    out,
                    "{}return Err(omnius_core_rocketpack::RocketPackEncoderError::IoError(std::io::Error::new(std::io::ErrorKind::Unsupported, \"u128 encode is not supported\")));",
                    indent(depth)
                )
                .ok();
            }
            BuiltinType::I128 => {
                writeln!(
                    out,
                    "{}return Err(omnius_core_rocketpack::RocketPackEncoderError::IoError(std::io::Error::new(std::io::ErrorKind::Unsupported, \"i128 encode is not supported\")));",
                    indent(depth)
                )
                .ok();
            }
        },
        ResolvedType::Named(_) => {
            writeln!(out, "{}encoder.write_struct({})?;", indent(depth), expr).ok();
        }
        ResolvedType::Option(inner) => {
            write_encode_value(out, inner, expr, depth)?;
        }
        ResolvedType::Vec(inner) => {
            writeln!(out, "{}encoder.write_array(({}).len())?;", indent(depth), expr).ok();
            writeln!(out, "{}for item in ({}).iter() {{", indent(depth), expr).ok();
            write_encode_value(out, inner, "item", depth + 1)?;
            writeln!(out, "{}}}", indent(depth)).ok();
        }
        ResolvedType::Map(key, value) => {
            writeln!(out, "{}encoder.write_map(({}).len())?;", indent(depth), expr).ok();
            writeln!(out, "{}for (key, value) in ({}).iter() {{", indent(depth), expr).ok();
            write_encode_value(out, key, "key", depth + 1)?;
            write_encode_value(out, value, "value", depth + 1)?;
            writeln!(out, "{}}}", indent(depth)).ok();
        }
        ResolvedType::Array(inner, _) => {
            writeln!(out, "{}encoder.write_array(({}).len())?;", indent(depth), expr).ok();
            writeln!(out, "{}for item in ({}).iter() {{", indent(depth), expr).ok();
            write_encode_value(out, inner, "item", depth + 1)?;
            writeln!(out, "{}}}", indent(depth)).ok();
        }
    }

    Ok(())
}

fn write_struct_unpack_fn(out: &mut String, index: &SchemaIndex, item: &Struct, fields: &[(&Field, ResolvedType)], depth: usize) -> Result<(), CodegenError> {
    writeln!(out, "{}fn unpack(", indent(depth)).ok();
    writeln!(out, "{}decoder: &mut impl omnius_core_rocketpack::RocketPackDecoder,", indent(depth + 1)).ok();
    writeln!(out, "{}) -> std::result::Result<Self, omnius_core_rocketpack::RocketPackDecoderError>", indent(depth)).ok();
    writeln!(out, "{}where", indent(depth)).ok();
    writeln!(out, "{}Self: Sized,", indent(depth + 1)).ok();
    writeln!(out, "{}{{", indent(depth)).ok();

    for (field, resolved) in fields {
        writeln!(
            out,
            "{}let mut {}: Option<{}> = None;",
            indent(depth + 1),
            sanitize_ident(&field.name.value),
            render_field_storage_type(index, field, resolved)
        )
        .ok();
    }

    writeln!(out, "{}let count = decoder.read_map()?;", indent(depth + 1)).ok();
    writeln!(out).ok();
    writeln!(out, "{}for _ in 0..count {{", indent(depth + 1)).ok();
    writeln!(out, "{}match decoder.read_u64()? {{", indent(depth + 2)).ok();

    let mut temp_counter = 0usize;
    for (field, resolved) in fields {
        let field_ident = sanitize_ident(&field.name.value);
        let decode_target = match resolved {
            ResolvedType::Option(inner) => inner,
            _ => resolved,
        };

        writeln!(out, "{}{} => {{", indent(depth + 3), field.tag.value).ok();
        let value_expr = write_decode_value(out, decode_target, "decoder", depth + 4, &field.name.value, &mut temp_counter)?;
        writeln!(out, "{}{} = Some({});", indent(depth + 4), field_ident, value_expr).ok();
        writeln!(out, "{}}}", indent(depth + 3)).ok();
    }

    writeln!(out, "{}_ => decoder.skip_field()?,", indent(depth + 3)).ok();
    writeln!(out, "{}}}", indent(depth + 2)).ok();
    writeln!(out, "{}}}", indent(depth + 1)).ok();
    writeln!(out).ok();
    writeln!(out, "{}Ok(Self {{", indent(depth + 1)).ok();

    for (field, resolved) in fields {
        writeln!(
            out,
            "{}{}: {},",
            indent(depth + 2),
            sanitize_ident(&field.name.value),
            render_struct_field_init(index, field, resolved)?
        )
        .ok();
    }

    writeln!(out, "{}}})", indent(depth + 1)).ok();
    writeln!(out, "{}}}", indent(depth)).ok();

    Ok(())
}

fn render_storage_type(index: &SchemaIndex, original_type: &Type, resolved: &ResolvedType) -> String {
    match (original_type, resolved) {
        (Type::Option(inner), _) => render_declaration_type(index, inner),
        (_, ResolvedType::Option(inner)) => render_resolved_type(inner),
        _ => render_declaration_type(index, original_type),
    }
}

fn render_field_storage_type(index: &SchemaIndex, field: &Field, resolved: &ResolvedType) -> String {
    render_storage_type(index, &field.ty.value, resolved)
}

fn render_value_init(value_ident: &str, field_name: &str, default: Option<&Literal>, resolved: &ResolvedType) -> Result<String, CodegenError> {
    if matches!(resolved, ResolvedType::Option(_)) {
        return Ok(value_ident.to_string());
    }

    if let Some(default) = default {
        return Ok(format!("{value_ident}.unwrap_or({})", render_literal(default)?));
    }

    Ok(format!(
        "{value_ident}.ok_or(omnius_core_rocketpack::RocketPackDecoderError::Other(\"missing field: {field_name}\"))?"
    ))
}

fn render_struct_field_init(_index: &SchemaIndex, field: &Field, resolved: &ResolvedType) -> Result<String, CodegenError> {
    let field_ident = sanitize_ident(&field.name.value);
    render_value_init(&field_ident, &field.name.value, field.default.as_ref().map(|default| &default.value), resolved)
}

fn write_decode_value(out: &mut String, resolved: &ResolvedType, decoder_ident: &str, depth: usize, context_name: &str, temp_counter: &mut usize) -> Result<String, CodegenError> {
    match resolved {
        ResolvedType::Builtin(builtin) => Ok(match builtin {
            BuiltinType::Bool => format!("{decoder_ident}.read_bool()?"),
            BuiltinType::U8 => format!("{decoder_ident}.read_u8()?"),
            BuiltinType::U16 => format!("{decoder_ident}.read_u16()?"),
            BuiltinType::U32 => format!("{decoder_ident}.read_u32()?"),
            BuiltinType::U64 => format!("{decoder_ident}.read_u64()?"),
            BuiltinType::I8 => format!("{decoder_ident}.read_i8()?"),
            BuiltinType::I16 => format!("{decoder_ident}.read_i16()?"),
            BuiltinType::I32 => format!("{decoder_ident}.read_i32()?"),
            BuiltinType::I64 => format!("{decoder_ident}.read_i64()?"),
            BuiltinType::F32 => format!("{decoder_ident}.read_f32()?"),
            BuiltinType::F64 => format!("{decoder_ident}.read_f64()?"),
            BuiltinType::String => format!("{decoder_ident}.read_string()?"),
            BuiltinType::Bytes => format!("{decoder_ident}.read_bytes_vec()?"),
            BuiltinType::U128 => "return Err(omnius_core_rocketpack::RocketPackDecoderError::Other(\"u128 decode is not supported\"))".to_string(),
            BuiltinType::I128 => "return Err(omnius_core_rocketpack::RocketPackDecoderError::Other(\"i128 decode is not supported\"))".to_string(),
        }),
        ResolvedType::Named(named) => Ok(format!("{decoder_ident}.read_struct::<{}>()?", render_named_type(named))),
        ResolvedType::Option(inner) => write_decode_value(out, inner, decoder_ident, depth, context_name, temp_counter),
        ResolvedType::Vec(inner) => {
            let count_name = next_temp_name(temp_counter, "count");
            let value_name = next_temp_name(temp_counter, "values");
            writeln!(out, "{}let {} = {}.read_array()?;", indent(depth), count_name, decoder_ident).ok();
            writeln!(
                out,
                "{}let mut {}: Vec<{}> = Vec::with_capacity({} as usize);",
                indent(depth),
                value_name,
                render_resolved_type(inner),
                count_name
            )
            .ok();
            writeln!(out, "{}for _ in 0..{} {{", indent(depth), count_name).ok();
            let inner_expr = write_decode_value(out, inner, decoder_ident, depth + 1, context_name, temp_counter)?;
            writeln!(out, "{}{}.push({});", indent(depth + 1), value_name, inner_expr).ok();
            writeln!(out, "{}}}", indent(depth)).ok();
            Ok(value_name)
        }
        ResolvedType::Map(key, value) => {
            let count_name = next_temp_name(temp_counter, "count");
            let map_name = next_temp_name(temp_counter, "map");
            writeln!(out, "{}let {} = {}.read_map()?;", indent(depth), count_name, decoder_ident).ok();
            writeln!(
                out,
                "{}let mut {}: std::collections::BTreeMap<{}, {}> = std::collections::BTreeMap::new();",
                indent(depth),
                map_name,
                render_resolved_type(key),
                render_resolved_type(value)
            )
            .ok();
            writeln!(out, "{}for _ in 0..{} {{", indent(depth), count_name).ok();
            let key_expr = write_decode_value(out, key, decoder_ident, depth + 1, context_name, temp_counter)?;
            let value_expr = write_decode_value(out, value, decoder_ident, depth + 1, context_name, temp_counter)?;
            writeln!(out, "{}{}.insert({}, {});", indent(depth + 1), map_name, key_expr, value_expr).ok();
            writeln!(out, "{}}}", indent(depth)).ok();
            Ok(map_name)
        }
        ResolvedType::Array(inner, len) => {
            let count_name = next_temp_name(temp_counter, "count");
            let values_name = next_temp_name(temp_counter, "values");
            let array_name = next_temp_name(temp_counter, "array");
            writeln!(out, "{}let {} = {}.read_array()?;", indent(depth), count_name, decoder_ident).ok();
            writeln!(out, "{}if {} != {} {{", indent(depth), count_name, len).ok();
            writeln!(
                out,
                "{}return Err(omnius_core_rocketpack::RocketPackDecoderError::Other(\"array length mismatch: {}\"));",
                indent(depth + 1),
                context_name
            )
            .ok();
            writeln!(out, "{}}}", indent(depth)).ok();
            writeln!(
                out,
                "{}let mut {}: Vec<{}> = Vec::with_capacity({} as usize);",
                indent(depth),
                values_name,
                render_resolved_type(inner),
                count_name
            )
            .ok();
            writeln!(out, "{}for _ in 0..{} {{", indent(depth), count_name).ok();
            let inner_expr = write_decode_value(out, inner, decoder_ident, depth + 1, context_name, temp_counter)?;
            writeln!(out, "{}{}.push({});", indent(depth + 1), values_name, inner_expr).ok();
            writeln!(out, "{}}}", indent(depth)).ok();
            writeln!(
                out,
                "{}let {}: {} = {}.try_into().map_err(|_| omnius_core_rocketpack::RocketPackDecoderError::Other(\"array length mismatch: {}\"))?;",
                indent(depth),
                array_name,
                render_resolved_type(resolved),
                values_name,
                context_name
            )
            .ok();
            Ok(array_name)
        }
    }
}

fn next_temp_name(counter: &mut usize, prefix: &str) -> String {
    let current = *counter;
    *counter += 1;
    format!("__{}_{}", prefix, current)
}

fn render_resolved_type(resolved: &ResolvedType) -> String {
    match resolved {
        ResolvedType::Builtin(builtin) => match builtin {
            BuiltinType::Bool => "bool".to_string(),
            BuiltinType::U8 => "u8".to_string(),
            BuiltinType::U16 => "u16".to_string(),
            BuiltinType::U32 => "u32".to_string(),
            BuiltinType::U64 => "u64".to_string(),
            BuiltinType::U128 => "u128".to_string(),
            BuiltinType::I8 => "i8".to_string(),
            BuiltinType::I16 => "i16".to_string(),
            BuiltinType::I32 => "i32".to_string(),
            BuiltinType::I64 => "i64".to_string(),
            BuiltinType::I128 => "i128".to_string(),
            BuiltinType::F32 => "f32".to_string(),
            BuiltinType::F64 => "f64".to_string(),
            BuiltinType::String => "String".to_string(),
            BuiltinType::Bytes => "Vec<u8>".to_string(),
        },
        ResolvedType::Named(named) => render_named_type(named),
        ResolvedType::Option(inner) => format!("Option<{}>", render_resolved_type(inner)),
        ResolvedType::Vec(inner) => format!("Vec<{}>", render_resolved_type(inner)),
        ResolvedType::Map(key, value) => format!("std::collections::BTreeMap<{}, {}>", render_resolved_type(key), render_resolved_type(value)),
        ResolvedType::Array(inner, len) => format!("[{}; {}]", render_resolved_type(inner), len),
    }
}

fn render_named_type(named: &NamedType) -> String {
    named.rust_path.join("::")
}

fn write_enum_codec_impl(out: &mut String, index: &SchemaIndex, item: &Enum, depth: usize) -> Result<(), CodegenError> {
    let enum_name = sanitize_ident(&item.name.value);

    writeln!(out, "{}impl omnius_core_rocketpack::RocketPackStruct for {} {{", indent(depth), enum_name).ok();
    write_enum_pack_fn(out, index, item, depth + 1)?;
    writeln!(out).ok();
    write_enum_unpack_fn(out, index, item, depth + 1)?;
    writeln!(out, "{}}}", indent(depth)).ok();

    Ok(())
}

fn write_enum_pack_fn(out: &mut String, index: &SchemaIndex, item: &Enum, depth: usize) -> Result<(), CodegenError> {
    writeln!(out, "{}fn pack(", indent(depth)).ok();
    writeln!(out, "{}encoder: &mut impl omnius_core_rocketpack::RocketPackEncoder,", indent(depth + 1)).ok();
    writeln!(out, "{}value: &Self,", indent(depth + 1)).ok();
    writeln!(out, "{}) -> std::result::Result<(), omnius_core_rocketpack::RocketPackEncoderError> {{", indent(depth)).ok();
    writeln!(out, "{}encoder.write_map(1)?;", indent(depth + 1)).ok();
    writeln!(out).ok();
    writeln!(out, "{}match value {{", indent(depth + 1)).ok();

    for variant in &item.variants {
        write_enum_pack_variant_arm(out, index, variant, depth + 2)?;
    }

    writeln!(out, "{}}}", indent(depth + 1)).ok();
    writeln!(out).ok();
    writeln!(out, "{}Ok(())", indent(depth + 1)).ok();
    writeln!(out, "{}}}", indent(depth)).ok();

    Ok(())
}

fn write_enum_pack_variant_arm(out: &mut String, index: &SchemaIndex, variant: &crate::parser::ast::Variant, depth: usize) -> Result<(), CodegenError> {
    let variant_name = sanitize_ident(&variant.name.value);

    match &variant.kind {
        VariantKind::Unit => {
            writeln!(out, "{}Self::{} => {{", indent(depth), variant_name).ok();
            writeln!(out, "{}encoder.write_u64({})?;", indent(depth + 1), variant.tag.value).ok();
            writeln!(out, "{}encoder.write_map(0)?;", indent(depth + 1)).ok();
            writeln!(out, "{}}}", indent(depth)).ok();
        }
        VariantKind::Tuple(fields) => {
            let bindings = fields.iter().map(|(name, _)| sanitize_ident(&name.value)).collect::<Vec<_>>().join(", ");
            writeln!(out, "{}Self::{} {{ {} }} => {{", indent(depth), variant_name, bindings).ok();
            writeln!(out, "{}encoder.write_u64({})?;", indent(depth + 1), variant.tag.value).ok();

            let resolved_fields = resolve_tuple_fields(index, fields)?;
            write_tuple_variant_inner_map_count(out, fields, &resolved_fields, depth + 1);
            write_tuple_variant_encode_body(out, fields, &resolved_fields, depth + 1)?;
            writeln!(out, "{}}}", indent(depth)).ok();
        }
        VariantKind::Record(fields) => {
            let bindings = fields.iter().map(|field| sanitize_ident(&field.name.value)).collect::<Vec<_>>().join(", ");
            writeln!(out, "{}Self::{} {{ {} }} => {{", indent(depth), variant_name, bindings).ok();
            writeln!(out, "{}encoder.write_u64({})?;", indent(depth + 1), variant.tag.value).ok();

            let resolved_fields = resolve_sorted_record_fields(index, fields)?;
            write_record_variant_inner_map_count(out, &resolved_fields, depth + 1);
            write_record_variant_encode_body(out, &resolved_fields, depth + 1)?;
            writeln!(out, "{}}}", indent(depth)).ok();
        }
    }

    Ok(())
}

fn write_tuple_variant_inner_map_count(
    out: &mut String,
    fields: &[(crate::parser::ast::Spanned<String>, crate::parser::ast::Spanned<Type>)],
    resolved_fields: &[(usize, ResolvedType)],
    depth: usize,
) {
    let required_count = resolved_fields.iter().filter(|(_, resolved)| !matches!(resolved, ResolvedType::Option(_))).count();
    let has_optional = resolved_fields.iter().any(|(_, resolved)| matches!(resolved, ResolvedType::Option(_)));

    if has_optional {
        writeln!(out, "{}let mut count = {};", indent(depth), required_count).ok();
        for ((name, _), (_, resolved)) in fields.iter().zip(resolved_fields.iter()) {
            if matches!(resolved, ResolvedType::Option(_)) {
                let field_ident = sanitize_ident(&name.value);
                writeln!(out, "{}if {}.is_some() {{", indent(depth), field_ident).ok();
                writeln!(out, "{}count += 1;", indent(depth + 1)).ok();
                writeln!(out, "{}}}", indent(depth)).ok();
            }
        }
        writeln!(out, "{}encoder.write_map(count)?;", indent(depth)).ok();
    } else {
        writeln!(out, "{}encoder.write_map({})?;", indent(depth), required_count).ok();
    }
}

fn write_record_variant_inner_map_count(out: &mut String, fields: &[(&Field, ResolvedType)], depth: usize) {
    let required_count = fields.iter().filter(|(_, resolved)| !matches!(resolved, ResolvedType::Option(_))).count();
    let has_optional = fields.iter().any(|(_, resolved)| matches!(resolved, ResolvedType::Option(_)));

    if has_optional {
        writeln!(out, "{}let mut count = {};", indent(depth), required_count).ok();
        for (field, resolved) in fields {
            if matches!(resolved, ResolvedType::Option(_)) {
                let field_ident = sanitize_ident(&field.name.value);
                writeln!(out, "{}if {}.is_some() {{", indent(depth), field_ident).ok();
                writeln!(out, "{}count += 1;", indent(depth + 1)).ok();
                writeln!(out, "{}}}", indent(depth)).ok();
            }
        }
        writeln!(out, "{}encoder.write_map(count)?;", indent(depth)).ok();
    } else {
        writeln!(out, "{}encoder.write_map({})?;", indent(depth), required_count).ok();
    }
}

fn write_tuple_variant_encode_body(
    out: &mut String,
    fields: &[(crate::parser::ast::Spanned<String>, crate::parser::ast::Spanned<Type>)],
    resolved_fields: &[(usize, ResolvedType)],
    depth: usize,
) -> Result<(), CodegenError> {
    for ((name, _), (tuple_index, resolved)) in fields.iter().zip(resolved_fields.iter()) {
        let binding = sanitize_ident(&name.value);
        match resolved {
            ResolvedType::Option(inner) => {
                writeln!(out, "{}if let Some({}) = {}.as_ref() {{", indent(depth), binding, binding).ok();
                writeln!(out, "{}encoder.write_u64({})?;", indent(depth + 1), tuple_index).ok();
                write_encode_value(out, inner.as_ref(), &binding, depth + 1)?;
                writeln!(out, "{}}}", indent(depth)).ok();
            }
            _ => {
                writeln!(out, "{}encoder.write_u64({})?;", indent(depth), tuple_index).ok();
                write_encode_value(out, resolved, &binding, depth)?;
            }
        }
    }

    Ok(())
}

fn write_record_variant_encode_body(out: &mut String, fields: &[(&Field, ResolvedType)], depth: usize) -> Result<(), CodegenError> {
    for (field, resolved) in fields {
        let field_ident = sanitize_ident(&field.name.value);
        match resolved {
            ResolvedType::Option(inner) => {
                writeln!(out, "{}if let Some({}) = {}.as_ref() {{", indent(depth), field_ident, field_ident).ok();
                writeln!(out, "{}encoder.write_u64({})?;", indent(depth + 1), field.tag.value).ok();
                write_encode_value(out, inner, &field_ident, depth + 1)?;
                writeln!(out, "{}}}", indent(depth)).ok();
            }
            _ => {
                writeln!(out, "{}encoder.write_u64({})?;", indent(depth), field.tag.value).ok();
                write_encode_value(out, resolved, &field_ident, depth)?;
            }
        }
    }

    Ok(())
}

fn write_enum_unpack_fn(out: &mut String, index: &SchemaIndex, item: &Enum, depth: usize) -> Result<(), CodegenError> {
    writeln!(out, "{}fn unpack(", indent(depth)).ok();
    writeln!(out, "{}decoder: &mut impl omnius_core_rocketpack::RocketPackDecoder,", indent(depth + 1)).ok();
    writeln!(out, "{}) -> std::result::Result<Self, omnius_core_rocketpack::RocketPackDecoderError>", indent(depth)).ok();
    writeln!(out, "{}where", indent(depth)).ok();
    writeln!(out, "{}Self: Sized,", indent(depth + 1)).ok();
    writeln!(out, "{}{{", indent(depth)).ok();
    writeln!(out, "{}let mut result: Option<Self> = None;", indent(depth + 1)).ok();
    writeln!(out, "{}let count = decoder.read_map()?;", indent(depth + 1)).ok();
    writeln!(out).ok();
    writeln!(out, "{}for _ in 0..count {{", indent(depth + 1)).ok();
    writeln!(out, "{}match decoder.read_u64()? {{", indent(depth + 2)).ok();

    let mut temp_counter = 0usize;
    for variant in &item.variants {
        write_enum_unpack_variant_arm(out, index, variant, depth + 3, &mut temp_counter)?;
    }

    writeln!(out, "{}_ => decoder.skip_field()?,", indent(depth + 3)).ok();
    writeln!(out, "{}}}", indent(depth + 2)).ok();
    writeln!(out, "{}}}", indent(depth + 1)).ok();
    writeln!(out).ok();
    writeln!(
        out,
        "{}result.ok_or(omnius_core_rocketpack::RocketPackDecoderError::Other(\"missing enum variant\"))",
        indent(depth + 1)
    )
    .ok();
    writeln!(out, "{}}}", indent(depth)).ok();

    Ok(())
}

fn write_enum_unpack_variant_arm(out: &mut String, index: &SchemaIndex, variant: &crate::parser::ast::Variant, depth: usize, temp_counter: &mut usize) -> Result<(), CodegenError> {
    let variant_name = sanitize_ident(&variant.name.value);
    writeln!(out, "{}{} => {{", indent(depth), variant.tag.value).ok();
    let inner_count = next_temp_name(temp_counter, "inner_count");
    writeln!(out, "{}let {} = decoder.read_map()?;", indent(depth + 1), inner_count).ok();

    match &variant.kind {
        VariantKind::Unit => {
            writeln!(out, "{}for _ in 0..{} {{", indent(depth + 1), inner_count).ok();
            writeln!(out, "{}let _ = decoder.read_u64()?;", indent(depth + 2)).ok();
            writeln!(out, "{}decoder.skip_field()?;", indent(depth + 2)).ok();
            writeln!(out, "{}}}", indent(depth + 1)).ok();
            writeln!(out, "{}result = Some(Self::{});", indent(depth + 1), variant_name).ok();
        }
        VariantKind::Tuple(fields) => {
            let resolved_fields = resolve_tuple_fields(index, fields)?;
            let tuple_bindings = declare_tuple_variant_storage(out, index, fields, &resolved_fields, depth + 1);
            writeln!(out, "{}for _ in 0..{} {{", indent(depth + 1), inner_count).ok();
            writeln!(out, "{}match decoder.read_u64()? {{", indent(depth + 2)).ok();

            for (((name, _), (tuple_index, resolved)), binding_name) in fields.iter().zip(resolved_fields.iter()).zip(tuple_bindings.iter()) {
                let decode_target = match resolved {
                    ResolvedType::Option(inner) => inner.as_ref(),
                    _ => resolved,
                };
                writeln!(out, "{}{} => {{", indent(depth + 3), tuple_index).ok();
                let value_expr = write_decode_value(out, decode_target, "decoder", depth + 4, &name.value, temp_counter)?;
                writeln!(out, "{}{} = Some({});", indent(depth + 4), binding_name, value_expr).ok();
                writeln!(out, "{}}}", indent(depth + 3)).ok();
            }

            writeln!(out, "{}_ => decoder.skip_field()?,", indent(depth + 3)).ok();
            writeln!(out, "{}}}", indent(depth + 2)).ok();
            writeln!(out, "{}}}", indent(depth + 1)).ok();

            let init_fields = fields
                .iter()
                .zip(resolved_fields.iter())
                .zip(tuple_bindings.iter())
                .map(|(((name, _), (_, resolved)), binding_name)| {
                    Ok(format!(
                        "{}: {}",
                        sanitize_ident(&name.value),
                        render_value_init(binding_name, &name.value, None, resolved)?
                    ))
                })
                .collect::<Result<Vec<_>, CodegenError>>()?;
            writeln!(out, "{}result = Some(Self::{} {{ {} }});", indent(depth + 1), variant_name, init_fields.join(", ")).ok();
        }
        VariantKind::Record(fields) => {
            let resolved_fields = resolve_sorted_record_fields(index, fields)?;
            declare_record_variant_storage(out, index, &resolved_fields, depth + 1);
            writeln!(out, "{}for _ in 0..{} {{", indent(depth + 1), inner_count).ok();
            writeln!(out, "{}match decoder.read_u64()? {{", indent(depth + 2)).ok();

            for (field, resolved) in &resolved_fields {
                let binding_name = sanitize_ident(&field.name.value);
                let decode_target = match resolved {
                    ResolvedType::Option(inner) => inner.as_ref(),
                    _ => resolved,
                };
                writeln!(out, "{}{} => {{", indent(depth + 3), field.tag.value).ok();
                let value_expr = write_decode_value(out, decode_target, "decoder", depth + 4, &field.name.value, temp_counter)?;
                writeln!(out, "{}{} = Some({});", indent(depth + 4), binding_name, value_expr).ok();
                writeln!(out, "{}}}", indent(depth + 3)).ok();
            }

            writeln!(out, "{}_ => decoder.skip_field()?,", indent(depth + 3)).ok();
            writeln!(out, "{}}}", indent(depth + 2)).ok();
            writeln!(out, "{}}}", indent(depth + 1)).ok();

            let init_fields = resolved_fields
                .iter()
                .map(|(field, resolved)| {
                    let binding_name = sanitize_ident(&field.name.value);
                    Ok(format!(
                        "{}: {}",
                        binding_name,
                        render_value_init(&binding_name, &field.name.value, field.default.as_ref().map(|default| &default.value), resolved)?
                    ))
                })
                .collect::<Result<Vec<_>, CodegenError>>()?;
            writeln!(out, "{}result = Some(Self::{} {{ {} }});", indent(depth + 1), variant_name, init_fields.join(", ")).ok();
        }
    }

    writeln!(out, "{}}}", indent(depth)).ok();
    Ok(())
}

fn resolve_tuple_fields(
    index: &SchemaIndex,
    fields: &[(crate::parser::ast::Spanned<String>, crate::parser::ast::Spanned<Type>)],
) -> Result<Vec<(usize, ResolvedType)>, CodegenError> {
    let mut resolved = Vec::with_capacity(fields.len());
    for (index_in_tuple, (_, ty)) in fields.iter().enumerate() {
        resolved.push((index_in_tuple, resolve_type(index, &ty.value)?));
    }
    Ok(resolved)
}

fn resolve_sorted_record_fields<'a>(index: &SchemaIndex, fields: &'a [Field]) -> Result<Vec<(&'a Field, ResolvedType)>, CodegenError> {
    let mut resolved = Vec::with_capacity(fields.len());
    for field in fields {
        resolved.push((field, resolve_type(index, &field.ty.value)?));
    }
    resolved.sort_by_key(|(field, _)| field.tag.value);
    Ok(resolved)
}

fn declare_tuple_variant_storage(
    out: &mut String,
    index: &SchemaIndex,
    fields: &[(crate::parser::ast::Spanned<String>, crate::parser::ast::Spanned<Type>)],
    resolved_fields: &[(usize, ResolvedType)],
    depth: usize,
) -> Vec<String> {
    let mut bindings = Vec::with_capacity(fields.len());
    for (((name, ty), (_, resolved))) in fields.iter().zip(resolved_fields.iter()) {
        let binding_name = sanitize_ident(&name.value);
        writeln!(
            out,
            "{}let mut {}: Option<{}> = None;",
            indent(depth),
            binding_name,
            render_storage_type(index, &ty.value, resolved)
        )
        .ok();
        bindings.push(binding_name);
    }
    bindings
}

fn declare_record_variant_storage(out: &mut String, index: &SchemaIndex, fields: &[(&Field, ResolvedType)], depth: usize) {
    for (field, resolved) in fields {
        writeln!(
            out,
            "{}let mut {}: Option<{}> = None;",
            indent(depth),
            sanitize_ident(&field.name.value),
            render_storage_type(index, &field.ty.value, resolved)
        )
        .ok();
    }
}

fn write_enum_declaration(out: &mut String, index: &SchemaIndex, item: &Enum, depth: usize) {
    writeln!(out, "{}#[derive(Debug, Clone, PartialEq)]", indent(depth)).ok();
    writeln!(out, "{}pub enum {} {{", indent(depth), sanitize_ident(&item.name.value)).ok();

    for variant in &item.variants {
        match &variant.kind {
            VariantKind::Unit => {
                writeln!(out, "{}{},", indent(depth + 1), sanitize_ident(&variant.name.value)).ok();
            }
            VariantKind::Tuple(fields) => {
                writeln!(out, "{}{} {{", indent(depth + 1), sanitize_ident(&variant.name.value)).ok();
                for (name, ty) in fields {
                    writeln!(out, "{}{}: {},", indent(depth + 2), sanitize_ident(&name.value), render_declaration_type(index, &ty.value)).ok();
                }
                writeln!(out, "{}}},", indent(depth + 1)).ok();
            }
            VariantKind::Record(fields) => {
                writeln!(out, "{}{} {{", indent(depth + 1), sanitize_ident(&variant.name.value)).ok();
                for field in fields {
                    writeln!(
                        out,
                        "{}{}: {},",
                        indent(depth + 2),
                        sanitize_ident(&field.name.value),
                        render_declaration_type(index, &field.ty.value)
                    )
                    .ok();
                }
                writeln!(out, "{}}},", indent(depth + 1)).ok();
            }
        }
    }

    writeln!(out, "{}}}", indent(depth)).ok();
}

fn write_type_alias_declaration(out: &mut String, index: &SchemaIndex, item: &crate::parser::ast::TypeAlias, depth: usize) {
    writeln!(
        out,
        "{}pub type {} = {};",
        indent(depth),
        sanitize_ident(&item.name.value),
        render_declaration_type(index, &item.ty.value)
    )
    .ok();
}

fn write_const_declaration(out: &mut String, index: &SchemaIndex, item: &Const, depth: usize) -> Result<(), CodegenError> {
    writeln!(
        out,
        "{}pub const {}: {} = {};",
        indent(depth),
        sanitize_ident(&item.name.value),
        render_declaration_type(index, &item.ty.value),
        render_literal(&item.value.value)?
    )
    .ok();

    Ok(())
}

fn render_declaration_type(index: &SchemaIndex, ty: &Type) -> String {
    match ty {
        Type::Path(path) => render_path_type(index, path),
        Type::Option(inner) => format!("Option<{}>", render_declaration_type(index, inner)),
        Type::Vec(inner) => format!("Vec<{}>", render_declaration_type(index, inner)),
        Type::Map(key, value) => format!(
            "std::collections::BTreeMap<{}, {}>",
            render_declaration_type(index, key),
            render_declaration_type(index, value)
        ),
        Type::Array(inner, len) => format!("[{}; {}]", render_declaration_type(index, inner), len),
    }
}

fn render_path_type(_index: &SchemaIndex, path: &AstPath) -> String {
    if let Some(builtin) = builtin_type(path) {
        return match builtin {
            BuiltinType::Bool => "bool".to_string(),
            BuiltinType::U8 => "u8".to_string(),
            BuiltinType::U16 => "u16".to_string(),
            BuiltinType::U32 => "u32".to_string(),
            BuiltinType::U64 => "u64".to_string(),
            BuiltinType::U128 => "u128".to_string(),
            BuiltinType::I8 => "i8".to_string(),
            BuiltinType::I16 => "i16".to_string(),
            BuiltinType::I32 => "i32".to_string(),
            BuiltinType::I64 => "i64".to_string(),
            BuiltinType::I128 => "i128".to_string(),
            BuiltinType::F32 => "f32".to_string(),
            BuiltinType::F64 => "f64".to_string(),
            BuiltinType::String => "String".to_string(),
            BuiltinType::Bytes => "Vec<u8>".to_string(),
        };
    }

    path_segments(path).iter().map(|segment| sanitize_ident(segment)).collect::<Vec<_>>().join("::")
}

fn resolve_type(index: &SchemaIndex, ty: &Type) -> Result<ResolvedType, CodegenError> {
    let mut resolving_aliases = Vec::<String>::new();
    resolve_type_inner(index, ty, &mut resolving_aliases)
}

fn resolve_type_inner(index: &SchemaIndex, ty: &Type, resolving_aliases: &mut Vec<String>) -> Result<ResolvedType, CodegenError> {
    match ty {
        Type::Path(path) => resolve_path_type(index, path, resolving_aliases),
        Type::Option(inner) => Ok(ResolvedType::Option(Box::new(resolve_type_inner(index, inner, resolving_aliases)?))),
        Type::Vec(inner) => Ok(ResolvedType::Vec(Box::new(resolve_type_inner(index, inner, resolving_aliases)?))),
        Type::Map(key, value) => Ok(ResolvedType::Map(
            Box::new(resolve_type_inner(index, key, resolving_aliases)?),
            Box::new(resolve_type_inner(index, value, resolving_aliases)?),
        )),
        Type::Array(inner, len) => Ok(ResolvedType::Array(Box::new(resolve_type_inner(index, inner, resolving_aliases)?), *len)),
    }
}

fn resolve_path_type(index: &SchemaIndex, path: &AstPath, resolving_aliases: &mut Vec<String>) -> Result<ResolvedType, CodegenError> {
    if let Some(builtin) = builtin_type(path) {
        return Ok(ResolvedType::Builtin(builtin));
    }

    let segments = path_segments(path);
    if segments.len() == 1 {
        let name = &segments[0];

        if let Some(alias_ty) = index.type_aliases.get(name) {
            if resolving_aliases.iter().any(|current| current == name) {
                return Err(CodegenError::Other(format!("cyclic type alias: {name}")));
            }

            resolving_aliases.push(name.clone());
            let resolved = resolve_type_inner(index, alias_ty, resolving_aliases)?;
            resolving_aliases.pop();
            return Ok(resolved);
        }

        if let Some(kind) = index.user_types.get(name) {
            return Ok(ResolvedType::Named(NamedType {
                rust_path: vec![sanitize_ident(name)],
                kind: kind.clone(),
            }));
        }

        if let Some(imported_path) = index.imported_paths.get(name) {
            return Ok(ResolvedType::Named(NamedType {
                rust_path: imported_path.iter().map(|segment| sanitize_ident(segment)).collect(),
                kind: NamedTypeKind::External,
            }));
        }
    }

    Ok(ResolvedType::Named(NamedType {
        rust_path: segments.iter().map(|segment| sanitize_ident(segment)).collect(),
        kind: NamedTypeKind::External,
    }))
}

fn render_literal(literal: &Literal) -> Result<String, CodegenError> {
    Ok(match literal {
        Literal::Bool(value) => value.to_string(),
        Literal::Int(value) => value.to_string(),
        Literal::Float(value) => {
            let mut rendered = value.to_string();
            if !rendered.contains('.') && !rendered.contains('e') && !rendered.contains('E') {
                rendered.push_str(".0");
            }
            rendered
        }
        Literal::String(value) => format!("{value:?}"),
        Literal::Bytes(bytes) => {
            let rendered = bytes.iter().map(|byte| byte.to_string()).collect::<Vec<_>>().join(", ");
            format!("vec![{rendered}]")
        }
    })
}

fn discover_source_files(root_dir: &FsPath, sources: &[SourceConfig]) -> Result<Vec<DiscoveredSource>, CodegenError> {
    let mut discovered = BTreeMap::<PathBuf, DiscoveredSource>::new();

    for source in sources {
        let base_dir = root_dir.join(&source.base_dir);
        if !base_dir.exists() {
            return Err(CodegenError::Other(format!("source base_dir not found: {}", base_dir.display())));
        }

        if !base_dir.is_dir() {
            return Err(CodegenError::Other(format!("source base_dir is not a directory: {}", base_dir.display())));
        }

        let mut relative_paths = Vec::new();
        collect_relative_files(&base_dir, &base_dir, &mut relative_paths)?;

        for relative_path in relative_paths {
            let normalized = normalize_path(&relative_path);
            let included = source.includes.is_empty() || source.includes.iter().any(|pattern| glob_matches(pattern, &normalized));
            let excluded = source.excludes.iter().any(|pattern| glob_matches(pattern, &normalized));

            if !included || excluded {
                continue;
            }

            let absolute_path = base_dir.join(&relative_path);
            discovered.entry(absolute_path.clone()).or_insert_with(|| DiscoveredSource {
                base_dir: base_dir.clone(),
                absolute_path,
                relative_path,
            });
        }
    }

    Ok(discovered.into_values().collect())
}

fn collect_relative_files(base_dir: &FsPath, current_dir: &FsPath, out: &mut Vec<PathBuf>) -> Result<(), CodegenError> {
    for entry in fs::read_dir(current_dir).map_err(|err| CodegenError::Other(format!("failed to read directory: {}: {}", current_dir.display(), err)))? {
        let entry = entry.map_err(|err| CodegenError::Other(format!("failed to read directory entry: {}: {}", current_dir.display(), err)))?;

        let path = entry.path();
        if path.is_dir() {
            collect_relative_files(base_dir, &path, out)?;
        } else if path.is_file() {
            let relative_path = path
                .strip_prefix(base_dir)
                .map_err(|err| CodegenError::Other(format!("failed to build relative path: {}: {}", path.display(), err)))?
                .to_path_buf();
            out.push(relative_path);
        }
    }

    Ok(())
}

fn normalize_path(path: &FsPath) -> String {
    path.components().map(|component| component.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

fn path_segments(path: &AstPath) -> Vec<String> {
    path.segments.iter().map(|segment| segment.value.clone()).collect()
}

fn builtin_type(path: &AstPath) -> Option<BuiltinType> {
    let segments = path_segments(path);
    if segments.len() != 1 {
        return None;
    }

    match segments[0].as_str() {
        "bool" => Some(BuiltinType::Bool),
        "u8" => Some(BuiltinType::U8),
        "u16" => Some(BuiltinType::U16),
        "u32" => Some(BuiltinType::U32),
        "u64" => Some(BuiltinType::U64),
        "u128" => Some(BuiltinType::U128),
        "i8" => Some(BuiltinType::I8),
        "i16" => Some(BuiltinType::I16),
        "i32" => Some(BuiltinType::I32),
        "i64" => Some(BuiltinType::I64),
        "i128" => Some(BuiltinType::I128),
        "f32" => Some(BuiltinType::F32),
        "f64" => Some(BuiltinType::F64),
        "string" => Some(BuiltinType::String),
        "bytes" => Some(BuiltinType::Bytes),
        _ => None,
    }
}

fn glob_matches(pattern: &str, candidate: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let candidate_chars: Vec<char> = candidate.chars().collect();
    let mut memo = BTreeMap::<(usize, usize), bool>::new();
    glob_matches_inner(&pattern_chars, &candidate_chars, 0, 0, &mut memo)
}

fn glob_matches_inner(pattern: &[char], candidate: &[char], pattern_index: usize, candidate_index: usize, memo: &mut BTreeMap<(usize, usize), bool>) -> bool {
    if let Some(value) = memo.get(&(pattern_index, candidate_index)) {
        return *value;
    }

    let result = if pattern_index == pattern.len() {
        candidate_index == candidate.len()
    } else if pattern[pattern_index] == '*' {
        let mut next_index = pattern_index;
        while next_index < pattern.len() && pattern[next_index] == '*' {
            next_index += 1;
        }

        let is_double_star = next_index - pattern_index >= 2;
        if is_double_star {
            let mut matched = glob_matches_inner(pattern, candidate, next_index, candidate_index, memo);

            if !matched && next_index < pattern.len() && pattern[next_index] == '/' {
                matched = glob_matches_inner(pattern, candidate, next_index + 1, candidate_index, memo);
            }

            if !matched && candidate_index < candidate.len() {
                matched = glob_matches_inner(pattern, candidate, pattern_index, candidate_index + 1, memo);
            }

            matched
        } else {
            glob_matches_inner(pattern, candidate, pattern_index + 1, candidate_index, memo)
                || (candidate_index < candidate.len() && candidate[candidate_index] != '/' && glob_matches_inner(pattern, candidate, pattern_index, candidate_index + 1, memo))
        }
    } else if pattern[pattern_index] == '?' {
        candidate_index < candidate.len() && candidate[candidate_index] != '/' && glob_matches_inner(pattern, candidate, pattern_index + 1, candidate_index + 1, memo)
    } else {
        candidate_index < candidate.len()
            && pattern[pattern_index] == candidate[candidate_index]
            && glob_matches_inner(pattern, candidate, pattern_index + 1, candidate_index + 1, memo)
    };

    memo.insert((pattern_index, candidate_index), result);
    result
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn sanitize_ident(value: &str) -> String {
    match value {
        "type" | "const" | "struct" | "enum" | "fn" | "mod" | "use" | "crate" | "super" | "self" | "match" | "loop" | "for" | "while" | "in" | "where" | "impl" | "trait"
        | "move" | "async" | "await" | "ref" | "mut" | "pub" | "let" | "break" | "continue" | "return" => format!("{value}_"),
        _ => value.to_string(),
    }
}
