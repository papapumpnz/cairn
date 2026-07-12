use cairn_cas::{Cas, CasLimits, PutStatus};
use cairn_consolidator::{
    Consolidator, InternalMiningLimits, ProofDatum, ProofGraph, ProofGraphArchive,
    ProofGraphArtifactMetadata, ProofNode, ProofValueType,
};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, Hash32, ProducerRecord,
    SchemaRef, SemanticsVersion,
};
use cairn_memory::{
    InternalMechanismStore, InternalPromotionProgress, InternalReuseArchiveError,
    InternalReuseArtifactArchive, InternalReuseReplay, ModuleStatus, ReuseArtifactMetadata,
    INTERNAL_REUSE_EVIDENCE_SCHEMA,
};
use cairn_verifier::InternalMechanismVerifier;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
#[allow(clippy::too_many_lines)] // One complete loss/restart/replay experiment.
fn canonical_evidence_reconstructs_and_repromotes_after_restart() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let sources = cohort();
    let held_out = [
        host_graph(31, 5, 3, "spiral", "violet"),
        host_graph(32, 2, 2, "wave", "orange"),
    ];
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("source proposal");
    let mechanism = Consolidator
        .compile_internal_proof_mechanism(&proposal)
        .expect("compiled mechanism");
    let compilation = InternalMechanismVerifier
        .verify_compilation(&proposal, &mechanism, &sources)
        .expect("verified compilation");
    let receipts = held_out
        .iter()
        .map(|host| {
            let binding = Consolidator
                .match_compiled_internal_mechanism(&mechanism, host, 64)
                .expect("runtime binding")
                .remove(0);
            InternalMechanismVerifier
                .verify_reuse(
                    &proposal,
                    &mechanism,
                    &compilation,
                    &sources,
                    host,
                    &binding,
                    64,
                )
                .expect("runtime receipt")
        })
        .collect::<Vec<_>>();
    let expected_mechanism = mechanism.semantic_hash();
    let expected_proposal = proposal.semantic_hash();

    let graph_archive = ProofGraphArchive;
    let all_graphs = sources.iter().chain(&held_out).cloned().collect::<Vec<_>>();
    let stored_graphs = all_graphs
        .iter()
        .enumerate()
        .map(|(index, graph)| {
            graph_archive
                .persist(
                    &cas,
                    graph,
                    proof_metadata(
                        parent(&cas, &format!("proof-parent-{index}")),
                        u64::try_from(index).expect("small") + 1,
                    ),
                )
                .expect("persist proof evidence")
        })
        .collect::<Vec<_>>();
    let source_parents = stored_graphs[..4]
        .iter()
        .map(|stored| stored.artifact_id)
        .collect::<Vec<_>>();
    let receipt_archive = InternalReuseArtifactArchive;
    let stored_receipts = receipts
        .iter()
        .enumerate()
        .map(|(index, receipt)| {
            let mut parents = source_parents.clone();
            parents.push(stored_graphs[4 + index].artifact_id);
            let metadata =
                reuse_metadata(parents.clone(), u64::try_from(index).expect("small") + 20);
            let first = receipt_archive
                .persist(&cas, receipt, metadata.clone())
                .expect("persist internal reuse");
            let duplicate = receipt_archive
                .persist(&cas, receipt, metadata)
                .expect("idempotent internal reuse");
            assert_eq!(first.artifact_id, duplicate.artifact_id);
            assert_eq!(first.envelope_blob_id, duplicate.envelope_blob_id);
            assert_eq!(duplicate.status, PutStatus::AlreadyPresent);
            let mut observed_differently = reuse_metadata(parents, 99_000);
            observed_differently.producer.host_run_id = "different-process".to_owned();
            observed_differently.budget.cpu_nanos = 777;
            let alternate = receipt_archive
                .persist(&cas, receipt, observed_differently)
                .expect("non-semantic metadata variation");
            assert_eq!(first.artifact_id, alternate.artifact_id);
            first
        })
        .collect::<Vec<_>>();

    drop(cas);
    drop(sources);
    drop(held_out);
    drop(proposal);
    drop(mechanism);
    drop(receipts);

    let reopened = Cas::open(&fixture.path, CasLimits::default()).expect("reopen CAS");
    reopened.recover().expect("recover CAS");
    let loaded_graphs = stored_graphs
        .iter()
        .map(|stored| {
            graph_archive
                .load(&reopened, stored.envelope_blob_id)
                .expect("reload proof graph")
        })
        .collect::<Vec<_>>();
    let restarted_sources = loaded_graphs[..4].to_vec();
    let restarted_hosts = loaded_graphs[4..].to_vec();
    let restarted_proposal = Consolidator
        .mine_internal_proof_subgraphs(&restarted_sources, limits())
        .expect("remine after restart");
    let restarted_mechanism = Consolidator
        .compile_internal_proof_mechanism(&restarted_proposal)
        .expect("recompile after restart");
    let restarted_compilation = InternalMechanismVerifier
        .verify_compilation(
            &restarted_proposal,
            &restarted_mechanism,
            &restarted_sources,
        )
        .expect("reverify compilation after restart");
    assert_eq!(restarted_proposal.semantic_hash(), expected_proposal);
    assert_eq!(restarted_mechanism.semantic_hash(), expected_mechanism);

    let loaded_receipts = stored_receipts
        .iter()
        .map(|stored| {
            receipt_archive
                .load(&reopened, stored.envelope_blob_id)
                .expect("load untrusted receipt")
        })
        .collect::<Vec<_>>();
    let replayed = loaded_receipts
        .iter()
        .zip(&restarted_hosts)
        .map(|(artifact, host)| {
            let binding = Consolidator
                .match_compiled_internal_mechanism(&restarted_mechanism, host, 64)
                .expect("rebind after restart")
                .remove(0);
            receipt_archive
                .replay(
                    *artifact,
                    InternalReuseReplay {
                        proposal: &restarted_proposal,
                        mechanism: &restarted_mechanism,
                        compilation: &restarted_compilation,
                        sources: &restarted_sources,
                        held_out: host,
                        binding: &binding,
                        maximum_match_windows: 64,
                    },
                )
                .expect("independent receipt replay")
        })
        .collect::<Vec<_>>();
    let mut restarted_store = InternalMechanismStore::new();
    let id = restarted_store
        .insert_provisional(restarted_mechanism, restarted_compilation)
        .expect("reconstructed provisional module");
    assert!(matches!(
        restarted_store
            .record_runtime_reuse(id, replayed[0].clone())
            .expect("first replayed receipt"),
        InternalPromotionProgress::Pending { verified_reuses: 1 }
    ));
    let InternalPromotionProgress::Promoted(certificate) = restarted_store
        .record_runtime_reuse(id, replayed[1].clone())
        .expect("restart automatic promotion")
    else {
        panic!("second replayed receipt must promote");
    };
    assert_eq!(certificate.mechanism(), expected_mechanism);
    assert_eq!(certificate.experiences().len(), 2);
    assert_eq!(
        restarted_store.get(id).expect("module").status(),
        ModuleStatus::Persistent
    );
    eprintln!(
        "PERSISTED_INTERNAL_REUSE mechanism={expected_mechanism} proposal={expected_proposal} proof_artifacts={},{},{},{},{},{} receipt_artifacts={},{} replayed=2 repromoted=1 candidates=18->9/5 examples=4->1 proof_units=28->4",
        stored_graphs[0].artifact_id,
        stored_graphs[1].artifact_id,
        stored_graphs[2].artifact_id,
        stored_graphs[3].artifact_id,
        stored_graphs[4].artifact_id,
        stored_graphs[5].artifact_id,
        stored_receipts[0].artifact_id,
        stored_receipts[1].artifact_id,
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Corruption and changed-evidence boundaries in one fixture.
fn malformed_or_changed_internal_reuse_fails_closed() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let sources = cohort();
    let host = host_graph(31, 5, 3, "spiral", "violet");
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("proposal");
    let mechanism = Consolidator
        .compile_internal_proof_mechanism(&proposal)
        .expect("mechanism");
    let compilation = InternalMechanismVerifier
        .verify_compilation(&proposal, &mechanism, &sources)
        .expect("compilation");
    let binding = Consolidator
        .match_compiled_internal_mechanism(&mechanism, &host, 64)
        .expect("binding")
        .remove(0);
    let receipt = InternalMechanismVerifier
        .verify_reuse(
            &proposal,
            &mechanism,
            &compilation,
            &sources,
            &host,
            &binding,
            64,
        )
        .expect("receipt");
    let archive = InternalReuseArtifactArchive;
    assert!(matches!(
        archive.persist(&cas, &receipt, reuse_metadata(Vec::new(), 1),),
        Err(InternalReuseArchiveError::MissingParent)
    ));
    let parent_id = parent(&cas, "valid-internal-reuse");
    let stored = archive
        .persist(&cas, &receipt, reuse_metadata(vec![parent_id], 2))
        .expect("persist valid receipt");
    let artifact = archive
        .load(&cas, stored.envelope_blob_id)
        .expect("load valid receipt");

    let changed_host = host_graph(41, 5, 3, "spiral", "violet");
    assert!(matches!(
        archive.replay(
            artifact,
            InternalReuseReplay {
                proposal: &proposal,
                mechanism: &mechanism,
                compilation: &compilation,
                sources: &sources,
                held_out: &changed_host,
                binding: &binding,
                maximum_match_windows: 64,
            }
        ),
        Err(InternalReuseArchiveError::IdentityMismatch)
    ));

    let mut changed_sources = sources.clone();
    changed_sources[0].nodes[2].arguments[0] = datum(ProofValueType::Relation, "overlaps");
    assert!(matches!(
        archive.replay(
            artifact,
            InternalReuseReplay {
                proposal: &proposal,
                mechanism: &mechanism,
                compilation: &compilation,
                sources: &changed_sources,
                held_out: &host,
                binding: &binding,
                maximum_match_windows: 64,
            }
        ),
        Err(InternalReuseArchiveError::VerificationFailure(_))
    ));

    let metrics = receipt.metrics();
    let changed_metrics = ArtifactEnvelope::new(
        SchemaRef::new(INTERNAL_REUSE_EVIDENCE_SCHEMA, 1).expect("schema"),
        ArtifactKind::new("memory.internal-reuse-evidence").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        vec![parent_id],
        Some(receipt.experience()),
        producer(),
        budget(),
        3,
        receipt_payload(&receipt, metrics.candidate_windows_without + 1),
    )
    .expect("changed metric envelope");
    let changed_metrics = cas
        .put_envelope(&changed_metrics)
        .expect("persist changed metric");
    let changed_metrics = archive
        .load(&cas, changed_metrics.envelope_blob_id)
        .expect("load changed metric claim");
    assert!(matches!(
        archive.replay(
            changed_metrics,
            InternalReuseReplay {
                proposal: &proposal,
                mechanism: &mechanism,
                compilation: &compilation,
                sources: &sources,
                held_out: &host,
                binding: &binding,
                maximum_match_windows: 64,
            }
        ),
        Err(InternalReuseArchiveError::ReplayMismatch)
    ));

    let malformed = ArtifactEnvelope::new(
        SchemaRef::new(INTERNAL_REUSE_EVIDENCE_SCHEMA, 1).expect("schema"),
        ArtifactKind::new("memory.internal-reuse-evidence").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        vec![parent_id],
        Some(receipt.experience()),
        producer(),
        budget(),
        4,
        CanonicalValue::Array(vec![CanonicalValue::Text("wrong-marker".to_owned())]),
    )
    .expect("structurally valid malformed envelope");
    let malformed = cas.put_envelope(&malformed).expect("persist malformed");
    assert!(matches!(
        archive.load(&cas, malformed.envelope_blob_id),
        Err(InternalReuseArchiveError::InvalidPayload(_))
    ));

    let wrong_scope = ArtifactEnvelope::new(
        SchemaRef::new(INTERNAL_REUSE_EVIDENCE_SCHEMA, 1).expect("schema"),
        ArtifactKind::new("memory.internal-reuse-evidence").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        vec![parent_id],
        Some(Hash32::digest(b"wrong-scope")),
        producer(),
        budget(),
        5,
        receipt_payload(&receipt, metrics.candidate_windows_without),
    )
    .expect("wrong scope envelope");
    let wrong_scope = cas.put_envelope(&wrong_scope).expect("persist wrong scope");
    assert!(matches!(
        archive.load(&cas, wrong_scope.envelope_blob_id),
        Err(InternalReuseArchiveError::InvalidEnvelope)
    ));

    for (schema_version, semantics) in [
        (2, SemanticsVersion::new(1, 0, 0)),
        (1, SemanticsVersion::new(2, 0, 0)),
    ] {
        let unsupported = ArtifactEnvelope::new(
            SchemaRef::new(INTERNAL_REUSE_EVIDENCE_SCHEMA, schema_version).expect("schema"),
            ArtifactKind::new("memory.internal-reuse-evidence").expect("kind"),
            semantics,
            vec![parent_id],
            Some(receipt.experience()),
            producer(),
            budget(),
            6,
            receipt_payload(&receipt, metrics.candidate_windows_without),
        )
        .expect("unsupported envelope remains structurally valid");
        let unsupported = cas
            .put_envelope(&unsupported)
            .expect("persist unsupported envelope");
        let rejected = archive.load(&cas, unsupported.envelope_blob_id);
        assert!(
            matches!(
                rejected,
                Err(InternalReuseArchiveError::Artifact(_) | InternalReuseArchiveError::Storage(_))
            ),
            "unsupported version must fail at schema/storage boundary: {rejected:?}"
        );
    }
}

fn receipt_payload(
    receipt: &cairn_verifier::VerifiedInternalReuse,
    candidate_windows_without: u64,
) -> CanonicalValue {
    let metrics = receipt.metrics();
    CanonicalValue::Array(vec![
        CanonicalValue::Text("cairn.memory.internal-reuse-evidence.v1".to_owned()),
        CanonicalValue::Bytes(receipt.mechanism().as_bytes().to_vec()),
        CanonicalValue::Bytes(receipt.source_proposal().as_bytes().to_vec()),
        CanonicalValue::Bytes(receipt.experience().as_bytes().to_vec()),
        CanonicalValue::Unsigned(receipt.verified_cases()),
        CanonicalValue::Unsigned(candidate_windows_without),
        CanonicalValue::Unsigned(metrics.candidate_windows_with),
        CanonicalValue::Unsigned(u64::from(metrics.examples_without)),
        CanonicalValue::Unsigned(u64::from(metrics.examples_with)),
        CanonicalValue::Unsigned(metrics.proof_units_without),
        CanonicalValue::Unsigned(metrics.proof_units_with),
    ])
}

fn cohort() -> [ProofGraph; 4] {
    [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ]
}

const fn limits() -> InternalMiningLimits {
    InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    }
}

fn host_graph(
    episode: u8,
    prefix_nodes: usize,
    suffix_nodes: usize,
    role: &str,
    effect: &str,
) -> ProofGraph {
    let mut nodes = vec![ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, &format!("world-{episode}"))],
    }];
    for index in 1..prefix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-prefix-{index}"),
            output_type: ProofValueType::Integer,
            inputs: vec![],
            arguments: vec![datum(ProofValueType::Integer, &index.to_string())],
        });
    }
    let start = nodes.len();
    nodes.extend([
        ProofNode {
            operation: "select-role".to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![0],
            arguments: vec![datum(ProofValueType::Atom, role)],
        },
        ProofNode {
            operation: "filter-relation".to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![u32::try_from(start).expect("small")],
            arguments: vec![datum(ProofValueType::Relation, "inside")],
        },
        ProofNode {
            operation: "map-effect".to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 1).expect("small")],
            arguments: vec![datum(ProofValueType::Atom, effect)],
        },
        ProofNode {
            operation: "render-effect".to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 2).expect("small")],
            arguments: vec![],
        },
    ]);
    let mut previous = start + 3;
    for index in 0..suffix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-suffix-{episode}-{index}"),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(previous).expect("small")],
            arguments: vec![],
        });
        previous += 1;
    }
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        output: u32::try_from(nodes.len() - 1).expect("small"),
        nodes,
    }
}

fn datum(ty: ProofValueType, value: &str) -> ProofDatum {
    ProofDatum::digest(ty, value.as_bytes())
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.internal-reuse-parent", 1).expect("schema"),
        ArtifactKind::new("test.internal-reuse-parent").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        Vec::new(),
        None,
        producer(),
        budget(),
        0,
        CanonicalValue::Text(name.to_owned()),
    )
    .expect("parent envelope");
    cas.put_envelope(&envelope)
        .expect("persist parent")
        .artifact_id
}

fn proof_metadata(
    parent_id: cairn_core::ArtifactId,
    created_at_logical: u64,
) -> ProofGraphArtifactMetadata {
    ProofGraphArtifactMetadata {
        parent_ids: vec![parent_id],
        producer: producer(),
        budget: budget(),
        created_at_logical,
    }
}

fn reuse_metadata(
    parent_ids: Vec<cairn_core::ArtifactId>,
    created_at_logical: u64,
) -> ReuseArtifactMetadata {
    ReuseArtifactMetadata {
        parent_ids,
        producer: producer(),
        budget: budget(),
        created_at_logical,
    }
}

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "internal-reuse-persistence-test".to_owned(),
        version: "0.1.0".to_owned(),
        commit: "test".to_owned(),
        host_run_id: "restart".to_owned(),
        agent_id: Some("memory".to_owned()),
    }
}

const fn budget() -> BudgetRecord {
    BudgetRecord {
        work_units: 64,
        cpu_nanos: 0,
        peak_rss_bytes: 0,
    }
}

struct TemporaryStore {
    path: PathBuf,
}

impl TemporaryStore {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        Self {
            path: std::env::temp_dir().join(format!(
                "cairn-persisted-internal-reuse-{}-{sequence}",
                std::process::id()
            )),
        }
    }
}

impl Drop for TemporaryStore {
    fn drop(&mut self) {
        let _ignored = std::fs::remove_dir_all(&self.path);
    }
}
