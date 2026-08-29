use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use serde::Deserialize;

/// 走査の起点に置く設定ファイルの名前。
const FILE_NAME: &str = "lint-style.toml";

/// 名前だけで走査から外すディレクトリ。
/// build 生成物と version 管理の内部は、どの repository でも検査対象にならない。
const IGNORED_DIR_NAMES: [&str; 2] = ["target", ".git"];

/// `lint-style.toml` の外部形式。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    exclude: Vec<PathBuf>,
}

pub struct Config {
    excludes: Vec<PathBuf>,
}

impl Config {
    pub fn load(base: &Path) -> Result<Self> {
        let path = base.join(FILE_NAME);
        let text = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let file: ConfigFile = toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;

        Ok(Self {
            excludes: file.exclude.iter().map(|relative| base.join(relative)).collect(),
        })
    }

    pub fn is_excluded(&self, path: &Path) -> bool {
        if path.file_name().is_some_and(|name| IGNORED_DIR_NAMES.iter().any(|ignored| name == *ignored)) {
            return true;
        }
        self.excludes.iter().any(|excluded| path.starts_with(excluded))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::Config;

    fn config(excludes: &[&str]) -> Config {
        Config {
            excludes: excludes.iter().map(|excluded| Path::new("/repo").join(excluded)).collect(),
        }
    }

    #[test]
    fn excludes_configured_paths_and_their_contents() {
        let config = config(&["refs", "entrypoints/interface"]);

        assert!(config.is_excluded(&PathBuf::from("/repo/refs")));
        assert!(config.is_excluded(&PathBuf::from("/repo/refs/core-rs/modules/base/src/lib.rs")));
        assert!(config.is_excluded(&PathBuf::from("/repo/entrypoints/interface/src/models.rs")));
        assert!(!config.is_excluded(&PathBuf::from("/repo/entrypoints/daemon/src/main.rs")));
    }

    #[test]
    fn excludes_ignored_directory_names_anywhere() {
        let config = config(&[]);

        assert!(config.is_excluded(&PathBuf::from("/repo/target")));
        assert!(config.is_excluded(&PathBuf::from("/repo/modules/engine/target")));
        assert!(config.is_excluded(&PathBuf::from("/repo/.git")));
        assert!(!config.is_excluded(&PathBuf::from("/repo/modules/engine/src")));
    }

    #[test]
    fn does_not_exclude_paths_that_only_share_a_name_prefix() {
        let config = config(&["refs"]);

        assert!(!config.is_excluded(&PathBuf::from("/repo/refs-extra/src/lib.rs")));
    }
}
