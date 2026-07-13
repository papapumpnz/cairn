use cairn_core::{EntityId, Hash32};
use cairn_curriculum::{
    DeterminismStatus, FailureDrivenObjectiveGenerator, LearningFailureCore, RedactionStatus,
    SearchFailure, ValueClass, FAILURE_CORE_SEMANTICS_VERSION,
};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_sequence::{SequenceAdapter, SequenceAttributedPrimitiveObservation};
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawRelationObservationInventor, RawRelationObservationTrace, RelationKeyOperator,
    RelationObservationLimits,
};
use cairn_vector::{VectorAdapter, VectorAttributedPrimitiveObservation};
use cairn_verifier::{
    RawRelationObservationPromotionConsolidator, RawRelationObservationVerifier,
    RelationLanguageExtensionConsolidator, RelationLanguageExtensionLimits,
};
use std::collections::BTreeSet;

#[test]
fn composed_relation_program_transfers_through_unchanged_adapters() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        composed_episode(
            Frontend::Sequence,
            "composed-adapter-sequence-a",
            &[2, 3, 4],
            100,
            [0, 1, 2],
        ),
        composed_episode(
            Frontend::Sequence,
            "composed-adapter-sequence-b",
            &[2, 3, 4, 5],
            300,
            [2, 0, 1],
        ),
    ];
    let learned = RawRelationObservationInventor
        .invent(
            &source,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("sequence composed-relation invention");
    assert!(learned
        .selected_bindings
        .values()
        .all(|binding| binding.operator == cairn_theory::RelationKeyOperator::Add));
    let held_out = [
        composed_episode(
            Frontend::Vector,
            "composed-adapter-vector-a",
            &[2, 3, 4, 5],
            -500,
            [1, 2, 0],
        ),
        composed_episode(
            Frontend::Vector,
            "composed-adapter-vector-b",
            &[2, 3, 4, 5, 6],
            700,
            [2, 1, 0],
        ),
    ];
    let rebound = RawRelationObservationInventor
        .invent(
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("vector composed-relation invention");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
    let receipt = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent composed adapter transfer replay");
    assert_eq!(receipt.replayed_groups(), 9);
    eprintln!(
        "G5_COMPOSED_RELATION source=sequence target=vector supplied_relations=0 supplied_entities=0 relation_structures=5 selected=add heldout_groups=9 exact=9 core_changes=0"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Full verified experience-to-cross-adapter language-growth gate.
fn learned_relation_language_growth_transfers_from_sequence_to_vector() {
    let base_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            composed_episode(
                Frontend::Sequence,
                "growth-adapter-base-sequence-a",
                &[2, 3, 4],
                100,
                [0, 1, 2],
            ),
            composed_episode(
                Frontend::Sequence,
                "growth-adapter-base-sequence-b",
                &[2, 3, 4, 5],
                300,
                [2, 0, 1],
            ),
        ],
        [
            composed_episode(
                Frontend::Vector,
                "growth-adapter-base-vector-a",
                &[3, 4, 5],
                -500,
                [1, 2, 0],
            ),
            composed_episode(
                Frontend::Vector,
                "growth-adapter-base-vector-b",
                &[2, 3, 5, 6],
                700,
                [2, 1, 0],
            ),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            RawRelationObservationInventor
                .invent(
                    cohort,
                    base_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("cross-adapter base invention")
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(&cohorts)
        .map(|(mechanism, cohort)| {
            RawRelationObservationVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    base_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("cross-adapter base reuse")
        })
        .collect::<Vec<_>>();
    let promotion = RawRelationObservationPromotionConsolidator
        .consolidate(&receipts)
        .expect("cross-adapter base promotion");
    let base_mechanism = learned[0]
        .source_independent()
        .expect("source-independent cross-adapter base");
    assert_eq!(base_mechanism.operator, RelationKeyOperator::Add);
    let growth_request = FailureDrivenObjectiveGenerator
        .generate(&arity_three_failure(), &BTreeSet::new())
        .expect("cross-adapter language objective")
        .relation_language_growth_request();
    let extended_limits = RelationLanguageExtensionConsolidator
        .consolidate(
            &base_mechanism,
            &promotion,
            &growth_request,
            RelationLanguageExtensionLimits::default(),
        )
        .expect("cross-adapter language extension")
        .apply(base_limits)
        .expect("apply cross-adapter language extension");

    let source = [
        fold_adapter_episode(
            Frontend::Sequence,
            "growth-adapter-fold-sequence-a",
            &[2, 3, 4],
            100,
            [0, 1, 2, 3],
        ),
        fold_adapter_episode(
            Frontend::Sequence,
            "growth-adapter-fold-sequence-b",
            &[2, 3, 4, 5],
            300,
            [3, 0, 2, 1],
        ),
    ];
    let source_learned = RawRelationObservationInventor
        .invent(
            &source,
            extended_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("sequence fold invention");
    let held_out = [
        fold_adapter_episode(
            Frontend::Vector,
            "growth-adapter-fold-vector-a",
            &[2, 3, 4, 5],
            -500,
            [1, 3, 0, 2],
        ),
        fold_adapter_episode(
            Frontend::Vector,
            "growth-adapter-fold-vector-b",
            &[2, 3, 4, 5, 6],
            700,
            [2, 0, 3, 1],
        ),
    ];
    let rebound = RawRelationObservationInventor
        .invent(
            &held_out,
            extended_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("vector fold rebinding");
    assert_eq!(source_learned.mechanism_hash(), rebound.mechanism_hash());
    let receipt = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            extended_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("cross-adapter fold replay");
    assert_eq!(receipt.replayed_groups(), 9);
    eprintln!(
        "G5_RELATION_LANGUAGE_GROWTH source=sequence target=vector prior=add2 unlocked=sum3 heldout_groups=9 exact=9 core_changes=0"
    );
}

#[test]
fn one_relation_invention_mechanism_rebinds_from_sequence_to_vector() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        episode(
            Frontend::Sequence,
            "raw-adapter-sequence-a",
            &[2, 3, 4],
            100,
            0,
        ),
        episode(
            Frontend::Sequence,
            "raw-adapter-sequence-b",
            &[2, 3, 4, 5],
            300,
            2,
        ),
    ];
    let learned = RawRelationObservationInventor
        .invent(
            &source,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("sequence raw-relation invention");
    let held_out = [
        episode(
            Frontend::Vector,
            "raw-adapter-vector-a",
            &[2, 3, 4, 5],
            -500,
            1,
        ),
        episode(
            Frontend::Vector,
            "raw-adapter-vector-b",
            &[2, 3, 4, 5, 6],
            700,
            0,
        ),
    ];
    let rebound = RawRelationObservationInventor
        .invent(
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("vector raw-relation invention");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
    let receipt = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent adapter transfer replay");
    assert_eq!(receipt.replayed_groups(), 9);
    eprintln!(
        "G5_RAW_RELATION source=sequence target=vector supplied_relations=0 supplied_entities=0 relation_programs=1 heldout_groups=9 exact=9 core_changes=0"
    );
}

#[derive(Clone, Copy)]
enum Frontend {
    Sequence,
    Vector,
}

fn episode(
    frontend: Frontend,
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    correct_axis: usize,
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = Vec::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let correct = i64::try_from(component).expect("correct key");
                let distractor =
                    i64::try_from((component + member) % component_count).expect("distractor key");
                let unique = i64::try_from(component * 3 + member).expect("unique key") + 70_000;
                let mut attributes = vec![distractor, unique, distractor];
                attributes[correct_axis] = correct;
                primitives.push((id, observation, attributes));
            }
            component += 1;
        }
    }
    let episode = Hash32::digest(domain.as_bytes());
    match frontend {
        Frontend::Sequence => SequenceAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| SequenceAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
        Frontend::Vector => VectorAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| VectorAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
    }
}

fn composed_episode(
    frontend: Frontend,
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 3],
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = Vec::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            let key = signature_offset * 20
                + 10_000
                + i64::try_from(component).expect("component key") * 1_000;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let first_key = i64::try_from(component).expect("component") * 100
                    + i64::try_from(member).expect("member");
                let logical = [
                    first_key,
                    key - first_key,
                    i64::try_from((component + member) % component_count).expect("distractor"),
                ];
                let mut attributes = vec![0_i64; 3];
                for (logical_index, destination) in permutation.into_iter().enumerate() {
                    attributes[destination] = logical[logical_index];
                }
                primitives.push((id, observation, attributes));
            }
            component += 1;
        }
    }
    let episode = Hash32::digest(domain.as_bytes());
    match frontend {
        Frontend::Sequence => SequenceAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| SequenceAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
        Frontend::Vector => VectorAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| VectorAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
    }
}

fn fold_adapter_episode(
    frontend: Frontend,
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 4],
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = Vec::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            let key = signature_offset * 20_000
                + 1_000_000
                + i64::try_from(component).expect("component key") * 100_000;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component}:{member}").as_bytes(),
                ));
                let coordinate = i64::try_from(component * 10 + member + 1).expect("coordinate");
                let logical = [
                    coordinate,
                    coordinate * coordinate + 7,
                    key - coordinate - (coordinate * coordinate + 7),
                    i64::try_from((component * 7 + member * 13) % (component_count * 17 + 1))
                        .expect("distractor"),
                ];
                let mut attributes = vec![0_i64; 4];
                for (logical_index, destination) in permutation.into_iter().enumerate() {
                    attributes[destination] = logical[logical_index];
                }
                primitives.push((id, observation, attributes));
            }
            component += 1;
        }
    }
    lower_adapter_primitives(frontend, domain, &primitives, outputs)
}

fn lower_adapter_primitives(
    frontend: Frontend,
    domain: &str,
    primitives: &[(EntityId, i64, Vec<i64>)],
    outputs: BTreeSet<i64>,
) -> RawRelationObservationTrace {
    let episode = Hash32::digest(domain.as_bytes());
    match frontend {
        Frontend::Sequence => SequenceAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| SequenceAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
        Frontend::Vector => VectorAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| VectorAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
    }
}

fn arity_three_failure() -> LearningFailureCore {
    LearningFailureCore {
        semantics_version: FAILURE_CORE_SEMANTICS_VERSION.to_owned(),
        failure: SearchFailure::NoDefinition { maximum_cost: 2 },
        parameter_types: vec![ValueClass::Entity, ValueClass::Relation],
        return_type: ValueClass::Relation,
        demonstrations: 4,
        varying_parameters: 3,
        distinct_output_classes: 3,
        constant_explanation_possible: false,
        direct_projection_candidates: 0,
        single_parameter_explanations: 0,
        exact_two_parameter_families: BTreeSet::new(),
        exact_three_parameter_families: BTreeSet::new(),
        determinism: DeterminismStatus::Deterministic,
        minimum_dependency_arity_lower_bound: 3,
        redaction: RedactionStatus::ContentFree,
    }
}
