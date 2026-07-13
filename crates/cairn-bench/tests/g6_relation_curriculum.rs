use cairn_core::{EntityId, Hash32};
use cairn_curriculum::{
    AutonomousCurriculum, DeterminismStatus, DevelopmentalNeed, DevelopmentalOutcome,
    FailureDrivenObjectiveGenerator, LearningAxis, LearningFailureCore, RedactionStatus,
    SearchFailure, ValueClass, FAILURE_CORE_SEMANTICS_VERSION,
};
use cairn_invent::InventionLimits;
use cairn_memory::RawRelationStore;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationObservationInventor, RawRelationObservationTrace,
    RelationObservationLimits,
};
use cairn_verifier::{
    RawRelationObservationPromotionConsolidator, RawRelationObservationVerifier,
    VerifiedRawRelationObservationReuse,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Connects failure, curriculum, invention, promotion, and transfer.
fn structural_failure_drives_a_verified_relation_learning_curriculum() {
    let generated = FailureDrivenObjectiveGenerator
        .generate(&relation_failure_core(), &BTreeSet::new())
        .expect("content-free relation objective");
    assert_eq!(generated.need, DevelopmentalNeed::RelationalStructure);
    assert_eq!(generated.objective.axis, LearningAxis::Layout);
    let request = generated.exercise_request();
    assert_eq!(request.minimum_independent_variations, 2);
    let mut curriculum =
        AutonomousCurriculum::new(vec![generated.objective.clone()], 2).expect("curriculum");
    let decision = curriculum.select_next().expect("select relation gap");
    assert_eq!(decision.objective, generated.objective.id);

    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            episode("curriculum-rel-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
            episode("curriculum-rel-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            episode("curriculum-rel-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
            episode("curriculum-rel-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
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
                .expect("curriculum materialized relation experience")
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
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("verified curriculum outcome")
        })
        .collect::<Vec<_>>();
    let promotion = RawRelationObservationPromotionConsolidator
        .consolidate(&receipts)
        .expect("curriculum mechanism promotion");
    let mut store = RawRelationStore::new();
    let id = store
        .insert_provisional(
            learned[0]
                .source_independent()
                .expect("source-independent relation"),
        )
        .expect("provisional relation");
    for receipt in &receipts {
        store
            .record_verified_reuse(id, receipt.clone())
            .expect("curriculum receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));

    let held_out = [
        episode("curriculum-rel-graphics", &[2, 3, 4, 5], 1_100, [1, 0, 2]),
        episode("curriculum-rel-control", &[2, 3, 4, 5, 6], -900, [2, 0, 1]),
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
        .expect("held-out curriculum transfer");
    assert_eq!(rebound.mechanism_hash(), learned[0].mechanism_hash());
    let transfer = RawRelationObservationVerifier
        .verify(
            &rebound,
            &held_out,
            relation_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("held-out curriculum verification");
    assert_eq!(transfer.replayed_groups(), 9);

    curriculum
        .record(DevelopmentalOutcome {
            decision_sequence: decision.sequence,
            objective: decision.objective,
            mechanism_evidence: format!("verified:{}", id.to_hex()),
            independently_verified: true,
            held_out_transfer_cases: 9,
            candidate_work_without: receipts
                .iter()
                .map(VerifiedRawRelationObservationReuse::bindings_without)
                .sum(),
            candidate_work_with: receipts
                .iter()
                .map(VerifiedRawRelationObservationReuse::bindings_with)
                .sum(),
            examples_without: 4,
            examples_with: 0,
            retained_prior_tasks: 0,
            prior_tasks: 0,
            negative_transfers: 0,
        })
        .expect("admit independently verified outcome");
    let restored = AutonomousCurriculum::restore(&curriculum.checkpoint()).expect("restore");
    assert_eq!(restored.completed_objectives(), 1);
    assert_eq!(restored.mechanisms().len(), 1);
    eprintln!(
        "G6_RELATION_CURRICULUM generated=1 selected=1 independent_variations=2 relation_structures=5 bindings=60->12 examples=4->0 heldout_groups=9 exact=9 restart_retained=1"
    );
}

fn relation_failure_core() -> LearningFailureCore {
    LearningFailureCore {
        semantics_version: FAILURE_CORE_SEMANTICS_VERSION.to_owned(),
        failure: SearchFailure::NoDefinition { maximum_cost: 1 },
        parameter_types: vec![ValueClass::Entity, ValueClass::Relation],
        return_type: ValueClass::Relation,
        demonstrations: 4,
        varying_parameters: 2,
        distinct_output_classes: 3,
        constant_explanation_possible: false,
        direct_projection_candidates: 0,
        single_parameter_explanations: 0,
        exact_two_parameter_families: BTreeSet::new(),
        exact_three_parameter_families: BTreeSet::new(),
        determinism: DeterminismStatus::Deterministic,
        minimum_dependency_arity_lower_bound: 1,
        redaction: RedactionStatus::ContentFree,
    }
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 3],
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
                primitives.insert(
                    id,
                    RawAttributedObservationPrimitive {
                        observation,
                        attributes,
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
