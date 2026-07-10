use cairn_check::{check_repository, CheckOptions, ViolationCode};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("cairn-check-{}-{sequence}", std::process::id()));
        fs::create_dir_all(root.join("src")).expect("create fixture source");
        fs::create_dir_all(root.join("policy")).expect("create fixture policy");
        write(
            &root.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        );
        write(
            &root.join("src/lib.rs"),
            "pub fn clean() -> bool { true }\n",
        );
        write(
            &root.join("policy/dependency-layers.toml"),
            "schema_version = 1\nlayers = [\"governance\", \"substrate\"]\n[crates.fixture]\nlayer = \"governance\"\nallow_unsafe_modules = []\n",
        );
        Self { root }
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        write(&path, content);
    }

    fn codes(&self) -> BTreeSet<ViolationCode> {
        check_repository(&CheckOptions::new(&self.root))
            .expect("checker completes")
            .violations
            .into_iter()
            .map(|violation| violation.code)
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, content: &str) {
    fs::write(path, content).expect("write fixture");
}

#[test]
fn clean_fixture_passes() {
    assert!(Fixture::new().codes().is_empty());
}

#[test]
fn each_source_boundary_has_a_stable_code() {
    let cases = [
        (
            "pub fn run() { let _backend = \"cuda\"; }\n",
            ViolationCode::AcceleratorSource,
        ),
        (
            "pub fn run() { let _runtime = \"transformers\"; }\n",
            ViolationCode::TransformerSource,
        ),
        (
            "pub fn run() { let _endpoint = \"https://api.openai.example/v1\"; }\n",
            ViolationCode::RemoteModelApi,
        ),
        (
            "pub fn run() { let _task = \"deadbeef\"; }\n",
            ViolationCode::ArcTaskIdLiteral,
        ),
        (
            "pub fn run() { let _root = \"private/evaluation/tasks\"; }\n",
            ViolationCode::EvaluationPath,
        ),
        (
            "pub fn run() { let _protocol = \"cairn-gossip/0\"; }\n",
            ViolationCode::ProtocolDowngrade,
        ),
        (
            "pub fn run() { unsafe { std::hint::unreachable_unchecked() } }\n",
            ViolationCode::UnsafeOutsideAllowlist,
        ),
    ];
    for (source, expected) in cases {
        let fixture = Fixture::new();
        fixture.write("src/lib.rs", source);
        assert!(fixture.codes().contains(&expected), "missing {expected:?}");
    }
}

#[test]
fn generated_source_is_checked() {
    let fixture = Fixture::new();
    fixture.write(
        "generated/bindings.rs",
        "pub const TASK: &str = \"cafebabe\";\n",
    );
    assert!(fixture.codes().contains(&ViolationCode::ArcTaskIdLiteral));
}

#[test]
fn native_link_and_activated_feature_are_checked() {
    let fixture = Fixture::new();
    fixture.write(
        "build.rs",
        "fn main() { println!(\"cargo:rustc-link-lib=cudart\"); }\n",
    );
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n\n[workspace]\n\n[features]\ndefault = [\"cuda\"]\ncuda = []\n",
    );
    let codes = fixture.codes();
    assert!(codes.contains(&ViolationCode::NativeAcceleratorLink));
    assert!(codes.contains(&ViolationCode::AcceleratorFeature));
}

#[test]
fn transitive_lockfile_package_is_checked_without_source_import() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.lock",
        "version = 3\n\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"cuda-driver\"\nversion = \"1.0.0\"\n",
    );
    assert!(fixture
        .codes()
        .contains(&ViolationCode::AcceleratorDependency));
}

#[test]
fn transformer_and_remote_model_dependencies_are_checked() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.lock",
        "version = 3\n\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"transformers\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"async-openai\"\nversion = \"1.0.0\"\n",
    );
    let codes = fixture.codes();
    assert!(codes.contains(&ViolationCode::TransformerDependency));
    assert!(codes.contains(&ViolationCode::RemoteModelApi));
}

#[test]
fn hidden_evaluation_configuration_is_checked() {
    let fixture = Fixture::new();
    fixture.write(
        "config/runtime.toml",
        "dataset_root = \"private/evaluation/tasks\"\n",
    );
    assert!(fixture.codes().contains(&ViolationCode::EvaluationPath));
}

#[test]
fn later_architecture_layer_cannot_flow_into_an_earlier_layer() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"early\", \"late\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "policy/dependency-layers.toml",
        "schema_version = 1\nlayers = [\"governance\", \"substrate\"]\n[crates.early]\nlayer = \"governance\"\n[crates.late]\nlayer = \"substrate\"\n",
    );
    fixture.write(
        "early/Cargo.toml",
        "[package]\nname = \"early\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\nlate = { path = \"../late\" }\n",
    );
    fixture.write("early/src/lib.rs", "pub fn early() {}\n");
    fixture.write(
        "late/Cargo.toml",
        "[package]\nname = \"late\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("late/src/lib.rs", "pub fn late() {}\n");
    assert!(fixture
        .codes()
        .contains(&ViolationCode::ForbiddenDependencyDirection));
}
