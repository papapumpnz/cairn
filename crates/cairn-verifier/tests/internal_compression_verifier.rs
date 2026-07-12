use cairn_consolidator::{
    Consolidator, InternalMiningLimits, ProofDatum, ProofGraph, ProofNode, ProofValueType,
};
use cairn_core::Hash32;
use cairn_verifier::{
    InternalCompressionVerifier, InternalCompressionVerifyError, InternalMechanismVerifier,
    InternalPromotionConsolidator,
};

#[test]
fn independently_reconstructs_internal_boundaries_costs_and_identity() {
    let sources = sources();
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("producer proposal");
    let verified = InternalCompressionVerifier
        .verify(&proposal, &sources)
        .expect("independent internal verification");
    assert_eq!(verified.proposal(), proposal.semantic_hash());
    assert_eq!(verified.replayed_sources(), 4);
    assert_eq!(verified.uncompressed_nodes(), 28);
    assert_eq!(verified.compressed_nodes(), 20);
    assert_eq!(verified.source_episodes().len(), 4);

    let reversed = sources.iter().cloned().rev().collect::<Vec<_>>();
    assert_eq!(
        InternalCompressionVerifier
            .verify(&proposal, &reversed)
            .expect("source order does not matter")
            .proposal(),
        proposal.semantic_hash()
    );
}

#[test]
fn changed_internal_content_identity_or_boundary_fails_closed() {
    let sources = sources();
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("producer proposal");

    let mut changed_content = sources.clone();
    changed_content[1].nodes[4].arguments[0] = datum(ProofValueType::Atom, "orange");
    assert_eq!(
        InternalCompressionVerifier.verify(&proposal, &changed_content),
        Err(InternalCompressionVerifyError::ReplayMismatch)
    );

    let mut changed_identity = sources.clone();
    changed_identity[1].episode = Hash32::digest(b"changed-episode");
    assert_eq!(
        InternalCompressionVerifier.verify(&proposal, &changed_identity),
        Err(InternalCompressionVerifyError::SourceMismatch)
    );

    let mut changed_boundary = sources.clone();
    changed_boundary[1].nodes[2].inputs = vec![1];
    assert_eq!(
        InternalCompressionVerifier.verify(&proposal, &changed_boundary),
        Err(InternalCompressionVerifyError::BoundaryMismatch)
    );
}

#[test]
fn independently_verifies_source_independent_compilation_and_new_invocation() {
    let sources = sources();
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("producer proposal");
    let mechanism = Consolidator
        .compile_internal_proof_mechanism(&proposal)
        .expect("producer compilation");
    let verified = InternalMechanismVerifier
        .verify_compilation(&proposal, &mechanism, &sources)
        .expect("independent compilation replay");
    assert_eq!(verified.mechanism(), mechanism.semantic_hash());
    assert_eq!(verified.source_proposal(), proposal.semantic_hash());
    assert_eq!(verified.replayed_sources(), 4);

    let held_out = host_graph(21, 5, 3, "spiral", "violet");
    let binding = Consolidator
        .match_compiled_internal_mechanism(&mechanism, &held_out, 64)
        .expect("producer host binding")
        .remove(0);
    let invocation = InternalMechanismVerifier
        .verify_invocation(&mechanism, &held_out, &binding)
        .expect("independent held-out invocation");
    assert_eq!(invocation.mechanism(), mechanism.semantic_hash());
    assert_eq!(invocation.episode(), held_out.episode);
    assert_eq!(invocation.selected_nodes(), [5, 6, 7, 8]);

    let other_sources = [
        host_graph(11, 4, 1, "arc", "cyan"),
        host_graph(12, 2, 4, "box", "magenta"),
        host_graph(13, 3, 2, "line", "amber"),
        host_graph(14, 1, 5, "point", "white"),
    ];
    let other_proposal = Consolidator
        .mine_internal_proof_subgraphs(&other_sources, limits())
        .expect("other evidence proposal");
    let other_mechanism = Consolidator
        .compile_internal_proof_mechanism(&other_proposal)
        .expect("other compiled schema");
    assert_eq!(mechanism.semantic_hash(), other_mechanism.semantic_hash());
    assert_eq!(
        InternalMechanismVerifier.verify_compilation(&other_proposal, &mechanism, &other_sources),
        Err(InternalCompressionVerifyError::CompiledLineageMismatch)
    );

    let mut wrong_episode = binding.clone();
    wrong_episode.episode = Hash32::digest(b"wrong-host");
    assert_eq!(
        InternalMechanismVerifier.verify_invocation(&mechanism, &held_out, &wrong_episode),
        Err(InternalCompressionVerifyError::SourceMismatch)
    );
    let mut wrong_type = binding;
    wrong_type.values[0] = datum(ProofValueType::Integer, "7");
    assert_eq!(
        InternalMechanismVerifier.verify_invocation(&mechanism, &held_out, &wrong_type),
        Err(InternalCompressionVerifyError::MissingBinding)
    );
}

#[test]
fn verifier_derives_reuse_work_and_consolidator_issues_certificate() {
    let sources = sources();
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits())
        .expect("producer proposal");
    let mechanism = Consolidator
        .compile_internal_proof_mechanism(&proposal)
        .expect("compiled mechanism");
    let compilation = InternalMechanismVerifier
        .verify_compilation(&proposal, &mechanism, &sources)
        .expect("verified compilation");

    let first_host = host_graph(31, 5, 3, "spiral", "violet");
    let first_binding = Consolidator
        .match_compiled_internal_mechanism(&mechanism, &first_host, 64)
        .expect("first runtime binding")
        .remove(0);
    let first = InternalMechanismVerifier
        .verify_reuse(
            &proposal,
            &mechanism,
            &compilation,
            &sources,
            &first_host,
            &first_binding,
            64,
        )
        .expect("first measured reuse");
    assert_eq!(first.metrics().candidate_windows_without, 18);
    assert_eq!(first.metrics().candidate_windows_with, 9);
    assert_eq!(first.metrics().examples_without, 4);
    assert_eq!(first.metrics().examples_with, 1);
    assert_eq!(first.metrics().proof_units_without, 28);
    assert_eq!(first.metrics().proof_units_with, 4);
    assert_eq!(
        InternalPromotionConsolidator.consolidate(
            &mechanism,
            std::slice::from_ref(&compilation),
            std::slice::from_ref(&first)
        ),
        Err(InternalCompressionVerifyError::InsufficientReuseEvidence)
    );

    let second_host = host_graph(32, 2, 2, "wave", "orange");
    let second_binding = Consolidator
        .match_compiled_internal_mechanism(&mechanism, &second_host, 64)
        .expect("second runtime binding")
        .remove(0);
    let second = InternalMechanismVerifier
        .verify_reuse(
            &proposal,
            &mechanism,
            &compilation,
            &sources,
            &second_host,
            &second_binding,
            64,
        )
        .expect("second measured reuse");
    let certificate = InternalPromotionConsolidator
        .consolidate(
            &mechanism,
            std::slice::from_ref(&compilation),
            &[first.clone(), second.clone()],
        )
        .expect("consolidator certificate");
    assert_eq!(certificate.mechanism(), mechanism.semantic_hash());
    assert_eq!(
        certificate.source_proposals(),
        &std::collections::BTreeSet::from([proposal.semantic_hash()])
    );
    assert_eq!(certificate.experiences().len(), 2);
    assert_eq!(certificate.verified_cases(), 10);
    assert_eq!(
        InternalPromotionConsolidator.consolidate(
            &mechanism,
            std::slice::from_ref(&compilation),
            &[first.clone(), first]
        ),
        Err(InternalCompressionVerifyError::InsufficientReuseEvidence)
    );
}

fn sources() -> [ProofGraph; 4] {
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
