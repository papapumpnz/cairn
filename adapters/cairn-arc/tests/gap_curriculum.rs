use cairn_arc::{lower_local_invention_problem, parse_task};
use cairn_curriculum::{FailureCoreExtractor, LearningFailureCore};
use cairn_invent::{DefinitionInventor, InventionLimits};
use cairn_rkl::ExecLimits;
use cairn_teacher_spool::{GapDrivenJobBuilder, OfflineTeacherOrchestrator, WireProtocol};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn collect_cores(
    directory: &Path,
    maximum_files: usize,
) -> (Vec<LearningFailureCore>, Vec<String>) {
    let mut paths = fs::read_dir(directory)
        .expect("corpus directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.truncate(maximum_files);
    let limits = InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 1_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    };
    let mut cores = BTreeSet::new();
    let mut task_names = Vec::new();
    for path in paths {
        let task = parse_task(&fs::read(&path).expect("task bytes")).expect("valid task");
        let Ok(problem) = lower_local_invention_problem(&task, limits) else {
            continue;
        };
        let Err(error) = DefinitionInventor.invent(&problem) else {
            continue;
        };
        let Ok(core) = FailureCoreExtractor.extract(&problem, &error) else {
            continue;
        };
        task_names.push(
            path.file_stem()
                .and_then(|name| name.to_str())
                .expect("task stem")
                .to_owned(),
        );
        cores.insert(core);
    }
    (cores.into_iter().collect(), task_names)
}

#[test]
fn official_failures_become_redacted_provider_neutral_curriculum_input() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let (cores, task_names) = collect_cores(&corpus, 40);
    assert!(cores.len() >= 3, "expected several distinct failure cores");
    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../docs/CAIRN_handoff/schemas/teacher-packet.schema.json"
    ))
    .expect("teacher schema");
    let job = GapDrivenJobBuilder
        .build(
            "generic-failure-curriculum",
            "offline-teacher",
            schema,
            &cores,
            16_000,
        )
        .expect("redacted job");
    let serialized_cores = serde_json::to_string_pretty(&cores).expect("serialize cores");
    for task_name in task_names {
        assert!(!serialized_cores.contains(&task_name));
        assert!(!job.curriculum_request.contains(&task_name));
    }
    for protocol in [WireProtocol::ResponsesSchema, WireProtocol::MessagesTool] {
        let request = OfflineTeacherOrchestrator
            .export(&job, protocol)
            .expect("offline spool");
        assert!(!request.request_bytes.is_empty());
    }
    eprintln!("REDACTED_FAILURE_CORES {serialized_cores}");
}

#[test]
#[ignore = "writes provider-shaped offline spool artifacts; run explicitly"]
fn write_offline_gap_curriculum_spools() {
    let corpus =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../datasets/arc-agi-1-v1.0.2/training");
    let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts/arc-agi-1-v1.0.2/distillation");
    let (cores, _) = collect_cores(&corpus, 40);
    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../docs/CAIRN_handoff/schemas/teacher-packet.schema.json"
    ))
    .expect("teacher schema");
    let job = GapDrivenJobBuilder
        .build(
            "generic-failure-curriculum",
            "offline-teacher",
            schema,
            &cores,
            16_000,
        )
        .expect("redacted job");
    fs::create_dir_all(&artifacts).expect("artifact directory");
    fs::write(
        artifacts.join("gap-driven-failure-cores-v1.json"),
        format!("{}\n", serde_json::to_string_pretty(&cores).expect("cores")),
    )
    .expect("failure cores");
    for (protocol, name) in [
        (WireProtocol::ResponsesSchema, "gpt-gap-request-v1.json"),
        (WireProtocol::MessagesTool, "anthropic-gap-request-v1.json"),
    ] {
        let request = OfflineTeacherOrchestrator
            .export(&job, protocol)
            .expect("offline spool");
        fs::write(artifacts.join(name), request.request_bytes).expect("request artifact");
    }
}
