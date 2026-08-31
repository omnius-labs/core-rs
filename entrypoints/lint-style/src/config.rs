use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use serde::Deserialize;

const CONFIG_FILE_NAME: &str = "lint-style.toml";
const IGNORED_DIR_NAMES: [&str; 2] = ["target", ".git"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigToml {
    #[serde(default)]
    exclude: Vec<PathBuf>,
}

pub struct Config {
    excludes: Vec<PathBuf>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(dir: P) -> anyhow::Result<Self> {
        let dir = dir.as_ref();
        let path = dir.join(CONFIG_FILE_NAME);
        let text = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let file: ConfigToml = toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;

        Ok(Self {
            excludes: file.exclude.iter().map(|relative| dir.join(relative)).collect(),
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

    #[test]
    fn excludes_configured_paths_and_their_contents() {
        let config = gen_config("/repo", &["refs", "entrypoints/interface"]);

        assert!(config.is_excluded(&PathBuf::from("/repo/refs")));
        assert!(config.is_excluded(&PathBuf::from("/repo/refs/core-rs/modules/base/src/lib.rs")));
        assert!(config.is_excluded(&PathBuf::from("/repo/entrypoints/interface/src/models.rs")));
        assert!(!config.is_excluded(&PathBuf::from("/repo/entrypoints/daemon/src/main.rs")));
    }

    #[test]
    fn excludes_ignored_directory_names_anywhere() {
        let config = gen_config("/repo", &[]);

        assert!(config.is_excluded(&PathBuf::from("/repo/target")));
        assert!(config.is_excluded(&PathBuf::from("/repo/modules/engine/target")));
        assert!(config.is_excluded(&PathBuf::from("/repo/.git")));
        assert!(!config.is_excluded(&PathBuf::from("/repo/modules/engine/src")));
    }

    #[test]
    fn does_not_exclude_paths_that_only_share_a_name_prefix() {
        let config = gen_config("/repo", &["refs"]);

        assert!(!config.is_excluded(&PathBuf::from("/repo/refs-extra/src/lib.rs")));
    }

    fn gen_config<P: AsRef<Path>>(dir: P, excludes: &[&str]) -> Config {
        let dir = dir.as_ref();
        Config {
            excludes: excludes.iter().map(|excluded| dir.join(excluded)).collect(),
        }
    }
}
