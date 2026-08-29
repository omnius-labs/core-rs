use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};

use crate::marker::Marker;

pub struct SourceFile {
    display_path: PathBuf,
    lines: Vec<String>,
    ast: syn::File,
    markers: BTreeMap<usize, Result<Marker, String>>,
}

impl SourceFile {
    pub fn read(path: &Path, base: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
        let display_path = path.strip_prefix(base).unwrap_or(path);
        Self::parse(display_path, &text)
    }

    fn parse(display_path: &Path, text: &str) -> Result<Self> {
        let ast = syn::parse_file(text).with_context(|| format!("failed to parse {}", display_path.display()))?;
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let markers = lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| Marker::parse(line).map(|marker| (index + 1, marker)))
            .collect();

        Ok(Self {
            display_path: display_path.to_path_buf(),
            lines,
            ast,
            markers,
        })
    }

    pub fn ast(&self) -> &syn::File {
        &self.ast
    }

    pub fn display_path(&self) -> &Path {
        &self.display_path
    }

    pub fn markers(&self) -> &BTreeMap<usize, Result<Marker, String>> {
        &self.markers
    }

    pub fn marker_above(&self, line: usize) -> Option<(usize, &Result<Marker, String>)> {
        let mut index = line.checked_sub(1)?;
        while index > 0 {
            let trimmed = self.lines.get(index - 1)?.trim();
            if trimmed.is_empty() || trimmed.starts_with("#[") || trimmed.starts_with("#![") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                index -= 1;
                continue;
            }
            return self.markers.get(&index).map(|marker| (index, marker));
        }
        None
    }

    #[cfg(test)]
    pub fn for_test(text: &str) -> Self {
        Self::parse(Path::new("test.rs"), text).expect("test source must parse")
    }
}
