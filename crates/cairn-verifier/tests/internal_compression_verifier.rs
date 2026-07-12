use cairn_consolidator::{
    Consolidator, InternalConstructionLimits, InternalMiningLimits, ObservationFrontierLimits,
    ProofDatum, ProofFrontier, ProofGraph, ProofNode, ProofValueType,
};
use cairn_core::{
    CanonicalValue, EvidenceElement, EvidenceElementId, EvidenceRelation, EvidenceValueType,
    Hash32, ModalityId, ObservationCertainty, RelationalObservation, RelationalObservationLimits,
    SourceReference,
};
use cairn_verifier::{
    InternalCompositionEvidence, InternalCompressionVerifier, InternalCompressionVerifyError,
    InternalMechanismVerifier, InternalPromotionConsolidator,
};

#[test]
fn independently_replays_relational_observation_lowering() {
    let observation = verifier_relational_observation();
    let limits = ObservationFrontierLimits {
        observation: RelationalObservationLimits::default(),
        maximum_frontier_nodes: 16,
    };
    let lowered = Consolidator
        .lower_relational_observation(&observation, limits)
        .expect("producer lowering");
    let verified = InternalMechanismVerifier
        .verify_observation_frontier(&observation, &lowered, limits)
        .expect("independent lowering replay");
    assert_eq!(verified.observation(), observation.observation.id.0);
    assert_eq!(verified.frontier(), lowered.commitment);

    let mut changed_node = lowered.clone();
    changed_node.frontier.nodes[1].operation = "changed-producer-node".to_owned();
    assert_eq!(
        InternalMechanismVerifier.verify_observation_frontier(&observation, &changed_node, limits),
        Err(InternalCompressionVerifyError::ObservationFrontierMismatch)
    );
    let mut changed_provenance = lowered.clone();
    changed_provenance.provenance.swap(1, 2);
    assert_eq!(
        InternalMechanismVerifier.verify_observation_frontier(
            &observation,
            &changed_provenance,
            limits
        ),
        Err(InternalCompressionVerifyError::ObservationFrontierMismatch)
    );
    let mut changed_observation = observation;
    changed_observation.observation.payload = CanonicalValue::Null;
    assert_eq!(
        InternalMechanismVerifier.verify_observation_frontier(
            &changed_observation,
            &lowered,
            limits
        ),
        Err(InternalCompressionVerifyError::InvalidObservation)
    );
}

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

#[test]
#[allow(clippy::too_many_lines)] // Keep all planted composition corruptions beside the positive evidence.
fn independently_verifies_joint_contribution_and_composition_reduction() {
    let first_sources = family_sources(60, "first");
    let second_sources = family_sources(70, "second");
    let first_proposal = Consolidator
        .mine_internal_proof_subgraphs(&first_sources, limits())
        .expect("first proposal");
    let second_proposal = Consolidator
        .mine_internal_proof_subgraphs(&second_sources, limits())
        .expect("second proposal");
    let first = Consolidator
        .compile_internal_proof_mechanism(&first_proposal)
        .expect("first mechanism");
    let second = Consolidator
        .compile_internal_proof_mechanism(&second_proposal)
        .expect("second mechanism");
    let first_compilation = InternalMechanismVerifier
        .verify_compilation(&first_proposal, &first, &first_sources)
        .expect("first compilation");
    let second_compilation = InternalMechanismVerifier
        .verify_compilation(&second_proposal, &second, &second_sources)
        .expect("second compilation");
    let host = composition_host(90);
    let plan = Consolidator
        .compose_compiled_internal_mechanisms(&[first.clone(), second.clone()], &host, 64, 64)
        .expect("producer composition");
    let first_binding = &plan
        .invocations
        .iter()
        .find(|invocation| invocation.mechanism == first.semantic_hash())
        .expect("first invocation")
        .binding;
    let second_binding = &plan
        .invocations
        .iter()
        .find(|invocation| invocation.mechanism == second.semantic_hash())
        .expect("second invocation")
        .binding;
    let verified = InternalMechanismVerifier
        .verify_composition(
            &host,
            &[
                InternalCompositionEvidence {
                    proposal: &second_proposal,
                    mechanism: &second,
                    compilation: &second_compilation,
                    sources: &second_sources,
                    binding: second_binding,
                },
                InternalCompositionEvidence {
                    proposal: &first_proposal,
                    mechanism: &first,
                    compilation: &first_compilation,
                    sources: &first_sources,
                    binding: first_binding,
                },
            ],
            64,
        )
        .expect("independent composition verification");
    assert_eq!(verified.experience(), host.episode);
    assert_eq!(verified.invocations().len(), 2);
    assert_eq!(verified.metrics().candidate_windows_without, 36);
    assert_eq!(verified.metrics().candidate_windows_with, 14);
    assert_eq!(verified.metrics().examples_without, 8);
    assert_eq!(verified.metrics().examples_with, 1);
    assert_eq!(verified.metrics().proof_units_without, 56);
    assert_eq!(verified.metrics().proof_units_with, 8);

    let mut irrelevant_host = host.clone();
    irrelevant_host.output = 4;
    assert_eq!(
        InternalMechanismVerifier.verify_composition(
            &irrelevant_host,
            &[
                InternalCompositionEvidence {
                    proposal: &first_proposal,
                    mechanism: &first,
                    compilation: &first_compilation,
                    sources: &first_sources,
                    binding: first_binding,
                },
                InternalCompositionEvidence {
                    proposal: &second_proposal,
                    mechanism: &second,
                    compilation: &second_compilation,
                    sources: &second_sources,
                    binding: second_binding,
                },
            ],
            64,
        ),
        Err(InternalCompressionVerifyError::IrrelevantInvocation)
    );
    let mut overlapping = second_binding.clone();
    overlapping.selected_nodes = first_binding.selected_nodes.clone();
    assert_eq!(
        InternalMechanismVerifier.verify_composition(
            &host,
            &[
                InternalCompositionEvidence {
                    proposal: &first_proposal,
                    mechanism: &first,
                    compilation: &first_compilation,
                    sources: &first_sources,
                    binding: first_binding,
                },
                InternalCompositionEvidence {
                    proposal: &second_proposal,
                    mechanism: &second,
                    compilation: &second_compilation,
                    sources: &second_sources,
                    binding: &overlapping,
                },
            ],
            64,
        ),
        Err(InternalCompressionVerifyError::OverlappingInvocation)
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Positive construction and planted trust-boundary changes form one audit.
fn independently_verifies_proof_construction_from_an_evidence_frontier() {
    let first_sources = family_sources(60, "first");
    let second_sources = family_sources(70, "second");
    let first_proposal = Consolidator
        .mine_internal_proof_subgraphs(&first_sources, limits())
        .expect("first proposal");
    let second_proposal = Consolidator
        .mine_internal_proof_subgraphs(&second_sources, limits())
        .expect("second proposal");
    let first = Consolidator
        .compile_internal_proof_mechanism(&first_proposal)
        .expect("first mechanism");
    let second = Consolidator
        .compile_internal_proof_mechanism(&second_proposal)
        .expect("second mechanism");
    let first_compilation = InternalMechanismVerifier
        .verify_compilation(&first_proposal, &first, &first_sources)
        .expect("first compilation");
    let second_compilation = InternalMechanismVerifier
        .verify_compilation(&second_proposal, &second, &second_sources)
        .expect("second compilation");
    let frontier = construction_frontier(100);
    let expected = expected_construction(&frontier, &first, &second);
    let candidates = Consolidator
        .construct_compiled_internal_compositions(
            &[first.clone(), second.clone()],
            &frontier,
            InternalConstructionLimits {
                maximum_bindings_per_mechanism: 32,
                maximum_candidates: 512,
            },
        )
        .expect("construction candidates");
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.graph == expected)
        .expect("exact held-out construction");
    let first_binding = &candidate
        .invocations
        .iter()
        .find(|invocation| invocation.mechanism == first.semantic_hash())
        .expect("first invocation")
        .binding;
    let second_binding = &candidate
        .invocations
        .iter()
        .find(|invocation| invocation.mechanism == second.semantic_hash())
        .expect("second invocation")
        .binding;
    let evidence = [
        InternalCompositionEvidence {
            proposal: &first_proposal,
            mechanism: &first,
            compilation: &first_compilation,
            sources: &first_sources,
            binding: first_binding,
        },
        InternalCompositionEvidence {
            proposal: &second_proposal,
            mechanism: &second,
            compilation: &second_compilation,
            sources: &second_sources,
            binding: second_binding,
        },
    ];
    let verified = InternalMechanismVerifier
        .verify_constructed_composition(&frontier, &expected, candidate, &evidence, 64)
        .expect("independent construction verification");
    assert_eq!(verified.composition().invocations().len(), 2);
    assert_eq!(
        verified.composition().metrics().candidate_windows_without,
        36
    );
    assert_eq!(verified.composition().metrics().candidate_windows_with, 16);

    let mut wrong_expected = expected.clone();
    wrong_expected.nodes[2].operation = "changed-held-out-law".to_owned();
    assert_eq!(
        InternalMechanismVerifier.verify_constructed_composition(
            &frontier,
            &wrong_expected,
            candidate,
            &evidence,
            64,
        ),
        Err(InternalCompressionVerifyError::ConstructedProofMismatch)
    );
    let mut wrong_frontier = frontier.clone();
    wrong_frontier.nodes[1]
        .arguments
        .push(datum(ProofValueType::Atom, "new-value"));
    assert_eq!(
        InternalMechanismVerifier.verify_constructed_composition(
            &wrong_frontier,
            &expected,
            candidate,
            &evidence,
            64,
        ),
        Err(InternalCompressionVerifyError::FrontierMismatch)
    );
    let mut invalid_joint = candidate.clone();
    invalid_joint
        .graph
        .nodes
        .last_mut()
        .expect("joint")
        .inputs
        .pop();
    let invalid_expected = invalid_joint.graph.clone();
    assert_eq!(
        InternalMechanismVerifier.verify_constructed_composition(
            &frontier,
            &invalid_expected,
            &invalid_joint,
            &evidence,
            64,
        ),
        Err(InternalCompressionVerifyError::InvalidJointResult)
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

fn family_sources(base: u8, family: &str) -> [ProofGraph; 4] {
    [
        family_host(base, family, 1, 1, "triangle", "red"),
        family_host(base + 1, family, 2, 2, "circle", "blue"),
        family_host(base + 2, family, 3, 1, "segment", "green"),
        family_host(base + 3, family, 1, 3, "polygon", "yellow"),
    ]
}

fn family_host(
    episode: u8,
    family: &str,
    prefix: usize,
    suffix: usize,
    role: &str,
    effect: &str,
) -> ProofGraph {
    let mut graph = host_graph(episode, prefix, suffix, role, effect);
    for (index, operation) in ["select", "filter", "map", "render"]
        .into_iter()
        .enumerate()
    {
        graph.nodes[prefix + index].operation = format!("{family}-{operation}");
    }
    graph
}

fn composition_host(episode: u8) -> ProofGraph {
    let mut nodes = vec![ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, "composition-world")],
    }];
    for (family, role, effect) in [("first", "spiral", "violet"), ("second", "wave", "orange")] {
        let start = nodes.len();
        nodes.extend([
            ProofNode {
                operation: format!("{family}-select"),
                output_type: ProofValueType::EntitySet,
                inputs: vec![0],
                arguments: vec![datum(ProofValueType::Atom, role)],
            },
            ProofNode {
                operation: format!("{family}-filter"),
                output_type: ProofValueType::EntitySet,
                inputs: vec![u32::try_from(start).expect("small")],
                arguments: vec![datum(ProofValueType::Relation, "inside")],
            },
            ProofNode {
                operation: format!("{family}-map"),
                output_type: ProofValueType::Effect,
                inputs: vec![u32::try_from(start + 1).expect("small")],
                arguments: vec![datum(ProofValueType::Atom, effect)],
            },
            ProofNode {
                operation: format!("{family}-render"),
                output_type: ProofValueType::Effect,
                inputs: vec![u32::try_from(start + 2).expect("small")],
                arguments: vec![],
            },
        ]);
    }
    nodes.push(ProofNode {
        operation: "combine-effects".to_owned(),
        output_type: ProofValueType::Effect,
        inputs: vec![4, 8],
        arguments: vec![],
    });
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        nodes,
        output: 9,
    }
}

fn construction_frontier(episode: u8) -> ProofFrontier {
    ProofFrontier {
        episode: Hash32::digest(&[episode]),
        nodes: vec![
            ProofNode {
                operation: "observe-entity-set".to_owned(),
                output_type: ProofValueType::EntitySet,
                inputs: vec![],
                arguments: vec![],
            },
            ProofNode {
                operation: "observe-typed-values".to_owned(),
                output_type: ProofValueType::Atom,
                inputs: vec![],
                arguments: ["first-role", "first-effect", "second-role", "second-effect"]
                    .into_iter()
                    .map(|value| datum(ProofValueType::Atom, value))
                    .collect(),
            },
        ],
    }
}

fn expected_construction(
    frontier: &ProofFrontier,
    first: &cairn_consolidator::CompiledInternalMechanism,
    second: &cairn_consolidator::CompiledInternalMechanism,
) -> ProofGraph {
    let mut nodes = frontier.nodes.clone();
    let mut families = [
        (first.semantic_hash(), "first", "first-role", "first-effect"),
        (
            second.semantic_hash(),
            "second",
            "second-role",
            "second-effect",
        ),
    ];
    families.sort_unstable_by_key(|family| family.0);
    let mut outputs = Vec::new();
    for (_, family, role, effect) in families {
        let start = nodes.len();
        nodes.extend([
            ProofNode {
                operation: format!("{family}-select"),
                output_type: ProofValueType::EntitySet,
                inputs: vec![0],
                arguments: vec![datum(ProofValueType::Atom, role)],
            },
            ProofNode {
                operation: format!("{family}-filter"),
                output_type: ProofValueType::EntitySet,
                inputs: vec![u32::try_from(start).expect("small")],
                arguments: vec![datum(ProofValueType::Relation, "inside")],
            },
            ProofNode {
                operation: format!("{family}-map"),
                output_type: ProofValueType::Effect,
                inputs: vec![u32::try_from(start + 1).expect("small")],
                arguments: vec![datum(ProofValueType::Atom, effect)],
            },
            ProofNode {
                operation: format!("{family}-render"),
                output_type: ProofValueType::Effect,
                inputs: vec![u32::try_from(start + 2).expect("small")],
                arguments: vec![],
            },
        ]);
        outputs.push(u32::try_from(start + 3).expect("small"));
    }
    nodes.push(ProofNode {
        operation: "join-learned-results".to_owned(),
        output_type: ProofValueType::Effect,
        inputs: outputs,
        arguments: vec![],
    });
    ProofGraph {
        episode: frontier.episode,
        output: u32::try_from(nodes.len() - 1).expect("small"),
        nodes,
    }
}

fn verifier_relational_observation() -> RelationalObservation {
    let actor = EvidenceElement {
        id: EvidenceElementId(Hash32::digest(b"actor-id")),
        value_type: EvidenceValueType::Entity,
        value: Hash32::digest(b"actor-value"),
    };
    let effect = EvidenceElement {
        id: EvidenceElementId(Hash32::digest(b"effect-id")),
        value_type: EvidenceValueType::Effect,
        value: Hash32::digest(b"effect-value"),
    };
    RelationalObservation::new(
        ModalityId("urn:cairn:test:verifier-relations:v1".to_owned()),
        Some(1),
        SourceReference {
            locator: "generated:verifier-relations".to_owned(),
            content_hash: Hash32::digest(b"verifier-relations"),
        },
        ObservationCertainty::Exact,
        vec![effect, actor],
        vec![EvidenceRelation {
            predicate: Hash32::digest(b"opaque-relation"),
            participants: vec![actor.id, effect.id],
        }],
        RelationalObservationLimits::default(),
    )
    .expect("relational observation")
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
