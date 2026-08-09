use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_yaml_ng::{Mapping, from_str};

use crate::error::ConfigError;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencyConfig>,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub generators: Vec<GeneratorConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConfig {
    #[serde(rename = "base_dir")]
    pub base_dir: String,
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub excludes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorConfig {
    pub id: String,
    pub plugin: String,
    #[serde(default)]
    pub options: Option<Mapping>,
    #[serde(default)]
    pub targets: Vec<GeneratorTargetConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorTargetConfig {
    #[allow(dead_code, reason = "legacy target fields are parsed so the Rust generator can reject them explicitly")]
    pub pattern: String,
    #[serde(default)]
    #[allow(dead_code, reason = "legacy target fields are parsed so the Rust generator can reject them explicitly")]
    pub options: Option<Mapping>,
}

#[derive(Debug, Clone)]
pub struct LoadedManifest {
    pub config: AppConfig,
    #[allow(dead_code, reason = "retained for diagnostics and future remote dependency identity")]
    pub manifest_path: PathBuf,
    pub root_dir: PathBuf,
    pub direct_dependencies: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct ManifestGraph {
    pub root_name: String,
    pub modules: BTreeMap<String, LoadedManifest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

#[derive(Default)]
struct GraphLoader {
    modules: BTreeMap<String, LoadedManifest>,
    names_by_path: BTreeMap<PathBuf, String>,
    paths_by_name: BTreeMap<String, PathBuf>,
    states: BTreeMap<PathBuf, VisitState>,
    stack: Vec<PathBuf>,
}

impl AppConfig {
    #[cfg_attr(not(test), allow(dead_code, reason = "used by focused config tests; graph loading is the production entry point"))]
    pub async fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = tokio::fs::read_to_string(path).await?;
        let config = Self::from_yaml(&contents)?;
        validate_manifest_header(&config, path)?;
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

impl ManifestGraph {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();
        tokio::task::spawn_blocking(move || Self::load_sync(&path))
            .await
            .map_err(|err| ConfigError::Invalid(format!("manifest loader task failed: {err}")))?
    }

    fn load_sync(path: &Path) -> Result<Self, ConfigError> {
        let canonical_path = canonical_manifest_path(path)?;
        let mut loader = GraphLoader::default();
        let root_name = loader.visit(&canonical_path, None)?;
        Ok(Self {
            root_name,
            modules: loader.modules,
        })
    }

    pub fn root(&self) -> &LoadedManifest {
        self.modules.get(&self.root_name).expect("manifest graph root must exist")
    }
}

impl GraphLoader {
    fn visit(&mut self, manifest_path: &Path, expected_name: Option<&str>) -> Result<String, ConfigError> {
        let canonical_path = canonical_manifest_path(manifest_path)?;

        if let Some(state) = self.states.get(&canonical_path) {
            let actual_name = self.names_by_path.get(&canonical_path).expect("visited manifest path must have a name").clone();
            validate_expected_name(expected_name, &actual_name, &canonical_path)?;

            if *state == VisitState::Visiting {
                return Err(self.cycle_error(&canonical_path));
            }

            return Ok(actual_name);
        }

        let contents = fs::read_to_string(&canonical_path)?;
        let config = AppConfig::from_yaml(&contents)?;
        validate_manifest_header(&config, &canonical_path)?;
        validate_expected_name(expected_name, &config.name, &canonical_path)?;
        self.register_identity(&config.name, &canonical_path)?;

        self.states.insert(canonical_path.clone(), VisitState::Visiting);
        self.stack.push(canonical_path.clone());

        let root_dir = canonical_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let mut direct_dependencies = BTreeSet::new();

        for (dependency_name, dependency) in &config.dependencies {
            let dependency_path = root_dir.join(&dependency.path);
            let actual_name = self.visit(&dependency_path, Some(dependency_name))?;
            direct_dependencies.insert(actual_name);
        }

        self.stack.pop();
        self.states.insert(canonical_path.clone(), VisitState::Visited);
        self.modules.insert(
            config.name.clone(),
            LoadedManifest {
                config: config.clone(),
                manifest_path: canonical_path,
                root_dir,
                direct_dependencies,
            },
        );

        Ok(config.name)
    }

    fn register_identity(&mut self, name: &str, path: &Path) -> Result<(), ConfigError> {
        if let Some(previous_path) = self.paths_by_name.get(name)
            && previous_path != path
        {
            return Err(ConfigError::Invalid(format!(
                "manifest name `{name}` resolves to different paths: {} and {}",
                previous_path.display(),
                path.display()
            )));
        }

        if let Some(previous_name) = self.names_by_path.get(path)
            && previous_name != name
        {
            return Err(ConfigError::Invalid(format!(
                "manifest path {} declares different names: `{previous_name}` and `{name}`",
                path.display()
            )));
        }

        self.paths_by_name.insert(name.to_string(), path.to_path_buf());
        self.names_by_path.insert(path.to_path_buf(), name.to_string());
        Ok(())
    }

    fn cycle_error(&self, repeated_path: &Path) -> ConfigError {
        let start = self.stack.iter().position(|path| path == repeated_path).unwrap_or(0);
        let mut entries = self.stack[start..]
            .iter()
            .map(|path| {
                let name = self.names_by_path.get(path).map(String::as_str).unwrap_or("<unknown>");
                format!("{name} ({})", path.display())
            })
            .collect::<Vec<_>>();
        let repeated_name = self.names_by_path.get(repeated_path).map(String::as_str).unwrap_or("<unknown>");
        entries.push(format!("{repeated_name} ({})", repeated_path.display()));
        ConfigError::Invalid(format!("manifest dependency cycle: {}", entries.join(" -> ")))
    }
}

fn validate_manifest_header(config: &AppConfig, path: &Path) -> Result<(), ConfigError> {
    if config.version != 1 {
        return Err(ConfigError::Invalid(format!(
            "unsupported manifest version {} in {}; expected 1",
            config.version,
            path.display()
        )));
    }

    if config.name.trim().is_empty() {
        return Err(ConfigError::Invalid(format!("manifest name must not be empty: {}", path.display())));
    }

    Ok(())
}

fn validate_expected_name(expected_name: Option<&str>, actual_name: &str, path: &Path) -> Result<(), ConfigError> {
    if let Some(expected_name) = expected_name
        && expected_name != actual_name
    {
        return Err(ConfigError::Invalid(format!(
            "dependency key `{expected_name}` does not match manifest name `{actual_name}` in {}",
            path.display()
        )));
    }
    Ok(())
}

fn canonical_manifest_path(path: &Path) -> Result<PathBuf, ConfigError> {
    fs::canonicalize(path).map_err(|err| ConfigError::Invalid(format!("failed to resolve manifest path {}: {err}", path.display())))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn config_load_test() -> TestResult {
        let config_path = Path::new("../rocketpack-compiled-example/showcase/rocketpack.yaml");
        let config = AppConfig::load(config_path).await?;
        assert_eq!(config.version, 1);
        assert_eq!(config.name, "rocketpack-showcase-schema");
        Ok(())
    }

    #[tokio::test]
    async fn dependency_graph() -> TestResult {
        let temp = tempfile::tempdir()?;
        write_manifest(temp.path(), "common", "version: 1\nname: common\n")?;
        write_manifest(
            temp.path(),
            "left",
            "version: 1\nname: left\ndependencies:\n  common:\n    path: ../common/rocketpack.yaml\n",
        )?;
        write_manifest(
            temp.path(),
            "right",
            "version: 1\nname: right\ndependencies:\n  common:\n    path: ../common/rocketpack.yaml\n",
        )?;
        let root = write_manifest(
            temp.path(),
            "root",
            "version: 1\nname: root\ndependencies:\n  left:\n    path: ../left/rocketpack.yaml\n  right:\n    path: ../right/rocketpack.yaml\n",
        )?;

        let graph = ManifestGraph::load(&root).await?;
        assert_eq!(graph.root_name, "root");
        assert_eq!(graph.modules.len(), 4);
        assert_eq!(graph.modules["root"].direct_dependencies, BTreeSet::from(["left".to_string(), "right".to_string()]));

        let invalid_version = write_manifest(temp.path(), "invalid-version", "version: 2\nname: invalid-version\n")?;
        let error = ManifestGraph::load(invalid_version).await.expect_err("unsupported version must fail");
        assert!(matches!(error, ConfigError::UnsupportedVersion(2)));

        let missing_name = write_manifest(temp.path(), "missing-name", "version: 1\n")?;
        let error = ManifestGraph::load(missing_name).await.expect_err("missing name must fail");
        assert!(error.to_string().contains("missing field `name`"));

        let mismatched = write_manifest(
            temp.path(),
            "mismatched",
            "version: 1\nname: mismatched\ndependencies:\n  alias:\n    path: ../common/rocketpack.yaml\n",
        )?;
        let error = ManifestGraph::load(mismatched).await.expect_err("dependency key mismatch must fail");
        assert!(error.to_string().contains("dependency key `alias`"));

        let cycle_a = write_manifest(
            temp.path(),
            "cycle-a",
            "version: 1\nname: cycle-a\ndependencies:\n  cycle-b:\n    path: ../cycle-b/rocketpack.yaml\n",
        )?;
        write_manifest(
            temp.path(),
            "cycle-b",
            "version: 1\nname: cycle-b\ndependencies:\n  cycle-a:\n    path: ../cycle-a/rocketpack.yaml\n",
        )?;
        let error = ManifestGraph::load(cycle_a).await.expect_err("cycle must fail");
        assert!(error.to_string().contains("cycle-a"));
        assert!(error.to_string().contains("cycle-b"));

        write_manifest(temp.path(), "common-other", "version: 1\nname: common\n")?;
        write_manifest(
            temp.path(),
            "identity-left",
            "version: 1\nname: identity-left\ndependencies:\n  common:\n    path: ../common/rocketpack.yaml\n",
        )?;
        write_manifest(
            temp.path(),
            "identity-right",
            "version: 1\nname: identity-right\ndependencies:\n  common:\n    path: ../common-other/rocketpack.yaml\n",
        )?;
        let identity_root = write_manifest(
            temp.path(),
            "identity-root",
            "version: 1\nname: identity-root\ndependencies:\n  identity-left:\n    path: ../identity-left/rocketpack.yaml\n  identity-right:\n    path: ../identity-right/rocketpack.yaml\n",
        )?;
        let error = ManifestGraph::load(identity_root).await.expect_err("same name at different paths must fail");
        assert!(error.to_string().contains("manifest name `common` resolves to different paths"));

        let missing_dependency = write_manifest(
            temp.path(),
            "missing-dependency",
            "version: 1\nname: missing-dependency\ndependencies:\n  absent:\n    path: ../absent/rocketpack.yaml\n",
        )?;
        let error = ManifestGraph::load(missing_dependency).await.expect_err("missing dependency manifest must fail");
        assert!(error.to_string().contains("failed to resolve manifest path"));

        Ok(())
    }

    #[test]
    fn rejects_unsupported_version() {
        let result = AppConfig::from_yaml("version: 2\nname: test\n");
        assert!(matches!(result, Err(ConfigError::UnsupportedVersion(2))));
    }

    fn write_manifest(root: &Path, directory: &str, contents: &str) -> std::io::Result<PathBuf> {
        let directory = root.join(directory);
        fs::create_dir_all(&directory)?;
        let path = directory.join("rocketpack.yaml");
        fs::write(&path, contents)?;
        Ok(path)
    }
}
