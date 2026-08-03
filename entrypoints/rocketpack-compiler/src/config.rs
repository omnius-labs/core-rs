use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml_ng::{Mapping, from_str};

use crate::error::ConfigError;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub generators: Vec<GeneratorConfig>,
    #[serde(skip)]
    pub root_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct SourceConfig {
    #[serde(rename = "base_dir")]
    pub base_dir: String,
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub excludes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratorConfig {
    pub id: String,
    pub plugin: String,
    #[expect(dead_code, reason = "reserved for future generator-wide options")]
    #[serde(default)]
    pub options: Option<Mapping>,
    #[serde(default)]
    pub targets: Vec<GeneratorTargetConfig>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratorTargetConfig {
    pub pattern: String,
    #[serde(default)]
    pub options: Option<Mapping>,
}

impl AppConfig {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_buf: PathBuf = path.as_ref().into();
        let contents = tokio::fs::read_to_string(&path_buf).await?;
        let mut config = Self::from_yaml(&contents)?;
        config.root_dir = path_buf.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        Ok(config)
    }

    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        let config: Self = from_str(yaml)?;
        if config.version != 1 {
            return Err(ConfigError::UnsupportedVersion(config.version));
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn config_load_test() -> TestResult {
        let config_path = Path::new("../rocketpack-compiled-example/rocketpack.yaml");
        let config = AppConfig::load(config_path).await?;
        assert_eq!(config.version, 1);
        assert_eq!(config.root_dir, Path::new("../rocketpack-compiled-example"));

        Ok(())
    }

    #[test]
    fn rejects_unsupported_version() {
        let result = AppConfig::from_yaml("version: 2");
        assert!(matches!(result, Err(ConfigError::UnsupportedVersion(2))));
    }
}
