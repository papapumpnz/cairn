use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_memory::{ModuleStatus, ObservationPredicateStore};
use cairn_rkl::ExecLimits;
use cairn_theory::{
    ObservationPredicateInventor, ObservationPredicateLimits, PredicateGroupingLimits,
    PrimitivePairEntity, PrimitivePairTrace,
};
use cairn_verifier::{ObservationPredicatePromotionConsolidator, ObservationPredicateVerifier};
use std::collections::BTreeSet;

#[test]
#[allow(clippy::too_many_lines)] // Covers promotion, support, reuse, and current replay together.
fn synthesized_feature_program_is_promoted_and_reused_as_a_proposal() {
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = source_cohorts();
    let learned = cohorts
        .iter()
        .map(|cohort| {
            ObservationPredicateInventor
                .invent(
                    cohort,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("independent feature-program invention")
        })
        .collect::<Vec<_>>();
    assert_eq!(learned[0].semantic_hash(), learned[1].semantic_hash());
    let fresh_work = learned[0]
        .feature_candidates_evaluated
        .saturating_add(u64::try_from(learned[0].feature_sets_evaluated).expect("bounded sets"))
        .saturating_add(
            u64::try_from(learned[0].grouping.relation_candidates_evaluated)
                .expect("bounded relations"),
        )
        .saturating_add(
            learned[0]
                .grouping
                .merge
                .constructor
                .proposal
                .candidates_evaluated,
        );
    let receipts = cohorts
        .iter()
        .map(|cohort| {
            ObservationPredicateVerifier
                .verify_reuse(
                    &learned[0],
                    cohort,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("independent reducing replay")
        })
        .collect::<Vec<_>>();
    let promotion = ObservationPredicatePromotionConsolidator
        .consolidate(&learned[0], &receipts)
        .expect("two disjoint receipts promote");
    let mut store = ObservationPredicateStore::new();
    let id = store
        .insert_provisional(learned[0].clone())
        .expect("provisional module");
    store
        .record_verified_reuse(id, receipts[0].clone())
        .expect("first receipt");
    assert!(!store.apply_verified_promotion(id, &promotion));
    store
        .record_verified_reuse(id, receipts[1].clone())
        .expect("second receipt");
    assert!(store.apply_verified_promotion(id, &promotion));
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );
    let support = store
        .module(id)
        .expect("module")
        .support()
        .expect("support");
    assert_eq!(support.observation_arity, 2);
    assert!(support.maximum_entities >= support.minimum_entities);
    let held_out = episode("feature-memory-heldout", &[2, 3, 4, 5, 6], -900, 5_000);
    let proposal = store
        .persistent_mechanisms()
        .next()
        .expect("persistent proposal");
    let predicted = ObservationPredicateInventor
        .predict(
            proposal,
            &held_out.entities,
            observation_limits,
            predicate_limits,
        )
        .expect("execute proposal on current evidence");
    assert_eq!(predicted, held_out.outputs);
    ObservationPredicateVerifier
        .verify(
            proposal,
            &[
                held_out.clone(),
                episode("feature-memory-heldout-2", &[3, 4, 5, 6, 7], 1_100, 6_000),
            ],
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("current evidence remains independently replayed");
    eprintln!(
        "FEATURE_PROGRAM_MEMORY receipts=2 feature_operation=add candidates={fresh_work}->1 examples=4->0 heldout_groups=5 exact=5 blank_proposals=0"
    );
}

fn source_cohorts() -> [[PrimitivePairTrace; 2]; 2] {
    [
        [
            episode("feature-memory-a1", &[2, 3, 4], 100, 1_000),
            episode("feature-memory-a2", &[2, 3, 4, 5], 300, 2_000),
        ],
        [
            episode("feature-memory-b1", &[2, 4, 5], -500, 3_000),
            episode("feature-memory-b2", &[3, 4, 5, 6], 700, 4_000),
        ],
    ]
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    sum_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    for (group_index, cardinality) in cardinalities.iter().copied().enumerate() {
        let sum = sum_offset + i64::try_from(group_index).expect("bounded group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            let member = i64::try_from(member).expect("bounded member");
            let first = sum_offset * 3 + member * 37 + 11;
            let second = sum - first;
            let observations = if (group_index + usize::try_from(member).expect("member")) % 2 == 0
            {
                vec![second, first]
            } else {
                vec![first, second]
            };
            entities.push(PrimitivePairEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + member,
                observations,
            });
        }
    }
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}
