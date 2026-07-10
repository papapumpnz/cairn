use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// A runtime-boundary record for CPU-only benchmark evidence.
///
/// Collections are sorted before serialization. Volatile observation fields
/// are explicitly non-semantic and this record is not used as an artifact ID.
#[derive(Clone, Debug, Serialize)]
pub struct RuntimeAttestation {
    pub schema_version: u32,
    pub observed_unix_seconds: u64,
    pub operating_system: String,
    pub kernel_release: Option<String>,
    pub cpu_model: Option<String>,
    pub physical_cpu_count: Option<usize>,
    pub logical_cpu_count: usize,
    pub memory_total_bytes: Option<u64>,
    pub process: ProcessRecord,
    pub loaded_native_libraries: Vec<String>,
    pub prohibited_runtime_findings: Vec<RuntimeFinding>,
    pub network: NetworkRecord,
    pub accelerator_environment: BTreeMap<String, String>,
    pub build: BuildRecord,
}

/// Stable runtime policy finding emitted directly into the attestation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RuntimeFinding {
    pub code: RuntimeFindingCode,
    pub evidence: String,
}

/// Runtime exclusion rule that fired while collecting an attestation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeFindingCode {
    AcceleratorLibraryLoaded,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProcessRecord {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub executable: Option<String>,
    pub tree: Vec<ProcessTreeEntry>,
}

/// One bounded process-tree entry, without command arguments or environment.
#[derive(Clone, Debug, Serialize)]
pub struct ProcessTreeEntry {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: Option<String>,
    pub executable: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkRecord {
    pub tcp_socket_entries: Option<usize>,
    pub udp_socket_entries: Option<usize>,
    pub interfaces: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BuildRecord {
    pub git_commit: Option<String>,
    pub cargo_lock_blake3: Option<String>,
    pub rustc_version: Option<String>,
}

/// Typed attestation collection or persistence failure.
#[derive(Debug)]
pub enum AttestError {
    Io { path: PathBuf, source: io::Error },
    Serialize(serde_json::Error),
    ClockBeforeEpoch,
}

impl fmt::Display for AttestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot access {}: {source}", path.display()),
            Self::Serialize(source) => write!(f, "cannot serialize attestation: {source}"),
            Self::ClockBeforeEpoch => f.write_str("system clock is before the Unix epoch"),
        }
    }
}

impl std::error::Error for AttestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Serialize(source) => Some(source),
            Self::ClockBeforeEpoch => None,
        }
    }
}

/// Collect the locally observable runtime boundary.
///
/// Missing optional operating-system facilities are represented as absent
/// fields rather than guessed values. Reading the repository is bounded by the
/// lockfile size; process ancestry is capped at 256 links.
///
/// # Errors
///
/// Returns [`AttestError::ClockBeforeEpoch`] when the system clock cannot be
/// represented in the schema.
pub fn collect_attestation(repository: &Path) -> Result<RuntimeAttestation, AttestError> {
    let observed_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AttestError::ClockBeforeEpoch)?
        .as_secs();
    let process = collect_process();
    let mut loaded_native_libraries = loaded_libraries();
    loaded_native_libraries.sort();
    loaded_native_libraries.dedup();
    let prohibited_runtime_findings = classify_loaded_libraries(&loaded_native_libraries);
    let mut interfaces = directory_names(Path::new("/sys/class/net"));
    interfaces.sort();

    Ok(RuntimeAttestation {
        schema_version: 1,
        observed_unix_seconds,
        operating_system: env::consts::OS.to_owned(),
        kernel_release: command_line("uname", &["-sr"]),
        cpu_model: cpu_model(),
        physical_cpu_count: physical_cpu_count(),
        logical_cpu_count: std::thread::available_parallelism().map_or(1, usize::from),
        memory_total_bytes: memory_total_bytes(),
        process,
        loaded_native_libraries,
        prohibited_runtime_findings,
        network: NetworkRecord {
            tcp_socket_entries: socket_entries(Path::new("/proc/net/tcp")),
            udp_socket_entries: socket_entries(Path::new("/proc/net/udp")),
            interfaces,
        },
        accelerator_environment: accelerator_environment(),
        build: BuildRecord {
            git_commit: git_commit(repository),
            cargo_lock_blake3: lockfile_hash(repository),
            rustc_version: command_line("rustc", &["--version"]),
        },
    })
}

fn classify_loaded_libraries(libraries: &[String]) -> Vec<RuntimeFinding> {
    let mut findings = libraries
        .iter()
        .filter(|library| is_prohibited_accelerator_library(library))
        .map(|library| RuntimeFinding {
            code: RuntimeFindingCode::AcceleratorLibraryLoaded,
            evidence: library.clone(),
        })
        .collect::<Vec<_>>();
    findings.sort();
    findings.dedup();
    findings
}

fn is_prohibited_accelerator_library(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        "libcuda",
        "libcudart",
        "libcublas",
        "libcudnn",
        "libamdhip",
        "librocm",
        "libopencl",
        "libvulkan",
        "libtensorrt",
    ]
    .into_iter()
    .any(|name| lower.contains(name))
}

/// Write a pretty, stable JSON attestation, replacing only the requested file.
///
/// # Errors
///
/// Returns a typed serialization or filesystem error without leaving an
/// accepted attestation record in memory.
pub fn write_attestation(path: &Path, attestation: &RuntimeAttestation) -> Result<(), AttestError> {
    let bytes = serde_json::to_vec_pretty(attestation).map_err(AttestError::Serialize)?;
    fs::write(path, bytes).map_err(|source| AttestError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn collect_process() -> ProcessRecord {
    let pid = std::process::id();
    let parent_pid = process_parent(pid);
    let executable = env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().into_owned());
    let mut tree = vec![process_tree_entry(pid)];
    let mut current = parent_pid;
    while let Some(value) = current {
        if value == 0 || tree.len() == 256 || tree.iter().any(|entry| entry.pid == value) {
            break;
        }
        tree.push(process_tree_entry(value));
        current = process_parent(value);
    }
    ProcessRecord {
        pid,
        parent_pid,
        executable,
        tree,
    }
}

fn process_tree_entry(pid: u32) -> ProcessTreeEntry {
    ProcessTreeEntry {
        pid,
        parent_pid: process_parent(pid),
        name: fs::read_to_string(format!("/proc/{pid}/comm"))
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
        executable: fs::read_link(format!("/proc/{pid}/exe"))
            .ok()
            .map(|path| path.to_string_lossy().into_owned()),
    }
}

fn process_parent(pid: u32) -> Option<u32> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("PPid:")
            .and_then(|value| value.trim().parse().ok())
    })
}

fn loaded_libraries() -> Vec<String> {
    let Some(maps) = fs::read_to_string("/proc/self/maps").ok() else {
        return Vec::new();
    };
    maps.lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|entry| entry.starts_with('/') && entry.contains(".so"))
        .map(str::to_owned)
        .collect()
}

fn cpu_model() -> Option<String> {
    let cpu_info = fs::read_to_string("/proc/cpuinfo").ok()?;
    cpu_info.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        matches!(key.trim(), "model name" | "Hardware").then(|| value.trim().to_owned())
    })
}

fn physical_cpu_count() -> Option<usize> {
    let cpu_info = fs::read_to_string("/proc/cpuinfo").ok()?;
    let mut cores = std::collections::BTreeSet::new();
    let mut physical_id = None;
    let mut core_id = None;
    for line in cpu_info.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let (Some(physical), Some(core)) = (physical_id.take(), core_id.take()) {
                cores.insert((physical, core));
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "physical id" => physical_id = value.trim().parse::<u32>().ok(),
            "core id" => core_id = value.trim().parse::<u32>().ok(),
            _ => {}
        }
    }
    (!cores.is_empty()).then_some(cores.len())
}

fn memory_total_bytes() -> Option<u64> {
    let memory_info = fs::read_to_string("/proc/meminfo").ok()?;
    let kibibytes = memory_info.lines().find_map(|line| {
        let value = line.strip_prefix("MemTotal:")?.split_whitespace().next()?;
        value.parse::<u64>().ok()
    })?;
    kibibytes.checked_mul(1024)
}

fn socket_entries(path: &Path) -> Option<usize> {
    let content = fs::read_to_string(path).ok()?;
    Some(content.lines().count().saturating_sub(1))
}

fn directory_names(path: &Path) -> Vec<String> {
    let Some(entries) = fs::read_dir(path).ok() else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

fn accelerator_environment() -> BTreeMap<String, String> {
    let keys = [
        "CUDA_VISIBLE_DEVICES",
        "HIP_VISIBLE_DEVICES",
        "ROCR_VISIBLE_DEVICES",
        "GPU_DEVICE_ORDINAL",
        "OPENCL_VENDOR_PATH",
    ];
    keys.into_iter()
        .filter_map(|key| env::var(key).ok().map(|value| (key.to_owned(), value)))
        .collect()
}

fn git_commit(repository: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repository)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn lockfile_hash(repository: &Path) -> Option<String> {
    let bytes = fs::read(repository.join("Cargo.lock")).ok()?;
    Some(blake3::hash(&bytes).to_hex().to_string())
}

fn command_line(program: &str, arguments: &[&str]) -> Option<String> {
    Command::new(program)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{classify_loaded_libraries, collect_attestation, RuntimeFindingCode};

    #[test]
    fn attestation_is_valid_json_with_required_boundary_fields() {
        let attestation = collect_attestation(std::path::Path::new(".")).expect("collect");
        let value = serde_json::to_value(attestation).expect("serialize");
        assert_eq!(value["schema_version"], 1);
        assert!(value["logical_cpu_count"]
            .as_u64()
            .is_some_and(|count| count > 0));
        assert!(value["process"]["pid"].as_u64().is_some());
        assert!(value["process"]["tree"]
            .as_array()
            .is_some_and(|tree| !tree.is_empty()));
        assert!(value["network"]["interfaces"].is_array());
    }

    #[test]
    fn planted_loaded_accelerator_library_is_reported() {
        let planted = ["/usr/lib/lib", "cuda.so.1"].concat();
        let findings = classify_loaded_libraries(&[planted.clone(), planted]);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].code,
            RuntimeFindingCode::AcceleratorLibraryLoaded
        );
    }
}
