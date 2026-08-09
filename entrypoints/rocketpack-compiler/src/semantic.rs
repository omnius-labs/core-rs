use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::{LoadedManifest, ManifestGraph, SourceConfig},
    error::CodegenError,
    parser::{
        self,
        ast::{File, Item, LengthBound, Literal, Path as AstPath, Span, Type, VariantKind},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Struct,
    Enum,
    Alias,
    Const,
}

#[derive(Debug, Clone)]
pub struct SchemaSymbol {
    pub key: String,
    pub path: Vec<String>,
    pub owner: String,
    pub file_index: usize,
    pub item_index: usize,
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SchemaFile {
    pub owner: String,
    pub absolute_path: PathBuf,
    pub relative_path: PathBuf,
    pub package: Vec<String>,
    pub text: String,
    pub ast: File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinType {
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
    Timestamp64,
    Timestamp96,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthConstraint {
    pub min: u64,
    pub max: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedType {
    Builtin(BuiltinType),
    Named(String),
    Option(Box<ResolvedType>),
    Vec(Box<ResolvedType>),
    Map(Box<ResolvedType>, Box<ResolvedType>),
    Array(Box<ResolvedType>, u64),
    Constrained(Box<ResolvedType>, LengthConstraint),
}

#[derive(Debug, Clone)]
pub struct SemanticGraph {
    pub manifests: ManifestGraph,
    pub files: Vec<SchemaFile>,
    pub symbols: BTreeMap<String, SchemaSymbol>,
    local_symbols: Vec<BTreeMap<String, String>>,
    bindings: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiscoveredSource {
    owner: String,
    absolute_path: PathBuf,
    relative_path: PathBuf,
}

impl SemanticGraph {
    pub fn build(manifests: ManifestGraph) -> Result<Self, CodegenError> {
        let discovered = discover_all_sources(&manifests)?;
        let mut files = Vec::with_capacity(discovered.len());

        for source in discovered {
            let text = fs::read_to_string(&source.absolute_path)?;
            let ast = parser::parse_source(&source.absolute_path, &text)?;
            let package = ast
                .package
                .as_ref()
                .map(|package| path_segments(&package.value))
                .filter(|segments| !segments.is_empty())
                .ok_or_else(|| semantic_error(&source.absolute_path, "missing required package declaration"))?;
            files.push(SchemaFile {
                owner: source.owner,
                absolute_path: source.absolute_path,
                relative_path: source.relative_path,
                package,
                text,
                ast,
            });
        }

        let mut graph = Self {
            manifests,
            local_symbols: vec![BTreeMap::new(); files.len()],
            bindings: vec![BTreeMap::new(); files.len()],
            files,
            symbols: BTreeMap::new(),
        };
        graph.register_symbols()?;
        graph.register_bindings()?;
        graph.validate_references()?;
        Ok(graph)
    }

    pub fn root_name(&self) -> &str {
        &self.manifests.root_name
    }

    pub fn root_manifest(&self) -> &LoadedManifest {
        self.manifests.root()
    }

    pub fn root_file_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.files
            .iter()
            .enumerate()
            .filter(|(_, file)| file.owner == self.manifests.root_name)
            .map(|(index, _)| index)
    }

    pub fn symbol(&self, key: &str) -> &SchemaSymbol {
        self.symbols.get(key).expect("resolved symbol key must exist")
    }

    pub fn resolve_symbol(&self, file_index: usize, path: &AstPath) -> Result<&SchemaSymbol, CodegenError> {
        let key = self.resolve_symbol_key(file_index, path)?;
        Ok(self.symbol(&key))
    }

    pub fn resolve_type(&self, file_index: usize, ty: &Type) -> Result<ResolvedType, CodegenError> {
        self.resolve_type_inner(file_index, ty, &mut Vec::new())
    }

    fn register_symbols(&mut self) -> Result<(), CodegenError> {
        for (file_index, file) in self.files.iter().enumerate() {
            for (item_index, item) in file.ast.items.iter().enumerate() {
                let (name, kind, span) = match item {
                    Item::Struct(item) => (&item.name.value, SymbolKind::Struct, item.name.span.clone()),
                    Item::Enum(item) => (&item.name.value, SymbolKind::Enum, item.name.span.clone()),
                    Item::TypeAlias(item) => (&item.name.value, SymbolKind::Alias, item.name.span.clone()),
                    Item::Const(item) => (&item.name.value, SymbolKind::Const, item.name.span.clone()),
                };
                if kind != SymbolKind::Const && is_reserved_type_name(name) {
                    return Err(semantic_error(&file.absolute_path, "reserved built-in type name cannot be declared"));
                }
                let mut path = file.package.clone();
                path.push(name.clone());
                let key = path.join("::");

                if let Some(previous) = self.symbols.get(&key) {
                    let previous_file = &self.files[previous.file_index];
                    return Err(semantic_error(
                        &file.absolute_path,
                        &format!(
                            "duplicate schema symbol `{key}` at bytes {}..{} (module `{}`); first defined at {}:{}..{} (module `{}`)",
                            span.start,
                            span.end,
                            file.owner,
                            previous_file.absolute_path.display(),
                            previous.span.start,
                            previous.span.end,
                            previous.owner,
                        ),
                    ));
                }

                if let Some(previous) = self.local_symbols[file_index].insert(name.clone(), key.clone()) {
                    return Err(semantic_error(&file.absolute_path, &format!("duplicate local symbol `{name}`: `{previous}` and `{key}`")));
                }

                self.symbols.insert(
                    key.clone(),
                    SchemaSymbol {
                        key,
                        path,
                        owner: file.owner.clone(),
                        file_index,
                        item_index,
                        kind,
                        span,
                    },
                );
            }
        }
        Ok(())
    }

    fn register_bindings(&mut self) -> Result<(), CodegenError> {
        for file_index in 0..self.files.len() {
            let file = &self.files[file_index];
            for use_decl in &file.ast.uses {
                let path = path_segments(&use_decl.path.value);
                let binding_name = use_decl
                    .alias
                    .as_ref()
                    .map(|alias| alias.value.clone())
                    .or_else(|| path.last().cloned())
                    .ok_or_else(|| semantic_error(&file.absolute_path, "empty use path"))?;
                if is_reserved_type_name(&binding_name) {
                    return Err(semantic_error(&file.absolute_path, "reserved built-in type name cannot be imported"));
                }
                let key = path.join("::");
                let symbol = self
                    .symbols
                    .get(&key)
                    .ok_or_else(|| semantic_error(&file.absolute_path, &format!("unknown schema symbol in use declaration: `{key}`")))?;
                self.ensure_visible(file_index, symbol)?;
                ensure_type_symbol(file, symbol)?;

                if let Some(local_key) = self.local_symbols[file_index].get(&binding_name) {
                    return Err(semantic_error(
                        &file.absolute_path,
                        &format!("use binding `{binding_name}` conflicts with local symbol `{local_key}`"),
                    ));
                }

                if let Some(previous) = self.bindings[file_index].insert(binding_name.clone(), key.clone()) {
                    return Err(semantic_error(
                        &file.absolute_path,
                        &format!("duplicate use binding `{binding_name}` for `{previous}` and `{key}`"),
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_references(&self) -> Result<(), CodegenError> {
        for (file_index, file) in self.files.iter().enumerate() {
            for item in &file.ast.items {
                match item {
                    Item::Struct(item) => {
                        validate_unique_tags(&file.absolute_path, &item.name.value, "field", item.fields.iter().map(|field| field.tag.value))?;
                        for field in &item.fields {
                            let resolved = self.resolve_type(file_index, &field.ty.value)?;
                            validate_default(
                                &file.absolute_path,
                                &item.name.value,
                                &field.name.value,
                                &resolved,
                                field.default.as_ref().map(|value| &value.value),
                            )?;
                        }
                    }
                    Item::Enum(item) => {
                        validate_unique_tags(&file.absolute_path, &item.name.value, "variant", item.variants.iter().map(|variant| variant.tag.value))?;
                        for variant in &item.variants {
                            match &variant.kind {
                                VariantKind::Unit => {}
                                VariantKind::Tuple(fields) => {
                                    for (_, ty) in fields {
                                        self.resolve_type(file_index, &ty.value)?;
                                    }
                                }
                                VariantKind::Record(fields) => {
                                    validate_unique_tags(
                                        &file.absolute_path,
                                        &format!("{}.{}", item.name.value, variant.name.value),
                                        "field",
                                        fields.iter().map(|field| field.tag.value),
                                    )?;
                                    for field in fields {
                                        let resolved = self.resolve_type(file_index, &field.ty.value)?;
                                        validate_default(
                                            &file.absolute_path,
                                            &format!("{}.{}", item.name.value, variant.name.value),
                                            &field.name.value,
                                            &resolved,
                                            field.default.as_ref().map(|value| &value.value),
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                    Item::TypeAlias(item) => {
                        self.resolve_type(file_index, &item.ty.value)?;
                    }
                    Item::Const(item) => {
                        self.resolve_type(file_index, &item.ty.value)?;
                        if let (Some(maximum), Literal::Int(value)) = (unsigned_integer_max(&item.ty.value), &item.value.value)
                            && *value > maximum
                        {
                            return Err(semantic_error(
                                &file.absolute_path,
                                &format!("constant value {value} exceeds the declared unsigned integer range"),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve_type_inner(&self, file_index: usize, ty: &Type, aliases: &mut Vec<String>) -> Result<ResolvedType, CodegenError> {
        match ty {
            Type::Path(path) => {
                if let Some(builtin) = builtin_type(path) {
                    return Ok(ResolvedType::Builtin(builtin));
                }

                let key = self.resolve_symbol_key(file_index, path)?;
                let symbol = self.symbol(&key);
                match symbol.kind {
                    SymbolKind::Struct | SymbolKind::Enum => Ok(ResolvedType::Named(key)),
                    SymbolKind::Alias => {
                        if aliases.iter().any(|alias| alias == &key) {
                            aliases.push(key.clone());
                            return Err(semantic_error(
                                &self.files[file_index].absolute_path,
                                &format!("cyclic type alias: {}", aliases.join(" -> ")),
                            ));
                        }
                        aliases.push(key);
                        let alias_type = match &self.files[symbol.file_index].ast.items[symbol.item_index] {
                            Item::TypeAlias(alias) => &alias.ty.value,
                            _ => unreachable!("alias symbol must reference a type alias item"),
                        };
                        let resolved = self.resolve_type_inner(symbol.file_index, alias_type, aliases)?;
                        aliases.pop();
                        Ok(resolved)
                    }
                    SymbolKind::Const => Err(semantic_error(&self.files[file_index].absolute_path, &format!("constant `{key}` cannot be used as a type"))),
                }
            }
            Type::Option(inner) => Ok(ResolvedType::Option(Box::new(self.resolve_type_inner(file_index, inner, aliases)?))),
            Type::Vec(inner) => Ok(ResolvedType::Vec(Box::new(self.resolve_type_inner(file_index, inner, aliases)?))),
            Type::Map(key, value) => Ok(ResolvedType::Map(
                Box::new(self.resolve_type_inner(file_index, key, aliases)?),
                Box::new(self.resolve_type_inner(file_index, value, aliases)?),
            )),
            Type::Array(inner, len) => Ok(ResolvedType::Array(Box::new(self.resolve_type_inner(file_index, inner, aliases)?), *len)),
            Type::Constrained(inner, range) => {
                if !is_direct_variable_length_type(inner) {
                    return Err(semantic_error(
                        &self.files[file_index].absolute_path,
                        "only string, bytes, Vec, and Map occurrences may carry a range; a range cannot be attached to a type alias",
                    ));
                }
                let min = range
                    .min
                    .as_ref()
                    .map(|bound| self.resolve_length_bound(file_index, &bound.value))
                    .transpose()?
                    .unwrap_or(0);
                let max = self.resolve_length_bound(file_index, &range.max.value)?;
                if min > max {
                    return Err(semantic_error(&self.files[file_index].absolute_path, &format!("length range is reversed: {min}..={max}")));
                }
                Ok(ResolvedType::Constrained(
                    Box::new(self.resolve_type_inner(file_index, inner, aliases)?),
                    LengthConstraint { min, max },
                ))
            }
        }
    }

    fn resolve_length_bound(&self, file_index: usize, bound: &LengthBound) -> Result<u64, CodegenError> {
        match bound {
            LengthBound::Literal(value) => u64::try_from(*value).map_err(|_| semantic_error(&self.files[file_index].absolute_path, "length bound exceeds u64")),
            LengthBound::Const(name) => {
                let file = &self.files[file_index];
                let mut path = file.package.clone();
                path.push(name.clone());
                let key = path.join("::");
                let symbol = self
                    .symbols
                    .get(&key)
                    .ok_or_else(|| semantic_error(&file.absolute_path, &format!("length bound constant `{name}` is unresolved")))?;
                self.ensure_visible(file_index, symbol)?;
                let Item::Const(item) = &self.files[symbol.file_index].ast.items[symbol.item_index] else {
                    return Err(semantic_error(&file.absolute_path, &format!("length bound `{name}` does not reference a constant")));
                };
                let maximum = unsigned_integer_max(&item.ty.value)
                    .ok_or_else(|| semantic_error(&file.absolute_path, &format!("length bound constant `{name}` must use u8, u16, u32, or u64")))?;
                let Literal::Int(value) = item.value.value else {
                    return Err(semantic_error(&file.absolute_path, &format!("length bound constant `{name}` must be an integer literal")));
                };
                if value > maximum {
                    return Err(semantic_error(
                        &file.absolute_path,
                        &format!("constant value {value} exceeds the declared unsigned integer range"),
                    ));
                }
                u64::try_from(value).map_err(|_| semantic_error(&file.absolute_path, &format!("length bound constant `{name}` exceeds u64")))
            }
        }
    }

    fn resolve_symbol_key(&self, file_index: usize, path: &AstPath) -> Result<String, CodegenError> {
        let segments = path_segments(path);
        let file = &self.files[file_index];
        let key = if segments.len() == 1 {
            let name = &segments[0];
            self.local_symbols[file_index]
                .get(name)
                .or_else(|| self.bindings[file_index].get(name))
                .cloned()
                .ok_or_else(|| {
                    semantic_error(
                        &file.absolute_path,
                        &format!("unknown unqualified type `{name}`; cross-file references require use or a fully qualified name"),
                    )
                })?
        } else {
            segments.join("::")
        };

        let symbol = self
            .symbols
            .get(&key)
            .ok_or_else(|| semantic_error(&file.absolute_path, &format!("unknown schema symbol `{key}`")))?;
        self.ensure_visible(file_index, symbol)?;
        ensure_type_symbol(file, symbol)?;
        Ok(key)
    }

    fn ensure_visible(&self, file_index: usize, symbol: &SchemaSymbol) -> Result<(), CodegenError> {
        let file = &self.files[file_index];
        if symbol.owner == file.owner {
            return Ok(());
        }

        let manifest = self.manifests.modules.get(&file.owner).expect("schema file owner must exist in manifest graph");
        if manifest.direct_dependencies.contains(&symbol.owner) {
            return Ok(());
        }

        Err(semantic_error(
            &file.absolute_path,
            &format!(
                "schema symbol `{}` is owned by undeclared or transitive dependency `{}`; module `{}` must declare it directly",
                symbol.key, symbol.owner, file.owner
            ),
        ))
    }
}

fn ensure_type_symbol(file: &SchemaFile, symbol: &SchemaSymbol) -> Result<(), CodegenError> {
    if symbol.kind == SymbolKind::Const {
        return Err(semantic_error(
            &file.absolute_path,
            &format!("constant `{}` cannot be imported or referenced as a type", symbol.key),
        ));
    }
    Ok(())
}

fn discover_all_sources(manifests: &ManifestGraph) -> Result<Vec<DiscoveredSource>, CodegenError> {
    let mut discovered = BTreeMap::<PathBuf, DiscoveredSource>::new();
    for (owner, manifest) in &manifests.modules {
        for source in &manifest.config.sources {
            discover_source_files(owner, &manifest.root_dir, source, &mut discovered)?;
        }
    }
    Ok(discovered.into_values().collect())
}

fn discover_source_files(owner: &str, root_dir: &Path, source: &SourceConfig, discovered: &mut BTreeMap<PathBuf, DiscoveredSource>) -> Result<(), CodegenError> {
    let base_dir = root_dir.join(&source.base_dir);
    if !base_dir.is_dir() {
        return Err(CodegenError::Other(format!(
            "source base_dir is not a directory for module `{owner}`: {}",
            base_dir.display()
        )));
    }

    let base_dir = fs::canonicalize(&base_dir)?;
    let mut relative_paths = Vec::new();
    collect_relative_files(&base_dir, &base_dir, &mut relative_paths)?;

    for relative_path in relative_paths {
        let normalized = normalize_path(&relative_path);
        let included = source.includes.is_empty() || source.includes.iter().any(|pattern| glob_matches(pattern, &normalized));
        let excluded = source.excludes.iter().any(|pattern| glob_matches(pattern, &normalized));
        if !included || excluded {
            continue;
        }

        let absolute_path = fs::canonicalize(base_dir.join(&relative_path))?;
        if let Some(previous) = discovered.get(&absolute_path) {
            if previous.owner != owner {
                return Err(CodegenError::Other(format!(
                    "RPF source {} is owned by both `{}` and `{owner}`",
                    absolute_path.display(),
                    previous.owner
                )));
            }
            continue;
        }

        discovered.insert(
            absolute_path.clone(),
            DiscoveredSource {
                owner: owner.to_string(),
                absolute_path,
                relative_path,
            },
        );
    }
    Ok(())
}

fn collect_relative_files(base_dir: &Path, current_dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), CodegenError> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_files(base_dir, &path, out)?;
        } else if path.is_file() {
            out.push(path.strip_prefix(base_dir).map_err(|err| CodegenError::Other(err.to_string()))?.to_path_buf());
        }
    }
    Ok(())
}

fn semantic_error(path: &Path, message: &str) -> CodegenError {
    CodegenError::Other(format!("semantic error in {}: {message}", path.display()))
}

pub fn path_segments(path: &AstPath) -> Vec<String> {
    path.segments.iter().map(|segment| segment.value.clone()).collect()
}

pub fn builtin_type(path: &AstPath) -> Option<BuiltinType> {
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
        "Timestamp64" => Some(BuiltinType::Timestamp64),
        "Timestamp96" => Some(BuiltinType::Timestamp96),
        _ => None,
    }
}

fn is_direct_variable_length_type(ty: &Type) -> bool {
    matches!(ty, Type::Vec(_) | Type::Map(_, _)) || matches!(ty, Type::Path(path) if matches!(builtin_type(path), Some(BuiltinType::String | BuiltinType::Bytes)))
}

fn unsigned_integer_max(ty: &Type) -> Option<u128> {
    let Type::Path(path) = ty else { return None };
    match builtin_type(path)? {
        BuiltinType::U8 => Some(u8::MAX as u128),
        BuiltinType::U16 => Some(u16::MAX as u128),
        BuiltinType::U32 => Some(u32::MAX as u128),
        BuiltinType::U64 => Some(u64::MAX as u128),
        _ => None,
    }
}

fn is_reserved_type_name(name: &str) -> bool {
    matches!(name, "Timestamp64" | "Timestamp96")
}

fn validate_unique_tags(path: &Path, context: &str, kind: &str, tags: impl Iterator<Item = u32>) -> Result<(), CodegenError> {
    let mut seen = BTreeSet::new();
    for tag in tags {
        if !seen.insert(tag) {
            return Err(semantic_error(path, &format!("duplicate {kind} tag @{tag} in {context}")));
        }
    }
    Ok(())
}

fn validate_default(path: &Path, parent: &str, field: &str, resolved: &ResolvedType, default: Option<&Literal>) -> Result<(), CodegenError> {
    let Some(default) = default else { return Ok(()) };
    if contains_timestamp_type(resolved) {
        return Err(semantic_error(path, &format!("timestamp types do not support default literals at {parent}.{field}")));
    }
    let Some((builtin, constraint)) = direct_literal_constraint(resolved) else {
        return Ok(());
    };
    let actual = match (builtin, default) {
        (BuiltinType::String, Literal::String(value)) => value.len() as u64,
        (BuiltinType::Bytes, Literal::Bytes(value)) => value.len() as u64,
        _ => return Ok(()),
    };
    if actual < constraint.min || actual > constraint.max {
        return Err(semantic_error(
            path,
            &format!("default literal length {actual} is outside {}..={} at {parent}.{field}", constraint.min, constraint.max),
        ));
    }
    Ok(())
}

fn contains_timestamp_type(resolved: &ResolvedType) -> bool {
    match resolved {
        ResolvedType::Builtin(BuiltinType::Timestamp64 | BuiltinType::Timestamp96) => true,
        ResolvedType::Option(inner) | ResolvedType::Vec(inner) | ResolvedType::Array(inner, _) | ResolvedType::Constrained(inner, _) => contains_timestamp_type(inner),
        ResolvedType::Map(key, value) => contains_timestamp_type(key) || contains_timestamp_type(value),
        ResolvedType::Builtin(_) | ResolvedType::Named(_) => false,
    }
}

fn direct_literal_constraint(resolved: &ResolvedType) -> Option<(BuiltinType, LengthConstraint)> {
    match resolved {
        ResolvedType::Constrained(inner, constraint) => match inner.as_ref() {
            ResolvedType::Builtin(BuiltinType::String) => Some((BuiltinType::String, *constraint)),
            ResolvedType::Builtin(BuiltinType::Bytes) => Some((BuiltinType::Bytes, *constraint)),
            _ => None,
        },
        ResolvedType::Option(inner) => direct_literal_constraint(inner),
        _ => None,
    }
}

fn normalize_path(path: &Path) -> String {
    path.components().map(|component| component.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
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

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn metadata_and_duplicate_symbols() -> TestResult {
        let temp = tempfile::tempdir()?;
        let missing_package = write_module(temp.path(), "missing", "", &[("missing.rpf", "version 1;\nstruct Item {}\n")])?;
        let graph = ManifestGraph::load(missing_package).await?;
        let error = SemanticGraph::build(graph).expect_err("package must be required");
        assert!(error.to_string().contains("missing required package"));

        let duplicate = write_module(
            temp.path(),
            "duplicate",
            "",
            &[
                ("a.rpf", "version 1;\npackage example::v1;\nstruct Item {}\n"),
                ("b.rpf", "version 1;\npackage example::v1;\nstruct Item {}\n"),
            ],
        )?;
        let graph = ManifestGraph::load(duplicate).await?;
        let error = SemanticGraph::build(graph).expect_err("global symbol duplicates must fail");
        assert!(error.to_string().contains("duplicate schema symbol `example::v1::Item`"));
        assert!(error.to_string().contains("a.rpf"));
        assert!(error.to_string().contains("b.rpf"));
        assert!(error.to_string().contains("bytes"));
        Ok(())
    }

    #[tokio::test]
    async fn cross_file_references_require_explicit_names() -> TestResult {
        let temp = tempfile::tempdir()?;
        let manifest = write_module(
            temp.path(),
            "root",
            "",
            &[
                ("common.rpf", "version 1;\npackage example::v1;\nstruct Item {}\nstruct Other {}\n"),
                (
                    "consumer.rpf",
                    "version 1;\npackage example::v1;\nuse example::v1::Item as ImportedItem;\nstruct Consumer {\n  @1 item: ImportedItem;\n  @2 other: example::v1::Other;\n}\n",
                ),
            ],
        )?;
        let graph = SemanticGraph::build(ManifestGraph::load(manifest).await?)?;
        let consumer_index = graph
            .files
            .iter()
            .position(|file| file.relative_path == Path::new("consumer.rpf"))
            .expect("consumer file must exist");
        let item = match &graph.files[consumer_index].ast.items[0] {
            Item::Struct(item) => item,
            _ => panic!("consumer item must be a struct"),
        };
        assert_eq!(
            graph.resolve_type(consumer_index, &item.fields[0].ty.value)?,
            ResolvedType::Named("example::v1::Item".to_string())
        );
        assert_eq!(
            graph.resolve_type(consumer_index, &item.fields[1].ty.value)?,
            ResolvedType::Named("example::v1::Other".to_string())
        );

        let implicit = write_module(
            temp.path(),
            "implicit",
            "",
            &[
                ("common.rpf", "version 1;\npackage implicit::v1;\nstruct Item {}\n"),
                ("consumer.rpf", "version 1;\npackage implicit::v1;\nstruct Consumer { @1 item: Item; }\n"),
            ],
        )?;
        let error = SemanticGraph::build(ManifestGraph::load(implicit).await?).expect_err("implicit cross-file type must fail");
        assert!(error.to_string().contains("cross-file references require use"));
        Ok(())
    }

    #[tokio::test]
    async fn transitive_dependencies_are_not_visible() -> TestResult {
        let temp = tempfile::tempdir()?;
        write_module(temp.path(), "leaf", "", &[("leaf.rpf", "version 1;\npackage leaf::v1;\nstruct Leaf {}\n")])?;
        write_module(
            temp.path(),
            "middle",
            "dependencies:\n  leaf:\n    path: ../leaf/rocketpack.yaml\n",
            &[("middle.rpf", "version 1;\npackage middle::v1;\nstruct Middle {}\n")],
        )?;
        let root = write_module(
            temp.path(),
            "root",
            "dependencies:\n  middle:\n    path: ../middle/rocketpack.yaml\n",
            &[("root.rpf", "version 1;\npackage root::v1;\nuse leaf::v1::Leaf;\nstruct Root { @1 leaf: Leaf; }\n")],
        )?;
        let error = SemanticGraph::build(ManifestGraph::load(root).await?).expect_err("transitive dependency must not be visible");
        assert!(error.to_string().contains("transitive dependency `leaf`"));
        Ok(())
    }

    #[tokio::test]
    async fn invalid_bindings_unknown_symbols_and_alias_cycles_fail() -> TestResult {
        let temp = tempfile::tempdir()?;
        let collision = write_module(
            temp.path(),
            "collision",
            "",
            &[
                ("common.rpf", "version 1;\npackage collision::v1;\nstruct Item {}\n"),
                ("consumer.rpf", "version 1;\npackage collision::v1;\nuse collision::v1::Item as Local;\nstruct Local {}\n"),
            ],
        )?;
        let error = SemanticGraph::build(ManifestGraph::load(collision).await?).expect_err("local and imported names must not collide");
        assert!(error.to_string().contains("use binding `Local` conflicts"));

        let unknown = write_module(
            temp.path(),
            "unknown",
            "",
            &[("unknown.rpf", "version 1;\npackage unknown::v1;\nstruct Consumer { @1 item: missing::v1::Item; }\n")],
        )?;
        let error = SemanticGraph::build(ManifestGraph::load(unknown).await?).expect_err("unknown symbol must fail");
        assert!(error.to_string().contains("unknown schema symbol `missing::v1::Item`"));

        let cycle = write_module(
            temp.path(),
            "alias-cycle",
            "",
            &[("cycle.rpf", "version 1;\npackage cycle::v1;\ntype A = B;\ntype B = A;\n")],
        )?;
        let error = SemanticGraph::build(ManifestGraph::load(cycle).await?).expect_err("alias cycle must fail");
        assert!(error.to_string().contains("cyclic type alias: cycle::v1::B -> cycle::v1::A -> cycle::v1::B"));
        Ok(())
    }

    fn write_module(root: &Path, name: &str, extra: &str, sources: &[(&str, &str)]) -> std::io::Result<PathBuf> {
        let directory = root.join(name);
        let source_dir = directory.join("rpfs");
        fs::create_dir_all(&source_dir)?;
        for (relative_path, contents) in sources {
            fs::write(source_dir.join(relative_path), contents)?;
        }
        let manifest = directory.join("rocketpack.yaml");
        fs::write(
            &manifest,
            format!("version: 1\nname: {name}\n{extra}sources:\n  - base_dir: rpfs\n    includes:\n      - '**/*.rpf'\n"),
        )?;
        Ok(manifest)
    }
}
