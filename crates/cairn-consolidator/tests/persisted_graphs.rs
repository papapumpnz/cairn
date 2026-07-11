use cairn_cas::{Cas, CasLimits, PutStatus};
use cairn_consolidator::{
    ConsolidationError, ConsolidationLimits, ConsolidationRuntime, ProofDatum, ProofGraph,
    ProofGraphArchive, ProofGraphArchiveError, ProofGraphArtifactMetadata, ProofNode,
    ProofValueType, PROOF_GRAPH_SCHEMA,
};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalEncode, CanonicalLimits, CanonicalValue,
    Hash32, ProducerRecord, SchemaRef, SemanticsVersion,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn consolidation_restarts_from_canonical_proof_graphs() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let graphs = [
        graph(1, "triangle", "red"),
        graph(2, "circle", "blue"),
        graph(3, "segment", "green"),
    ];
    for graph in &graphs {
        let bytes = graph.encode_canonical().expect("encode graph");
        assert_eq!(
            ProofGraph::decode_canonical(&bytes, CanonicalLimits::default()).expect("decode graph"),
            *graph
        );
    }
    let fresh = consolidate(graphs.clone());
    let archive = ProofGraphArchive;
    let stored = graphs
        .iter()
        .enumerate()
        .map(|(index, graph)| {
            let metadata = metadata(
                parent(&cas, &format!("source-{index}")),
                u64::try_from(index).expect("small") + 1,
            );
            let first = archive
                .persist(&cas, graph, metadata.clone())
                .expect("persist proof graph");
            let duplicate = archive
                .persist(&cas, graph, metadata)
                .expect("idempotent proof graph");
            assert_eq!(first.artifact_id, duplicate.artifact_id);
            assert_eq!(first.envelope_blob_id, duplicate.envelope_blob_id);
            assert_eq!(duplicate.status, PutStatus::AlreadyPresent);
            first
        })
        .collect::<Vec<_>>();
    drop(cas);

    let reopened = Cas::open(&fixture.path, CasLimits::default()).expect("reopen CAS");
    reopened.recover().expect("recover CAS");
    let loaded = stored
        .iter()
        .map(|stored| {
            archive
                .load(&reopened, stored.envelope_blob_id)
                .expect("load graph")
        })
        .collect::<Vec<_>>();
    let restarted = consolidate(loaded.clone().try_into().expect("three graphs"));
    assert_eq!(fresh.semantic_hash(), restarted.semantic_hash());
    assert_eq!(fresh.verified_replays(), 3);
    assert_eq!(restarted.verified_replays(), 3);
    assert_eq!(fresh.source_episodes(), restarted.source_episodes());
    eprintln!(
        "PERSISTED_PROOF_GRAPHS artifacts={},{},{} proposal={} replays=3 restart_equal=1",
        stored[0].artifact_id,
        stored[1].artifact_id,
        stored[2].artifact_id,
        restarted.semantic_hash()
    );
}

#[test]
fn invalid_or_malformed_persisted_graphs_fail_closed() {
    let fixture = TemporaryStore::new();
    let cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let archive = ProofGraphArchive;
    let valid = graph(1, "triangle", "red");
    assert!(matches!(
        archive.persist(
            &cas,
            &valid,
            ProofGraphArtifactMetadata {
                parent_ids: Vec::new(),
                ..metadata(parent(&cas, "unused"), 0)
            }
        ),
        Err(ProofGraphArchiveError::MissingParent)
    ));
    let mut cyclic = valid.clone();
    cyclic.nodes[1].inputs = vec![1];
    assert!(matches!(
        archive.persist(&cas, &cyclic, metadata(parent(&cas, "cyclic"), 1)),
        Err(ProofGraphArchiveError::InvalidGraph(
            ConsolidationError::InvalidTopology
        ))
    ));

    let malformed = ArtifactEnvelope::new(
        SchemaRef::new(PROOF_GRAPH_SCHEMA, 1).expect("schema"),
        ArtifactKind::new("consolidator.proof-graph").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        vec![parent(&cas, "malformed")],
        Some(Hash32::digest(b"malformed-episode")),
        producer(),
        budget(),
        2,
        CanonicalValue::Array(vec![CanonicalValue::Text("wrong-marker".to_owned())]),
    )
    .expect("structurally valid envelope");
    let malformed = cas.put_envelope(&malformed).expect("store malformed");
    assert!(matches!(
        archive.load(&cas, malformed.envelope_blob_id),
        Err(ProofGraphArchiveError::Decode(_))
    ));
}

fn consolidate(graphs: [ProofGraph; 3]) -> cairn_consolidator::GraphCompressionProposal {
    let mut runtime = ConsolidationRuntime::new(ConsolidationLimits {
        maximum_episodes: 8,
        maximum_nodes_per_episode: 8,
        minimum_independent_episodes: 3,
    });
    for graph in graphs {
        runtime.submit(graph).expect("submit graph");
    }
    let report = runtime.consolidate().expect("consolidate graphs");
    assert_eq!(report.metrics.proposals_created, 1);
    report.proposals.into_iter().next().expect("proposal")
}

fn graph(episode: u8, role: &str, color: &str) -> ProofGraph {
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        nodes: vec![
            ProofNode {
                operation: "observe-entities".to_owned(),
                output_type: ProofValueType::EntitySet,
                inputs: vec![],
                arguments: vec![datum(ProofValueType::Atom, role)],
            },
            ProofNode {
                operation: "filter-by-relation".to_owned(),
                output_type: ProofValueType::EntitySet,
                inputs: vec![0],
                arguments: vec![datum(ProofValueType::Relation, "inside")],
            },
            ProofNode {
                operation: "map-effect".to_owned(),
                output_type: ProofValueType::Effect,
                inputs: vec![1],
                arguments: vec![datum(ProofValueType::Atom, color)],
            },
            ProofNode {
                operation: "render".to_owned(),
                output_type: ProofValueType::Effect,
                inputs: vec![2],
                arguments: vec![],
            },
        ],
        output: 3,
    }
}

fn datum(ty: ProofValueType, value: &str) -> ProofDatum {
    ProofDatum::digest(ty, value.as_bytes())
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.proof-source", 1).expect("schema"),
        ArtifactKind::new("test.proof-source").expect("kind"),
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

fn metadata(
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

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "proof-graph-persistence-test".to_owned(),
        version: "0.1.0".to_owned(),
        commit: "test".to_owned(),
        host_run_id: "restart".to_owned(),
        agent_id: Some("consolidator".to_owned()),
    }
}

const fn budget() -> BudgetRecord {
    BudgetRecord {
        work_units: 4,
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
                "cairn-persisted-proof-{}-{sequence}",
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
