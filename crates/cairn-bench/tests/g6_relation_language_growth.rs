use cairn_core::{EntityId, Hash32};
use cairn_curriculum::{
    AutonomousCurriculum, DeterminismStatus, DevelopmentalNeed, DevelopmentalOutcome,
    FailureDrivenObjectiveGenerator, LearningFailureCore, RedactionStatus, SearchFailure,
    ValueClass, FAILURE_CORE_SEMANTICS_VERSION,
};
use cairn_invent::InventionLimits;
use cairn_memory::RawRelationStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationObservationInventor, RawRelationObservationTrace,
    RelationKeyOperator, RelationObservationError, RelationObservationLimits,
};
use cairn_verifier::{
    RawRelationObservationPromotionConsolidator, RawRelationObservationVerifier,
    RelationLanguageExtensionConsolidator, RelationLanguageExtensionError,
    RelationLanguageExtensionLimits, VerifiedRawRelationObservationReuse,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Complete failure -> language growth -> promotion -> reuse loop.
fn verified_experience_expands_the_relation_language_for_a_structural_failure() {
    let base_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();

    let base_cohorts = [
        [
            pair_episode("growth-base-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
            pair_episode("growth-base-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            pair_episode("growth-base-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
            pair_episode("growth-base-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let (base_mechanism, base_promotion) = learn_and_promote(
        &base_cohorts,
        base_limits,
        entity_limits,
        observation_limits,
        predicate_limits,
    );
    assert_eq!(base_mechanism.operator, RelationKeyOperator::Add);
    assert_eq!(base_mechanism.key_arity, 2);

    let generated = FailureDrivenObjectiveGenerator
        .generate(&arity_failure_core(3), &BTreeSet::new())
        .expect("content-free arity objective");
    assert_eq!(generated.need, DevelopmentalNeed::RelationalStructure);
    let request = generated.exercise_request();
    assert_eq!(request.minimum_dependency_arity_lower_bound, 3);
    let growth_request = generated.relation_language_growth_request();
    let mut curriculum =
        AutonomousCurriculum::new(vec![generated.objective.clone()], 2).expect("curriculum");
    let decision = curriculum.select_next().expect("select structural gap");

    let extension = RelationLanguageExtensionConsolidator
        .consolidate(
            &base_mechanism,
            &base_promotion,
            &growth_request,
            RelationLanguageExtensionLimits::default(),
        )
        .expect("verified associative language extension");
    assert_eq!(extension.fold_operator(), RelationKeyOperator::Sum);
    assert_eq!(extension.maximum_key_arity(), 3);
    assert_eq!(extension.failure_core(), request.failure_core);
    assert_eq!(
        RelationLanguageExtensionConsolidator.consolidate(
            &base_mechanism,
            &base_promotion,
            &FailureDrivenObjectiveGenerator
                .generate(&arity_failure_core(2), &BTreeSet::new())
                .expect("low-arity objective")
                .relation_language_growth_request(),
            RelationLanguageExtensionLimits::default(),
        ),
        Err(RelationLanguageExtensionError::InvalidRequestedArity)
    );
    assert_eq!(
        RelationLanguageExtensionConsolidator.consolidate(
            &base_mechanism,
            &base_promotion,
            &FailureDrivenObjectiveGenerator
                .generate(&arity_failure_core(9), &BTreeSet::new())
                .expect("over-cap objective")
                .relation_language_growth_request(),
            RelationLanguageExtensionLimits {
                maximum_key_arity: 8,
            },
        ),
        Err(RelationLanguageExtensionError::InvalidRequestedArity)
    );

    let fold_cohorts = [
        [
            fold_episode("growth-fold-sequence-a", &[2, 3, 4], 100, [0, 1, 2, 3]),
            fold_episode("growth-fold-table-a", &[2, 3, 4, 5], 300, [3, 0, 2, 1]),
        ],
        [
            fold_episode("growth-fold-vector-b", &[3, 4, 5], -500, [1, 3, 0, 2]),
            fold_episode("growth-fold-world-b", &[2, 3, 5, 6], 700, [2, 0, 3, 1]),
        ],
    ];
    assert!(matches!(
        RawRelationObservationInventor.invent(
            &fold_cohorts[0],
            base_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        ),
        Err(RelationObservationError::Entity(_))
    ));
    let extended_limits = extension
        .apply(base_limits)
        .expect("apply proposal language");
    let fold_learned = fold_cohorts
        .iter()
        .map(|cohort| {
            RawRelationObservationInventor
                .invent(
                    cohort,
                    extended_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("invent newly reachable fold")
        })
        .collect::<Vec<_>>();
    assert!(fold_learned.iter().all(|learned| learned
        .selected_bindings
        .values()
        .all(|binding| binding.operator == RelationKeyOperator::Sum)));
    let fold_receipts = fold_learned
        .iter()
        .zip(&fold_cohorts)
        .map(|(learned, cohort)| {
            RawRelationObservationVerifier
                .verify_reuse(
                    learned,
                    cohort,
                    extended_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("independent fold reuse")
        })
        .collect::<Vec<_>>();
    assert!(fold_receipts
        .iter()
        .all(|receipt| { receipt.bindings_without() == 64 && receipt.bindings_with() == 8 }));
    let fold_promotion = RawRelationObservationPromotionConsolidator
        .consolidate(&fold_receipts)
        .expect("fold promotion");
    let mut store = RawRelationStore::new();
    let fold_id = store
        .insert_provisional(
            fold_learned[0]
                .source_independent()
                .expect("source-independent fold"),
        )
        .expect("provisional fold");
    for receipt in &fold_receipts {
        store
            .record_verified_reuse(fold_id, receipt.clone())
            .expect("fold receipt");
    }
    assert!(store.apply_verified_promotion(fold_id, &fold_promotion));

    let held_out = [
        fold_episode("growth-fold-graphics", &[2, 3, 4, 5], 1_100, [1, 3, 0, 2]),
        fold_episode("growth-fold-control", &[2, 3, 4, 5, 6], -900, [2, 0, 3, 1]),
    ];
    let runtime = CognitionRuntime::with_raw_relation_memory(store);
    let exact = held_out
        .iter()
        .map(|trace| {
            let outcome = runtime
                .propose_raw_relation_programs_from_memory(
                    trace,
                    extended_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                )
                .expect("learned fold proposals");
            assert_eq!(outcome.relation_bindings_attempted, 4);
            u64::from(
                outcome
                    .proposals
                    .iter()
                    .any(|proposal| proposal.output == trace.outputs),
            )
        })
        .sum::<u64>();
    assert_eq!(exact, 2);

    curriculum
        .record(DevelopmentalOutcome {
            decision_sequence: decision.sequence,
            objective: decision.objective,
            mechanism_evidence: format!("verified:{fold_id}"),
            independently_verified: true,
            held_out_transfer_cases: 9,
            candidate_work_without: fold_receipts
                .iter()
                .map(VerifiedRawRelationObservationReuse::bindings_without)
                .sum(),
            candidate_work_with: fold_receipts
                .iter()
                .map(VerifiedRawRelationObservationReuse::bindings_with)
                .sum(),
            examples_without: 4,
            examples_with: 0,
            retained_prior_tasks: 1,
            prior_tasks: 1,
            negative_transfers: 0,
        })
        .expect("record verified language growth");
    assert_eq!(curriculum.completed_objectives(), 1);
    eprintln!(
        "G6_RELATION_LANGUAGE_GROWTH binary_experience=add2 requested_arity=3 unlocked=sum3 blank_exact=0 cohort_bindings=128->16 examples=4->0 heldout_groups=9 exact=9 negative_transfer=0"
    );
}

#[test]
fn verified_minimum_experience_unlocks_only_the_minimum_fold_family() {
    let base_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let base_cohorts = [
        [
            minimum_pair_episode("growth-min-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
            minimum_pair_episode("growth-min-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            minimum_pair_episode("growth-min-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
            minimum_pair_episode("growth-min-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let (base_mechanism, base_promotion) = learn_and_promote(
        &base_cohorts,
        base_limits,
        entity_limits,
        observation_limits,
        predicate_limits,
    );
    assert_eq!(base_mechanism.operator, RelationKeyOperator::Minimum);

    let growth_request = FailureDrivenObjectiveGenerator
        .generate(&arity_failure_core(3), &BTreeSet::new())
        .expect("content-free minimum-fold objective")
        .relation_language_growth_request();
    let extension = RelationLanguageExtensionConsolidator
        .consolidate(
            &base_mechanism,
            &base_promotion,
            &growth_request,
            RelationLanguageExtensionLimits::default(),
        )
        .expect("minimum fold extension");
    assert_eq!(extension.fold_operator(), RelationKeyOperator::MinimumFold);
    let extended_limits = extension.apply(base_limits).expect("apply minimum fold");

    let source = [
        minimum_fold_episode("growth-min-fold-sequence", &[2, 3, 4], 100, [0, 1, 2, 3]),
        minimum_fold_episode("growth-min-fold-table", &[2, 3, 4, 5], 300, [3, 0, 2, 1]),
    ];
    let learned = RawRelationObservationInventor
        .invent(
            &source,
            extended_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("minimum-fold invention");
    assert!(learned.selected_bindings.values().all(|binding| {
        binding.operator == RelationKeyOperator::MinimumFold && binding.attributes.len() == 3
    }));

    let held_out = [
        minimum_fold_episode(
            "growth-min-fold-graphics",
            &[2, 3, 4, 5],
            -500,
            [1, 3, 0, 2],
        ),
        minimum_fold_episode(
            "growth-min-fold-control",
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
        .expect("held-out minimum-fold invention");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
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
        .expect("independent minimum-fold replay");
    assert_eq!(receipt.replayed_groups(), 9);
    eprintln!(
        "G6_RELATION_LANGUAGE_MINIMUM binary_experience=minimum2 requested_arity=3 unlocked=minimum3 heldout_groups=9 exact=9"
    );
}

fn learn_and_promote(
    cohorts: &[[RawRelationObservationTrace; 2]; 2],
    relation_limits: RelationObservationLimits,
    entity_limits: EntityObservationLimits,
    observation_limits: ObservationPredicateLimits,
    predicate_limits: PredicateGroupingLimits,
) -> (
    cairn_theory::RawRelationMechanism,
    cairn_verifier::VerifiedRawRelationObservationPromotion,
) {
    let learned = cohorts
        .iter()
        .map(|cohort| {
            RawRelationObservationInventor
                .invent(
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("base relation invention")
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(cohorts)
        .map(|(mechanism, cohort)| {
            RawRelationObservationVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("base relation receipt")
        })
        .collect::<Vec<_>>();
    (
        learned[0]
            .source_independent()
            .expect("source-independent base"),
        RawRelationObservationPromotionConsolidator
            .consolidate(&receipts)
            .expect("base promotion"),
    )
}

fn arity_failure_core(minimum_dependency_arity_lower_bound: usize) -> LearningFailureCore {
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
        minimum_dependency_arity_lower_bound,
        redaction: RedactionStatus::ContentFree,
    }
}

fn pair_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 3],
) -> RawRelationObservationTrace {
    build_episode(
        domain,
        cardinalities,
        signature_offset,
        3,
        |component, member, key, component_count| {
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
            attributes
        },
    )
}

fn minimum_pair_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 3],
) -> RawRelationObservationTrace {
    build_episode(
        domain,
        cardinalities,
        signature_offset,
        3,
        |component, member, key, component_count| {
            let offset = i64::try_from(component_count + component + 1).expect("offset") * 100;
            let logical = match member {
                0 => [key, key + offset, key + offset * 7],
                1 => [key + offset * 2, key, key + offset * 11],
                _ => [key, key + offset * 3, key + offset * 13],
            };
            let mut attributes = vec![0_i64; 3];
            for (logical_index, destination) in permutation.into_iter().enumerate() {
                attributes[destination] = logical[logical_index];
            }
            attributes
        },
    )
}

fn fold_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 4],
) -> RawRelationObservationTrace {
    build_episode(
        domain,
        cardinalities,
        signature_offset,
        4,
        |component, member, key, component_count| {
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
            attributes
        },
    )
}

fn minimum_fold_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 4],
) -> RawRelationObservationTrace {
    build_episode(
        domain,
        cardinalities,
        signature_offset,
        4,
        |component, member, key, component_count| {
            let offset = i64::try_from(component_count + component + 1).expect("offset") * 100;
            let logical = match member {
                0 => [key, key + offset, key + offset * 2, key + offset * 7],
                1 => [key + offset * 3, key, key + offset * 4, key + offset * 11],
                _ => [key + offset * 5, key + offset * 6, key, key + offset * 13],
            };
            let mut attributes = vec![0_i64; 4];
            for (logical_index, destination) in permutation.into_iter().enumerate() {
                attributes[destination] = logical[logical_index];
            }
            attributes
        },
    )
}

fn build_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    attribute_arity: usize,
    attributes: impl Fn(usize, usize, i64, usize) -> Vec<i64>,
) -> RawRelationObservationTrace {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = BTreeMap::new();
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
                let primitive_attributes = attributes(component, member, key, component_count);
                assert_eq!(primitive_attributes.len(), attribute_arity);
                primitives.insert(
                    id,
                    RawAttributedObservationPrimitive {
                        observation,
                        attributes: primitive_attributes,
                    },
                );
            }
            component += 1;
        }
    }
    RawRelationObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        primitives,
        outputs,
    }
}
