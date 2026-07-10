use cargo_metadata::{DependencyKind, Metadata, MetadataCommand};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Stable machine-readable category for a repository policy violation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ViolationCode {
    AcceleratorDependency,
    AcceleratorFeature,
    AcceleratorSource,
    NativeAcceleratorLink,
    TransformerDependency,
    TransformerSource,
    RemoteModelApi,
    ArcTaskIdLiteral,
    EvaluationPath,
    ForbiddenDependencyDirection,
    UndeclaredWorkspaceCrate,
    UnsafeOutsideAllowlist,
    ProtocolDowngrade,
    InvalidPolicyManifest,
    MetadataUnavailable,
}

/// One deterministic policy finding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Violation {
    pub code: ViolationCode,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

/// Options controlling a read-only repository policy check.
#[derive(Clone, Debug)]
pub struct CheckOptions {
    /// Repository root containing `Cargo.toml`.
    pub path: PathBuf,
    /// Relative directory names excluded from traversal.
    pub excluded_directories: BTreeSet<String>,
}

impl CheckOptions {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            excluded_directories: [".git", "artifacts", "target"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }
}

/// Complete deterministic checker result.
#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    pub schema_version: u32,
    pub root: String,
    pub scanned_files: usize,
    pub duration_ms: u128,
    pub violations: Vec<Violation>,
}

impl CheckReport {
    /// Whether no policy boundary was crossed.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Trust-boundary error that prevented a complete check.
#[derive(Debug)]
pub enum CheckError {
    Io { path: PathBuf, source: io::Error },
    MissingWorkspace(PathBuf),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot inspect {}: {source}", path.display()),
            Self::MissingWorkspace(path) => {
                write!(f, "workspace manifest not found at {}", path.display())
            }
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::MissingWorkspace(_) => None,
        }
    }
}

#[derive(Debug)]
struct LayerPolicy {
    layers: Vec<String>,
    crates: BTreeMap<String, CratePolicy>,
}

#[derive(Debug)]
struct CratePolicy {
    layer: String,
    allow_unsafe_modules: BTreeSet<String>,
}

/// Inspect a repository without modifying it.
///
/// Findings are sorted independently of filesystem, hash-map, and Cargo graph
/// iteration order. Malformed policy inputs fail closed as structured findings;
/// filesystem failures at the repository boundary are returned as typed errors.
///
/// # Errors
///
/// Returns [`CheckError`] when the workspace root is absent or a traversed
/// policy input cannot be read. Invalid semantic policy data is instead
/// retained as a machine-readable violation in the completed report.
pub fn check_repository(options: &CheckOptions) -> Result<CheckReport, CheckError> {
    let started = Instant::now();
    let root = canonical_root(&options.path)?;
    if !root.join("Cargo.toml").is_file() {
        return Err(CheckError::MissingWorkspace(root.join("Cargo.toml")));
    }

    let mut files = Vec::new();
    collect_files(&root, &root, &options.excluded_directories, &mut files)?;
    files.sort();
    let mut violations = Vec::new();

    let policy_path = root.join("policy/dependency-layers.toml");
    let layer_policy = match read_layer_policy(&policy_path) {
        Ok(policy) => Some(policy),
        Err(message) => {
            violations.push(violation(
                ViolationCode::InvalidPolicyManifest,
                "policy/dependency-layers.toml",
                None,
                message,
            ));
            None
        }
    };

    for file in &files {
        scan_file(&root, file, layer_policy.as_ref(), &mut violations)?;
    }

    match load_metadata(&root) {
        Ok(metadata) => scan_metadata(&metadata, layer_policy.as_ref(), &mut violations),
        Err(message) => violations.push(violation(
            ViolationCode::MetadataUnavailable,
            "Cargo.toml",
            None,
            message,
        )),
    }

    violations.sort();
    violations.dedup();
    Ok(CheckReport {
        schema_version: 1,
        root: root.display().to_string(),
        scanned_files: files.len(),
        duration_ms: started.elapsed().as_millis(),
        violations,
    })
}

fn canonical_root(path: &Path) -> Result<PathBuf, CheckError> {
    fs::canonicalize(path).map_err(|source| CheckError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn collect_files(
    root: &Path,
    directory: &Path,
    excluded: &BTreeSet<String>,
    output: &mut Vec<PathBuf>,
) -> Result<(), CheckError> {
    let entries = fs::read_dir(directory).map_err(|source| CheckError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    let mut paths = entries
        .map(|entry| {
            entry
                .map(|value| value.path())
                .map_err(|source| CheckError::Io {
                    path: directory.to_path_buf(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();

    for path in paths {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if excluded.contains(file_name) || is_handoff_documentation(root, &path) {
                continue;
            }
            collect_files(root, &path, excluded, output)?;
        } else if is_policy_input(&path) {
            output.push(path);
        }
    }
    Ok(())
}

fn is_handoff_documentation(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .is_ok_and(|relative| relative.starts_with("docs/CAIRN_handoff"))
}

fn is_policy_input(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    name == "Cargo.toml"
        || name == "Cargo.lock"
        || name == "build.rs"
        || matches!(extension, "rs" | "toml")
}

fn read_text(path: &Path) -> Result<String, CheckError> {
    fs::read_to_string(path).map_err(|source| CheckError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn read_layer_policy(path: &Path) -> Result<LayerPolicy, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let document = content
        .parse::<toml::Table>()
        .map_err(|error| format!("TOML parse failed: {error}"))?;
    let version = document
        .get("schema_version")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| "missing integer schema_version".to_owned())?;
    if version != 1 {
        return Err(format!("unsupported policy schema version {version}"));
    }
    let layers = document
        .get("layers")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "missing layers array".to_owned())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "layer names must be strings".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut crates = BTreeMap::new();
    let crate_entries = document
        .get("crates")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "missing crates table".to_owned())?;
    for (name, value) in crate_entries {
        let table = value
            .as_table()
            .ok_or_else(|| format!("crate {name} policy must be a table"))?;
        let layer = table
            .get("layer")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("crate {name} is missing layer"))?
            .to_owned();
        if !layers.contains(&layer) {
            return Err(format!("crate {name} names unknown layer {layer}"));
        }
        let allow_unsafe_modules = table
            .get("allow_unsafe_modules")
            .and_then(toml::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        crates.insert(
            name.clone(),
            CratePolicy {
                layer,
                allow_unsafe_modules,
            },
        );
    }
    Ok(LayerPolicy { layers, crates })
}

fn load_metadata(root: &Path) -> Result<Metadata, String> {
    let mut command = MetadataCommand::new();
    command.current_dir(root);
    command.manifest_path(root.join("Cargo.toml"));
    if root.join("Cargo.lock").is_file() {
        command.other_options(["--locked".to_owned()]);
    }
    command.exec().map_err(|error| error.to_string())
}

fn scan_metadata(
    metadata: &Metadata,
    policy: Option<&LayerPolicy>,
    violations: &mut Vec<Violation>,
) {
    let workspace_ids = metadata.workspace_members.iter().collect::<BTreeSet<_>>();
    let packages = metadata
        .packages
        .iter()
        .map(|package| (&package.id, package))
        .collect::<BTreeMap<_, _>>();

    for package in &metadata.packages {
        let name = package.name.as_str();
        if let Some((code, category)) = forbidden_dependency(name) {
            violations.push(violation(
                code,
                package.manifest_path.to_string(),
                None,
                format!("resolved {category} package `{name}`"),
            ));
        }
    }

    if let Some(resolve) = &metadata.resolve {
        for node in &resolve.nodes {
            if let Some(package) = packages.get(&node.id) {
                for feature in &node.features {
                    if accelerator_term(feature).is_some() {
                        violations.push(violation(
                            ViolationCode::AcceleratorFeature,
                            package.manifest_path.to_string(),
                            None,
                            format!("accelerator-related feature `{feature}` is active"),
                        ));
                    }
                }
            }
        }
    }

    let Some(policy) = policy else { return };
    for package in metadata
        .packages
        .iter()
        .filter(|package| workspace_ids.contains(&package.id))
    {
        let Some(source_policy) = policy.crates.get(package.name.as_str()) else {
            violations.push(violation(
                ViolationCode::UndeclaredWorkspaceCrate,
                package.manifest_path.to_string(),
                None,
                format!("workspace crate `{}` has no checked layer", package.name),
            ));
            continue;
        };
        for dependency in package
            .dependencies
            .iter()
            .filter(|dependency| dependency.kind == DependencyKind::Normal)
        {
            let dependency_name = dependency.rename.as_ref().unwrap_or(&dependency.name);
            let Some(target_policy) = policy.crates.get(dependency_name) else {
                continue;
            };
            let source_rank = policy
                .layers
                .iter()
                .position(|item| item == &source_policy.layer);
            let target_rank = policy
                .layers
                .iter()
                .position(|item| item == &target_policy.layer);
            if source_rank
                .zip(target_rank)
                .is_some_and(|(source, target)| target > source)
            {
                violations.push(violation(
                    ViolationCode::ForbiddenDependencyDirection,
                    package.manifest_path.to_string(),
                    None,
                    format!(
                        "crate `{}` in layer `{}` depends on later layer `{}` via `{dependency_name}`",
                        package.name, source_policy.layer, target_policy.layer
                    ),
                ));
            }
        }
    }
}

fn scan_file(
    root: &Path,
    path: &Path,
    policy: Option<&LayerPolicy>,
    violations: &mut Vec<Violation>,
) -> Result<(), CheckError> {
    let relative = relative_string(root, path);
    let content = read_text(path)?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if name == "Cargo.lock" {
        scan_lockfile(&relative, &content, violations);
    }
    if name == "Cargo.toml" {
        scan_manifest(&relative, &content, violations);
    }
    if path.extension().and_then(|value| value.to_str()) == Some("toml")
        && name != "Cargo.toml"
        && relative != "policy/dependency-layers.toml"
    {
        scan_configuration(&relative, &content, violations);
    }
    if path.extension().and_then(|value| value.to_str()) == Some("rs") {
        scan_rust(root, path, &relative, &content, policy, violations);
    }
    if name == "build.rs" || relative.contains(".cargo/config") {
        scan_native_configuration(&relative, &content, violations);
    }
    Ok(())
}

fn scan_configuration(path: &str, content: &str, violations: &mut Vec<Violation>) {
    for (index, line) in content.lines().enumerate() {
        let semantic_line = line.split('#').next().unwrap_or_default();
        for literal in quoted_literals(semantic_line) {
            let lower = literal.to_ascii_lowercase();
            let line_number = Some(index + 1);
            if is_evaluation_path(&lower) {
                violations.push(violation(
                    ViolationCode::EvaluationPath,
                    path,
                    line_number,
                    "configuration references an evaluation path",
                ));
            }
            if is_protocol_downgrade(&lower) {
                violations.push(violation(
                    ViolationCode::ProtocolDowngrade,
                    path,
                    line_number,
                    "configuration requests an unsupported protocol downgrade",
                ));
            }
            if accelerator_term(&lower).is_some() {
                violations.push(violation(
                    ViolationCode::AcceleratorSource,
                    path,
                    line_number,
                    "configuration references an accelerator execution path",
                ));
            }
            if transformer_term(&lower).is_some() {
                violations.push(violation(
                    ViolationCode::TransformerSource,
                    path,
                    line_number,
                    "configuration references a transformer runtime",
                ));
            }
            if remote_model_term(&lower).is_some() {
                violations.push(violation(
                    ViolationCode::RemoteModelApi,
                    path,
                    line_number,
                    "configuration references a remote model API",
                ));
            }
        }
    }
}

fn scan_lockfile(path: &str, content: &str, violations: &mut Vec<Violation>) {
    let Ok(document) = content.parse::<toml::Table>() else {
        return;
    };
    let Some(packages) = document.get("package").and_then(toml::Value::as_array) else {
        return;
    };
    for package in packages {
        let Some(name) = package
            .as_table()
            .and_then(|table| table.get("name"))
            .and_then(toml::Value::as_str)
        else {
            continue;
        };
        if let Some((code, category)) = forbidden_dependency(name) {
            violations.push(violation(
                code,
                path,
                None,
                format!("lockfile contains {category} package `{name}`"),
            ));
        }
    }
}

fn scan_manifest(path: &str, content: &str, violations: &mut Vec<Violation>) {
    let lower = content.to_ascii_lowercase();
    for (index, line) in lower.lines().enumerate() {
        if line.contains("default") || line.contains("features") {
            if let Some(term) = accelerator_term(line) {
                violations.push(violation(
                    ViolationCode::AcceleratorFeature,
                    path,
                    Some(index + 1),
                    format!("manifest enables accelerator-related feature `{term}`"),
                ));
            }
        }
    }
}

fn scan_native_configuration(path: &str, content: &str, violations: &mut Vec<Violation>) {
    for (index, line) in content.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        if (lower.contains("rustc-link-lib") || lower.contains("rustflags"))
            && accelerator_term(&lower).is_some()
        {
            violations.push(violation(
                ViolationCode::NativeAcceleratorLink,
                path,
                Some(index + 1),
                "native link configuration references an accelerator library",
            ));
        }
    }
}

fn scan_rust(
    root: &Path,
    path: &Path,
    relative: &str,
    content: &str,
    policy: Option<&LayerPolicy>,
    violations: &mut Vec<Violation>,
) {
    let lexical = strip_comments(content);
    let policy_engine = matches!(
        relative,
        "crates/cairn-check/src/policy.rs" | "crates/cairn-check/src/attest.rs"
    );
    if relative.contains("/tests/") || relative.starts_with("tests/") {
        return;
    }
    let crate_name = crate_name_for_path(root, path);
    let unsafe_allowed = crate_name
        .as_deref()
        .and_then(|name| policy.and_then(|value| value.crates.get(name)))
        .is_some_and(|crate_policy| {
            crate_policy
                .allow_unsafe_modules
                .iter()
                .any(|module| relative.ends_with(module))
        });

    for (index, line) in lexical.lines().enumerate() {
        let line_number = index + 1;
        if contains_token(&strip_string_literals(line), "unsafe") && !unsafe_allowed {
            violations.push(violation(
                ViolationCode::UnsafeOutsideAllowlist,
                relative,
                Some(line_number),
                "unsafe Rust is outside the checked module allowlist",
            ));
        }
        if !policy_engine {
            for literal in quoted_literals(line) {
                if is_arc_task_id(literal) {
                    violations.push(violation(
                        ViolationCode::ArcTaskIdLiteral,
                        relative,
                        Some(line_number),
                        "production source contains an ARC-shaped task identifier",
                    ));
                }
                let lower = literal.to_ascii_lowercase();
                if is_evaluation_path(&lower) {
                    violations.push(violation(
                        ViolationCode::EvaluationPath,
                        relative,
                        Some(line_number),
                        "production source references an evaluation path",
                    ));
                }
                if is_protocol_downgrade(&lower) {
                    violations.push(violation(
                        ViolationCode::ProtocolDowngrade,
                        relative,
                        Some(line_number),
                        "source requests an unsupported protocol downgrade",
                    ));
                }
                if remote_model_term(&lower).is_some() {
                    violations.push(violation(
                        ViolationCode::RemoteModelApi,
                        relative,
                        Some(line_number),
                        "production source references a remote model API",
                    ));
                }
            }
            let lower = line.to_ascii_lowercase();
            if accelerator_term(&lower).is_some() {
                violations.push(violation(
                    ViolationCode::AcceleratorSource,
                    relative,
                    Some(line_number),
                    "production source references an accelerator execution path",
                ));
            }
            if transformer_term(&lower).is_some() {
                violations.push(violation(
                    ViolationCode::TransformerSource,
                    relative,
                    Some(line_number),
                    "production source references a transformer runtime",
                ));
            }
        }
    }
}

fn crate_name_for_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut components = relative.components();
    if components.next()?.as_os_str() != "crates" {
        return None;
    }
    components
        .next()
        .map(|value| value.as_os_str().to_string_lossy().into_owned())
}

fn strip_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    let mut block_depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    while index < bytes.len() {
        let current = bytes[index];
        let next = bytes.get(index + 1).copied();
        if block_depth > 0 {
            if current == b'/' && next == Some(b'*') {
                block_depth += 1;
                output.push_str("  ");
                index += 2;
            } else if current == b'*' && next == Some(b'/') {
                block_depth -= 1;
                output.push_str("  ");
                index += 2;
            } else {
                output.push(if current == b'\n' { '\n' } else { ' ' });
                index += 1;
            }
        } else if in_string {
            output.push(char::from(current));
            if escaped {
                escaped = false;
            } else if current == b'\\' {
                escaped = true;
            } else if current == b'"' {
                in_string = false;
            }
            index += 1;
        } else if current == b'/' && next == Some(b'/') {
            while index < bytes.len() && bytes[index] != b'\n' {
                output.push(' ');
                index += 1;
            }
        } else if current == b'/' && next == Some(b'*') {
            block_depth = 1;
            output.push_str("  ");
            index += 2;
        } else {
            output.push(char::from(current));
            if current == b'"' {
                in_string = true;
            }
            index += 1;
        }
    }
    output
}

fn quoted_literals(line: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = None;
    let mut escaped = false;
    for (index, byte) in line.bytes().enumerate() {
        if let Some(begin) = start {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                result.push(&line[begin..index]);
                start = None;
            }
        } else if byte == b'"' {
            start = Some(index + 1);
        }
    }
    result
}

fn strip_string_literals(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    for byte in line.bytes() {
        if in_string {
            result.push(' ');
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
            result.push(' ');
        } else {
            result.push(char::from(byte));
        }
    }
    result
}

fn contains_token(line: &str, token: &str) -> bool {
    line.match_indices(token).any(|(index, _)| {
        let before = line[..index].chars().next_back();
        let after = line[index + token.len()..].chars().next();
        before.is_none_or(|value| !is_identifier(value))
            && after.is_none_or(|value| !is_identifier(value))
    })
}

fn is_identifier(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

fn is_arc_task_id(value: &str) -> bool {
    value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_evaluation_path(value: &str) -> bool {
    value.contains("/evaluation/")
        || value.contains("/evaluation")
        || value.contains("evaluation/")
        || value.contains("eval_root")
        || value.contains("eval-path")
}

fn is_protocol_downgrade(value: &str) -> bool {
    value
        .rsplit_once('/')
        .and_then(|(_, version)| version.parse::<u32>().ok())
        .is_some_and(|version| version < 1)
}

fn forbidden_dependency(name: &str) -> Option<(ViolationCode, &'static str)> {
    let lower = name.to_ascii_lowercase();
    if accelerator_term(&lower).is_some() {
        Some((ViolationCode::AcceleratorDependency, "accelerator"))
    } else if transformer_term(&lower).is_some() {
        Some((ViolationCode::TransformerDependency, "transformer"))
    } else if remote_model_term(&lower).is_some() {
        Some((ViolationCode::RemoteModelApi, "remote-model API"))
    } else {
        None
    }
}

fn accelerator_term(value: &str) -> Option<&'static str> {
    [
        "cuda", "cudart", "cublas", "cudnn", "rocm", "hip-sys", "opencl", "metal", "vulkan",
        "tensorrt", "wgpu", "gpu",
    ]
    .into_iter()
    .find(|term| value.contains(term))
}

fn transformer_term(value: &str) -> Option<&'static str> {
    [
        "transformers",
        "tokenizers",
        "sentencepiece",
        "llama",
        "bert",
        "gpt",
        "t5-model",
    ]
    .into_iter()
    .find(|term| value.contains(term))
}

fn remote_model_term(value: &str) -> Option<&'static str> {
    [
        "openai",
        "anthropic",
        "generativelanguage",
        "huggingface_hub",
        "model-api",
    ]
    .into_iter()
    .find(|term| value.contains(term))
}

fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn violation(
    code: ViolationCode,
    path: impl Into<String>,
    line: Option<usize>,
    message: impl Into<String>,
) -> Violation {
    Violation {
        code,
        path: path.into(),
        line,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{contains_token, is_arc_task_id, quoted_literals, strip_comments};

    #[test]
    fn comment_stripping_preserves_lines_and_strings() {
        let source = "// unsafe\nlet value = \"unsafe\"; /* unsafe\nunsafe */\n";
        let stripped = strip_comments(source);
        assert_eq!(stripped.lines().count(), source.lines().count());
        assert!(!contains_token(
            stripped.lines().next().unwrap_or_default(),
            "unsafe"
        ));
        assert_eq!(quoted_literals(&stripped), vec!["unsafe"]);
    }

    #[test]
    fn task_identifier_shape_is_exact() {
        let planted = ["dead", "beef"].concat();
        assert!(is_arc_task_id(&planted));
        assert!(!is_arc_task_id("ordinary-name"));
    }
}
