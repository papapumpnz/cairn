use cairn_consolidator::{
    ConsolidationError, Consolidator, InternalConstructionLimits, InternalEpisodeBindings,
    InternalMiningLimits, InternalPatternInput, ProofDatum, ProofFrontier, ProofGraph, ProofNode,
    ProofValueType,
};
use cairn_core::{
    CanonicalEncode, EvidenceElement, EvidenceElementId, EvidenceRelation, EvidenceValueType,
    Hash32, ModalityId, ObservationCertainty, RelationalObservation, RelationalObservationLimits,
    SourceReference,
};

#[test]
fn mines_and_rebinds_an_internal_mechanism_across_different_hosts() {
    let sources = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    assert_eq!(
        Consolidator.compress_proof_subgraphs(&sources),
        Err(ConsolidationError::IncompatibleGraphShapes)
    );
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&sources, limits)
        .expect("embedded repeated mechanism");
    assert_eq!(proposal.pattern().len(), 4);
    assert_eq!(proposal.boundary_types(), [ProofValueType::EntitySet]);
    assert_eq!(proposal.bindings().len(), 4);
    assert_eq!(proposal.bindings()[0].values.len(), 2);
    assert!(proposal.pattern()[0]
        .inputs
        .contains(&InternalPatternInput::Boundary(0)));
    assert!(proposal.compressed_nodes() < proposal.uncompressed_nodes());
    assert_eq!(proposal.verified_replays(), 4);

    let held_out = host_graph(9, 4, 4, "star", "purple");
    let matches = Consolidator
        .match_internal_proof_subgraph(&proposal, &held_out, 64)
        .expect("held-out host rebinding");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].values.len(), 2);
    assert_eq!(matches[0].boundary_nodes.len(), 1);
    assert_eq!(matches[0].selected_nodes, vec![4, 5, 6, 7]);

    let reversed = sources.iter().cloned().rev().collect::<Vec<_>>();
    let replayed = Consolidator
        .mine_internal_proof_subgraphs(&reversed, limits)
        .expect("order-independent mining");
    assert_eq!(proposal.semantic_hash(), replayed.semantic_hash());
    eprintln!(
        "INTERNAL_SUBGRAPH proposal={} hosts=4 held_out=1 nodes=4 boundary_parameters=1 value_parameters=2 cost={}->{}",
        proposal.semantic_hash(),
        proposal.uncompressed_nodes(),
        proposal.compressed_nodes()
    );
}

#[test]
fn compiles_a_source_independent_schema_and_expands_unseen_bindings() {
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let first_sources = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    let second_sources = [
        host_graph(11, 4, 1, "arc", "cyan"),
        host_graph(12, 2, 4, "box", "magenta"),
        host_graph(13, 3, 2, "line", "amber"),
        host_graph(14, 1, 5, "point", "white"),
    ];
    let first_proposal = Consolidator
        .mine_internal_proof_subgraphs(&first_sources, limits)
        .expect("first evidence cohort");
    let second_proposal = Consolidator
        .mine_internal_proof_subgraphs(&second_sources, limits)
        .expect("independent evidence cohort");
    assert_ne!(
        first_proposal.semantic_hash(),
        second_proposal.semantic_hash(),
        "evidence-bearing proposal identities must retain distinct lineage"
    );
    let first = Consolidator
        .compile_internal_proof_mechanism(&first_proposal)
        .expect("compiled first cohort");
    let second = Consolidator
        .compile_internal_proof_mechanism(&second_proposal)
        .expect("compiled second cohort");
    assert_eq!(first.semantic_hash(), second.semantic_hash());
    assert_eq!(
        first.semantic_hash(),
        Hash32::digest(&first.encode_canonical().expect("canonical mechanism"))
    );
    assert_eq!(
        first.encode_canonical().expect("first canonical mechanism"),
        second
            .encode_canonical()
            .expect("second canonical mechanism"),
        "source proposal lineage must not enter reusable mechanism semantics"
    );
    assert_ne!(first.source_proposal(), second.source_proposal());
    assert_eq!(first.parameter_types(), [ProofValueType::Atom; 2]);

    let held_out = host_graph(21, 5, 3, "spiral", "violet");
    let bindings = Consolidator
        .match_compiled_internal_mechanism(&first, &held_out, 64)
        .expect("compiled schema rebinding");
    assert_eq!(bindings.len(), 1);
    let expanded = Consolidator
        .expand_compiled_internal_mechanism(&first, &bindings[0])
        .expect("reference expansion");
    assert_eq!(expanded, held_out.nodes[5..9]);

    let mut mistyped = bindings[0].clone();
    mistyped.values[0] = datum(ProofValueType::Integer, "7");
    assert_eq!(
        Consolidator.expand_compiled_internal_mechanism(&first, &mistyped),
        Err(ConsolidationError::ReplayMismatch)
    );
    let missing_boundary = InternalEpisodeBindings {
        boundary_nodes: Vec::new(),
        ..bindings[0].clone()
    };
    assert_eq!(
        Consolidator.expand_compiled_internal_mechanism(&first, &missing_boundary),
        Err(ConsolidationError::ReplayMismatch)
    );
    eprintln!(
        "COMPILED_INTERNAL mechanism={} proposals=2 source_cohorts=8 held_out=1 exact_expansion=1",
        first.semantic_hash()
    );
}

#[test]
fn near_matches_types_and_resource_caps_fail_closed() {
    let mut sources = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let internal_start = 3;
    sources[2].nodes[internal_start + 1].operation = "different-filter".to_owned();
    assert_eq!(
        Consolidator.mine_internal_proof_subgraphs(&sources, limits),
        Err(ConsolidationError::NoInternalCompression)
    );

    let clean = [
        host_graph(1, 1, 1, "triangle", "red"),
        host_graph(2, 2, 2, "circle", "blue"),
        host_graph(3, 3, 1, "segment", "green"),
        host_graph(4, 1, 3, "polygon", "yellow"),
    ];
    assert_eq!(
        Consolidator.mine_internal_proof_subgraphs(
            &clean,
            InternalMiningLimits {
                maximum_windows_per_episode: 1,
                ..limits
            }
        ),
        Err(ConsolidationError::Capacity)
    );
    assert_eq!(
        Consolidator.mine_internal_proof_subgraphs(
            &clean,
            InternalMiningLimits {
                minimum_nodes: 1,
                ..limits
            }
        ),
        Err(ConsolidationError::InvalidMiningLimits)
    );

    let proposal = Consolidator
        .mine_internal_proof_subgraphs(&clean, limits)
        .expect("clean proposal");
    let mut wrong_fixed_relation = host_graph(8, 2, 2, "star", "orange");
    let start = 2;
    wrong_fixed_relation.nodes[start + 1].arguments[0] =
        datum(ProofValueType::Relation, "overlaps");
    assert_eq!(
        Consolidator.match_internal_proof_subgraph(&proposal, &wrong_fixed_relation, 64),
        Err(ConsolidationError::NoInternalMatch)
    );
}

#[test]
fn composes_distinct_learned_mechanisms_only_when_both_contribute() {
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let first_sources = [
        renamed_host(31, "a", "triangle", "red"),
        renamed_host(32, "a", "circle", "blue"),
        renamed_host(33, "a", "segment", "green"),
        renamed_host(34, "a", "polygon", "yellow"),
    ];
    let second_sources = [
        renamed_host(41, "b", "arc", "cyan"),
        renamed_host(42, "b", "box", "magenta"),
        renamed_host(43, "b", "line", "amber"),
        renamed_host(44, "b", "point", "white"),
    ];
    let first = Consolidator
        .mine_internal_proof_subgraphs(&first_sources, limits)
        .and_then(|proposal| Consolidator.compile_internal_proof_mechanism(&proposal))
        .expect("first mechanism");
    let second = Consolidator
        .mine_internal_proof_subgraphs(&second_sources, limits)
        .and_then(|proposal| Consolidator.compile_internal_proof_mechanism(&proposal))
        .expect("second mechanism");
    let host = parallel_composition_host(50, true);
    assert_eq!(
        Consolidator
            .match_compiled_internal_mechanism(&first, &host, 64)
            .expect("first match")
            .len(),
        1
    );
    assert_eq!(
        Consolidator
            .match_compiled_internal_mechanism(&second, &host, 64)
            .expect("second match")
            .len(),
        1
    );
    let plan = Consolidator
        .compose_compiled_internal_mechanisms(&[first.clone(), second.clone()], &host, 64, 64)
        .expect("composition plan");
    assert_eq!(plan.episode, host.episode);
    assert_eq!(plan.invocations.len(), 2);
    assert!(plan.invocations[0]
        .binding
        .selected_nodes
        .iter()
        .all(|node| !plan.invocations[1].binding.selected_nodes.contains(node)));

    assert_eq!(
        Consolidator.compose_compiled_internal_mechanisms(
            &[first.clone(), first.clone()],
            &host,
            64,
            64
        ),
        Err(ConsolidationError::DuplicateMechanism)
    );
    assert_eq!(
        Consolidator.compose_compiled_internal_mechanisms(
            &[second.clone(), first.clone()],
            &parallel_composition_host(51, false),
            64,
            64,
        ),
        Err(ConsolidationError::NoInternalComposition)
    );
    assert_eq!(
        Consolidator.compose_compiled_internal_mechanisms(&[second, first], &host, 64, 1),
        Err(ConsolidationError::Capacity)
    );
}

#[test]
fn constructs_composition_candidates_from_evidence_without_a_completed_host() {
    let (first, second, frontier) = construction_fixture();
    assert!(frontier
        .nodes
        .iter()
        .all(|node| !node.operation.starts_with("a-") && !node.operation.starts_with("b-")));
    let candidates = Consolidator
        .construct_compiled_internal_compositions(
            &[first.clone(), second.clone()],
            &frontier,
            InternalConstructionLimits {
                maximum_bindings_per_mechanism: 32,
                maximum_candidates: 512,
            },
        )
        .expect("constructed candidates");
    assert_eq!(candidates.len(), 256);
    assert!(candidates.iter().all(|candidate| {
        candidate.graph.nodes.len() == frontier.nodes.len() + 9
            && candidate
                .graph
                .nodes
                .last()
                .is_some_and(|node| node.operation == "join-learned-results")
            && candidate.invocations.len() == 2
    }));
    let desired = [
        datum(ProofValueType::Atom, "first-role"),
        datum(ProofValueType::Atom, "first-effect"),
        datum(ProofValueType::Atom, "second-role"),
        datum(ProofValueType::Atom, "second-effect"),
    ];
    assert_eq!(
        candidates
            .iter()
            .filter(|candidate| {
                candidate.invocations.iter().all(|invocation| {
                    if invocation.mechanism == first.semantic_hash() {
                        invocation.binding.values == desired[..2]
                    } else {
                        invocation.binding.values == desired[2..]
                    }
                })
            })
            .count(),
        1
    );
    let training = candidates
        .iter()
        .find(|candidate| {
            candidate.invocations.iter().all(|invocation| {
                if invocation.mechanism == first.semantic_hash() {
                    invocation.binding.values == desired[..2]
                } else {
                    invocation.binding.values == desired[2..]
                }
            })
        })
        .expect("training construction");
    let policy = Consolidator
        .induce_internal_construction_policy(&[first.clone(), second.clone()], &frontier, training)
        .expect("typed ordinal policy");
    let mut changed = construction_frontier(81);
    changed.nodes[1].arguments = ["new-a", "new-b", "new-c", "new-d"]
        .into_iter()
        .map(|value| datum(ProofValueType::Atom, value))
        .collect();
    let transferred = Consolidator
        .apply_internal_construction_policy(&[first.clone(), second.clone()], &changed, &policy)
        .expect("changed-value construction");
    assert_eq!(transferred.candidates_considered, 1);
    assert!(transferred.invocations.iter().all(|invocation| {
        if invocation.mechanism == first.semantic_hash() {
            invocation.binding.values == changed.nodes[1].arguments[..2]
        } else {
            invocation.binding.values == changed.nodes[1].arguments[2..]
        }
    }));
    changed.nodes[1].arguments.pop();
    assert_eq!(
        Consolidator.apply_internal_construction_policy(
            &[first.clone(), second.clone()],
            &changed,
            &policy
        ),
        Err(ConsolidationError::ReplayMismatch)
    );
    assert_eq!(
        Consolidator.construct_compiled_internal_compositions(
            &[first, second],
            &frontier,
            InternalConstructionLimits {
                maximum_bindings_per_mechanism: 1,
                maximum_candidates: 1,
            },
        ),
        Err(ConsolidationError::Capacity)
    );
}

#[test]
fn lowers_relational_observation_and_transfers_by_structure_not_order() {
    let (first, second, _) = construction_fixture();
    let training_observation = relational_observation(
        90,
        ["first-role", "first-effect", "second-role", "second-effect"],
        false,
    );
    let limits = cairn_consolidator::ObservationFrontierLimits {
        observation: RelationalObservationLimits::default(),
        maximum_frontier_nodes: 64,
    };
    let training_lowering = Consolidator
        .lower_relational_observation(&training_observation, limits)
        .expect("training lowering");
    assert_eq!(training_lowering.frontier.nodes.len(), 13);
    assert_eq!(
        training_lowering.provenance.len(),
        training_lowering.frontier.nodes.len()
    );
    let candidates = Consolidator
        .construct_compiled_internal_compositions(
            &[first.clone(), second.clone()],
            &training_lowering.frontier,
            InternalConstructionLimits {
                maximum_bindings_per_mechanism: 64,
                maximum_candidates: 2_000,
            },
        )
        .expect("training construction candidates");
    let desired = [
        datum(ProofValueType::Atom, "first-role"),
        datum(ProofValueType::Atom, "first-effect"),
        datum(ProofValueType::Atom, "second-role"),
        datum(ProofValueType::Atom, "second-effect"),
    ];
    let training = candidates
        .iter()
        .find(|candidate| {
            candidate.invocations.iter().all(|invocation| {
                if invocation.mechanism == first.semantic_hash() {
                    invocation.binding.values == desired[..2]
                } else {
                    invocation.binding.values == desired[2..]
                }
            })
        })
        .expect("training truth");
    let policy = Consolidator
        .induce_relational_construction_policy(
            &[first.clone(), second.clone()],
            &training_observation,
            &training_lowering,
            training,
            limits,
        )
        .expect("relational policy");

    let held_out_observation =
        relational_observation(91, ["new-a", "new-b", "new-c", "new-d"], true);
    let held_out_lowering = Consolidator
        .lower_relational_observation(&held_out_observation, limits)
        .expect("held-out lowering");
    let transferred = Consolidator
        .apply_relational_construction_policy(
            &[first.clone(), second.clone()],
            &held_out_observation,
            &held_out_lowering,
            &policy,
            limits,
        )
        .expect("relational transfer");
    let changed = [
        datum(ProofValueType::Atom, "new-a"),
        datum(ProofValueType::Atom, "new-b"),
        datum(ProofValueType::Atom, "new-c"),
        datum(ProofValueType::Atom, "new-d"),
    ];
    assert_eq!(transferred.candidates_considered, 1);
    assert!(transferred.invocations.iter().all(|invocation| {
        if invocation.mechanism == first.semantic_hash() {
            invocation.binding.values == changed[..2]
        } else {
            invocation.binding.values == changed[2..]
        }
    }));

    let ambiguous = relational_observation_with_duplicate_structure(92);
    let ambiguous_lowering = Consolidator
        .lower_relational_observation(&ambiguous, limits)
        .expect("ambiguous lowering remains valid evidence");
    assert_eq!(
        Consolidator.apply_relational_construction_policy(
            &[first, second],
            &ambiguous,
            &ambiguous_lowering,
            &policy,
            limits,
        ),
        Err(ConsolidationError::AmbiguousCorrespondence)
    );
}

fn construction_fixture() -> (
    cairn_consolidator::CompiledInternalMechanism,
    cairn_consolidator::CompiledInternalMechanism,
    ProofFrontier,
) {
    let limits = InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    };
    let first_sources = [
        renamed_host(31, "a", "triangle", "red"),
        renamed_host(32, "a", "circle", "blue"),
        renamed_host(33, "a", "segment", "green"),
        renamed_host(34, "a", "polygon", "yellow"),
    ];
    let second_sources = [
        renamed_host(41, "b", "arc", "cyan"),
        renamed_host(42, "b", "box", "magenta"),
        renamed_host(43, "b", "line", "amber"),
        renamed_host(44, "b", "point", "white"),
    ];
    (
        plan_mechanism(&first_sources, limits),
        plan_mechanism(&second_sources, limits),
        construction_frontier(80),
    )
}

fn relational_observation(
    episode: u8,
    values: [&str; 4],
    reverse_input: bool,
) -> RelationalObservation {
    let marker_types = [
        EvidenceValueType::Boolean,
        EvidenceValueType::Integer,
        EvidenceValueType::Action,
        EvidenceValueType::Mechanism,
    ];
    let mut elements = Vec::new();
    let mut relations = Vec::new();
    for (index, (value, marker_type)) in values.into_iter().zip(marker_types).enumerate() {
        let item = EvidenceElement {
            id: EvidenceElementId(Hash32::digest(format!("item-{episode}-{index}").as_bytes())),
            value_type: EvidenceValueType::Atom,
            value: Hash32::digest(value.as_bytes()),
        };
        let marker = EvidenceElement {
            id: EvidenceElementId(Hash32::digest(
                format!("marker-{episode}-{index}").as_bytes(),
            )),
            value_type: marker_type,
            value: Hash32::digest(format!("marker-value-{episode}-{index}").as_bytes()),
        };
        relations.push(EvidenceRelation {
            predicate: Hash32::digest(b"opaque-structural-link"),
            participants: vec![item.id, marker.id],
        });
        elements.extend([item, marker]);
    }
    if reverse_input {
        elements.reverse();
        relations.reverse();
    }
    RelationalObservation::new(
        ModalityId(format!("urn:cairn:test:domain-{episode}:v1")),
        Some(u64::from(episode)),
        SourceReference {
            locator: format!("generated:relational/{episode}"),
            content_hash: Hash32::digest(&[episode]),
        },
        ObservationCertainty::Exact,
        elements,
        relations,
        RelationalObservationLimits::default(),
    )
    .expect("relational observation")
}

fn relational_observation_with_duplicate_structure(episode: u8) -> RelationalObservation {
    let mut observation =
        relational_observation(episode, ["new-a", "new-b", "new-c", "new-d"], true);
    let item = EvidenceElement {
        id: EvidenceElementId(Hash32::digest(b"duplicate-item")),
        value_type: EvidenceValueType::Atom,
        value: Hash32::digest(b"distractor"),
    };
    let marker = EvidenceElement {
        id: EvidenceElementId(Hash32::digest(b"duplicate-marker")),
        value_type: EvidenceValueType::Boolean,
        value: Hash32::digest(b"distractor-marker"),
    };
    let mut elements = observation.elements;
    let mut relations = observation.relations;
    elements.extend([item, marker]);
    relations.push(EvidenceRelation {
        predicate: Hash32::digest(b"opaque-structural-link"),
        participants: vec![item.id, marker.id],
    });
    observation = RelationalObservation::new(
        ModalityId(format!("urn:cairn:test:ambiguous-{episode}:v1")),
        Some(u64::from(episode)),
        SourceReference {
            locator: format!("generated:ambiguous/{episode}"),
            content_hash: Hash32::digest(&[episode, 1]),
        },
        ObservationCertainty::Exact,
        elements,
        relations,
        RelationalObservationLimits::default(),
    )
    .expect("ambiguous observation");
    observation
}

fn plan_mechanism(
    sources: &[ProofGraph],
    limits: InternalMiningLimits,
) -> cairn_consolidator::CompiledInternalMechanism {
    Consolidator
        .mine_internal_proof_subgraphs(sources, limits)
        .and_then(|proposal| Consolidator.compile_internal_proof_mechanism(&proposal))
        .expect("compiled mechanism")
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

fn renamed_host(episode: u8, family: &str, role: &str, effect: &str) -> ProofGraph {
    let prefix = usize::from(episode % 3) + 1;
    let suffix = usize::from(episode % 2) + 1;
    let mut graph = host_graph(episode, prefix, suffix, role, effect);
    for (index, operation) in ["select", "filter", "map", "render"]
        .into_iter()
        .enumerate()
    {
        graph.nodes[index + prefix].operation = format!("{family}-{operation}");
    }
    graph
}

fn parallel_composition_host(episode: u8, both_contribute: bool) -> ProofGraph {
    let mut nodes = vec![ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, "composed-world")],
    }];
    for (family, role, effect) in [("a", "spiral", "violet"), ("b", "wave", "orange")] {
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
        inputs: if both_contribute { vec![4, 8] } else { vec![4] },
        arguments: vec![],
    });
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        output: 9,
        nodes,
    }
}

fn host_graph(
    episode: u8,
    prefix_nodes: usize,
    suffix_nodes: usize,
    role: &str,
    effect: &str,
) -> ProofGraph {
    let mut nodes = Vec::new();
    nodes.push(ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, &format!("world-{episode}"))],
    });
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
