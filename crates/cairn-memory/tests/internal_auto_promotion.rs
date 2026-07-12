use cairn_consolidator::{
    Consolidator, InternalMiningLimits, ProofDatum, ProofGraph, ProofNode, ProofValueType,
};
use cairn_core::Hash32;
use cairn_memory::{
    InternalMechanismStore, InternalPromotionProgress, InternalStoreError, ModuleStatus,
};
use cairn_verifier::InternalMechanismVerifier;

#[test]
fn runtime_reuse_automatically_becomes_a_consolidator_certificate() {
    let sources = cohort(1);
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("mined internal structure");
    let mechanism = Consolidator
        .compile_internal_proof_mechanism(&proposal)
        .expect("source-independent mechanism");
    let compilation = InternalMechanismVerifier
        .verify_compilation(&proposal, &mechanism, &sources)
        .expect("independent compilation");
    let mut store = InternalMechanismStore::new();
    let id = store
        .insert_provisional(mechanism.clone(), compilation.clone())
        .expect("provisional module");
    assert_eq!(
        store.get(id).expect("module").status(),
        ModuleStatus::Provisional
    );

    let first_host = host_graph(31, 5, 3, "spiral", "violet");
    let first = reuse_receipt(&proposal, &mechanism, &compilation, &sources, &first_host);
    assert_eq!(
        store
            .record_runtime_reuse(id, first.clone())
            .expect("first receipt"),
        InternalPromotionProgress::Pending { verified_reuses: 1 }
    );
    assert_eq!(
        store.get(id).expect("module").status(),
        ModuleStatus::Provisional
    );

    let second_host = host_graph(32, 2, 2, "wave", "orange");
    let second = reuse_receipt(&proposal, &mechanism, &compilation, &sources, &second_host);
    let InternalPromotionProgress::Promoted(certificate) = store
        .record_runtime_reuse(id, second)
        .expect("automatic consolidation")
    else {
        panic!("second independent receipt must produce a certificate");
    };
    assert_eq!(certificate.mechanism(), id);
    assert_eq!(
        certificate.source_proposals(),
        &std::collections::BTreeSet::from([proposal.semantic_hash()])
    );
    assert_eq!(certificate.experiences().len(), 2);
    assert_eq!(certificate.verified_cases(), 10);
    let module = store.get(id).expect("persistent module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.verified_reuses(), 2);
    assert_eq!(
        store.record_runtime_reuse(id, first),
        Err(InternalStoreError::DuplicateExperience)
    );
    eprintln!(
        "AUTO_INTERNAL_PROMOTION mechanism={} source_proposal={} experiences=2 cases=10 candidate_windows=18->9/5 examples=4->1 proof_units=28->4",
        id,
        proposal.semantic_hash()
    );
}

#[test]
fn independently_verified_equivalent_cohorts_share_one_mechanism_safely() {
    let first_sources = cohort(1);
    let first_proposal = Consolidator
        .mine_internal_proof_subgraphs(&first_sources, limits())
        .expect("first proposal");
    let first_mechanism = Consolidator
        .compile_internal_proof_mechanism(&first_proposal)
        .expect("first mechanism");
    let first_compilation = InternalMechanismVerifier
        .verify_compilation(&first_proposal, &first_mechanism, &first_sources)
        .expect("first verification");
    let mut store = InternalMechanismStore::new();
    let id = store
        .insert_provisional(first_mechanism.clone(), first_compilation.clone())
        .expect("first module");

    let other_sources = cohort(11);
    let other_proposal = Consolidator
        .mine_internal_proof_subgraphs(&other_sources, limits())
        .expect("other proposal");
    let other_mechanism = Consolidator
        .compile_internal_proof_mechanism(&other_proposal)
        .expect("other mechanism");
    assert_eq!(id, other_mechanism.semantic_hash());
    assert_ne!(
        first_proposal.semantic_hash(),
        other_proposal.semantic_hash()
    );
    let other_compilation = InternalMechanismVerifier
        .verify_compilation(&other_proposal, &other_mechanism, &other_sources)
        .expect("other verification");
    let host = host_graph(41, 3, 2, "helix", "silver");
    let receipt = reuse_receipt(
        &other_proposal,
        &other_mechanism,
        &other_compilation,
        &other_sources,
        &host,
    );
    assert_eq!(
        store.record_runtime_reuse(id, receipt.clone()),
        Err(InternalStoreError::CompilationMismatch)
    );
    store
        .register_compilation(id, &other_mechanism, other_compilation)
        .expect("independent equivalent compilation registered");
    assert_eq!(store.get(id).expect("module").verified_compilations(), 2);
    assert_eq!(
        store
            .record_runtime_reuse(id, receipt)
            .expect("other-cohort receipt accepted after registration"),
        InternalPromotionProgress::Pending { verified_reuses: 1 }
    );
    let first_host = host_graph(42, 2, 3, "coil", "gold");
    let first_receipt = reuse_receipt(
        &first_proposal,
        &first_mechanism,
        &first_compilation,
        &first_sources,
        &first_host,
    );
    let InternalPromotionProgress::Promoted(certificate) = store
        .record_runtime_reuse(id, first_receipt)
        .expect("cross-cohort consolidation")
    else {
        panic!("two registered independent lineages must promote");
    };
    assert_eq!(certificate.source_proposals().len(), 2);
    assert_eq!(certificate.experiences().len(), 2);
    assert_eq!(
        store.get(id).expect("module").status(),
        ModuleStatus::Persistent
    );
}

fn reuse_receipt(
    proposal: &cairn_consolidator::InternalGraphCompressionProposal,
    mechanism: &cairn_consolidator::CompiledInternalMechanism,
    compilation: &cairn_verifier::VerifiedInternalMechanism,
    sources: &[ProofGraph],
    host: &ProofGraph,
) -> cairn_verifier::VerifiedInternalReuse {
    let binding = Consolidator
        .match_compiled_internal_mechanism(mechanism, host, 64)
        .expect("runtime match")
        .remove(0);
    InternalMechanismVerifier
        .verify_reuse(
            proposal,
            mechanism,
            compilation,
            sources,
            host,
            &binding,
            64,
        )
        .expect("verified runtime reuse")
}

fn cohort(start: u8) -> [ProofGraph; 4] {
    [
        host_graph(start, 1, 1, "triangle", "red"),
        host_graph(start + 1, 2, 2, "circle", "blue"),
        host_graph(start + 2, 3, 1, "segment", "green"),
        host_graph(start + 3, 1, 3, "polygon", "yellow"),
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
