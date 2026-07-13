use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_rkl::{BinaryOp, ExecLimits, Expr};
use cairn_theory::{
    ObservationPredicateError, ObservationPredicateInventor, ObservationPredicateLimits,
    PredicateGroupingLimits, PrimitivePairEntity, PrimitivePairTrace,
};
use cairn_verifier::ObservationPredicateVerifier;
use std::collections::BTreeSet;

#[test]
fn derives_canonical_features_then_invents_enclosure() {
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits {
        maximum_relation_candidates: 128,
        ..PredicateGroupingLimits::default()
    };
    let first = [
        episode("primitive-vector-a", &[2, 3, 4], 0, 1_000),
        episode("primitive-sequence-a", &[2, 3, 4, 5], 10, 2_000),
    ];
    let second = [
        episode("primitive-table-b", &[2, 4, 5], 20, 3_000),
        episode("primitive-workflow-b", &[2, 3, 5, 6], 30, 4_000),
    ];
    let learned = ObservationPredicateInventor
        .invent(
            &first,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("observation-derived theory");
    let corroborating = ObservationPredicateInventor
        .invent(
            &second,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("corroborating observation-derived theory");
    assert_eq!(learned.semantic_hash(), corroborating.semantic_hash());
    assert!(learned.feature_candidates_evaluated > 2);
    assert!(learned.feature_sets_evaluated > 2);
    assert!(learned
        .feature_definitions
        .iter()
        .any(|definition| matches!(
            definition.body,
            Expr::Binary {
                op: BinaryOp::Minimum,
                ..
            }
        )));
    assert!(learned
        .feature_definitions
        .iter()
        .any(|definition| matches!(
            definition.body,
            Expr::Binary {
                op: BinaryOp::Maximum,
                ..
            }
        )));
    let held_out = [
        episode("primitive-graphics-held-out", &[2, 3, 4, 5, 6], 40, 5_000),
        episode(
            "primitive-operations-held-out",
            &[2, 3, 4, 5, 6, 7],
            50,
            6_000,
        ),
    ];
    let receipt = ObservationPredicateVerifier
        .verify(
            &learned,
            &held_out,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent observation and relation replay");
    assert_eq!(receipt.replayed_groups(), 11);
    assert_eq!(
        ObservationPredicateInventor
            .predict(
                &learned,
                &held_out[0].entities,
                observation_limits,
                predicate_limits,
            )
            .expect("held-out prediction"),
        held_out[0].outputs
    );
    let mut ablated = learned.clone();
    for (index, definition) in ablated.feature_definitions.iter_mut().enumerate() {
        definition.body = Expr::Var(cairn_rkl::VarName(format!("observation_{index}")));
    }
    assert!(ObservationPredicateVerifier
        .verify(
            &ablated,
            &held_out,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .is_err());
    eprintln!(
        "OBSERVATION_FEATURE_METRICS supplied_features=0 candidates={} sets={} learned_min_max=1 heldout_groups=11 exact=11 ablation_accepts=0",
        learned.feature_candidates_evaluated, learned.feature_sets_evaluated
    );
}

#[test]
fn synthesizes_addition_feature_absent_from_named_templates() {
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        sum_episode("sum-sequence-a", &[2, 3, 4], 100, 7_000),
        sum_episode("sum-table-a", &[2, 3, 4, 5], 300, 8_000),
    ];
    let learned = ObservationPredicateInventor
        .invent(
            &source,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("synthesize an omitted feature program");
    assert_eq!(learned.feature_definitions.len(), 1);
    assert!(matches!(
        learned.feature_definitions[0].body,
        Expr::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
    let corroborating = [
        sum_episode("sum-vector-b", &[2, 4, 5], -500, 9_000),
        sum_episode("sum-workflow-b", &[3, 4, 5, 6], 700, 10_000),
    ];
    let independently_learned = ObservationPredicateInventor
        .invent(
            &corroborating,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("independent addition-feature invention");
    assert_eq!(
        learned.semantic_hash(),
        independently_learned.semantic_hash()
    );
    let held_out = [
        sum_episode("sum-graphics-heldout", &[2, 3, 4, 5, 6], -900, 11_000),
        sum_episode(
            "sum-interactive-heldout",
            &[2, 3, 4, 5, 6, 7],
            1_100,
            12_000,
        ),
    ];
    let receipt = ObservationPredicateVerifier
        .verify(
            &learned,
            &held_out,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independently replay synthesized feature");
    assert_eq!(receipt.replayed_groups(), 11);
    let mut ablated = learned.clone();
    ablated.feature_definitions[0].body = Expr::Var(cairn_rkl::VarName("observation_0".to_owned()));
    assert!(ObservationPredicateVerifier
        .verify(
            &ablated,
            &held_out,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .is_err());
    eprintln!(
        "FEATURE_PROGRAM_SYNTHESIS supplied_templates=0 operation=add candidates={} sets={} heldout_groups=11 exact=11 ablation_accepts=0",
        learned.feature_candidates_evaluated, learned.feature_sets_evaluated
    );
}

#[test]
fn synthesizes_permutation_invariant_feature_over_three_observations() {
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        triple_sum_episode("triple-sequence-a", &[2, 3, 4], 100, 15_000),
        triple_sum_episode("triple-table-a", &[2, 3, 4, 5], 300, 16_000),
    ];
    let learned = ObservationPredicateInventor
        .invent(
            &source,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("invent variable-arity invariant feature");
    assert_eq!(learned.observation_arity, 3);
    assert!(learned
        .feature_definitions
        .iter()
        .all(|definition| definition.parameters.len() == 3));
    let held_out = [
        triple_sum_episode("triple-vector-heldout", &[2, 4, 5, 6], -500, 17_000),
        triple_sum_episode("triple-world-heldout", &[3, 4, 5, 6, 7], 700, 18_000),
    ];
    let receipt = ObservationPredicateVerifier
        .verify(
            &learned,
            &held_out,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("verify all six observation permutations");
    assert_eq!(receipt.replayed_groups(), 9);
    assert_eq!(
        ObservationPredicateInventor
            .predict(
                &learned,
                &held_out[0].entities,
                observation_limits,
                predicate_limits,
            )
            .expect("variable-arity prediction"),
        held_out[0].outputs
    );
    eprintln!(
        "VARIABLE_ARITY_FEATURE observations=3 permutations=6 supplied_templates=0 candidates={} sets={} heldout_groups=9 exact=9",
        learned.feature_candidates_evaluated, learned.feature_sets_evaluated
    );
}

#[test]
fn feature_program_search_honors_explicit_capacity() {
    let source = [
        sum_episode("capacity-a", &[2, 3], 100, 13_000),
        sum_episode("capacity-b", &[2, 3, 4], 200, 14_000),
    ];
    let error = ObservationPredicateInventor
        .invent(
            &source,
            ObservationPredicateLimits {
                maximum_feature_candidates: 1,
                ..ObservationPredicateLimits::default()
            },
            PredicateGroupingLimits::default(),
            InventionLimits::default(),
        )
        .expect_err("candidate cap must terminate explicitly");
    assert_eq!(error, ObservationPredicateError::Capacity);
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    coordinate_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    for (group_index, cardinality) in cardinalities.iter().copied().enumerate() {
        let root = coordinate_offset * 10_000;
        let group_base = root + i64::try_from(group_index).expect("bounded group") * 1_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            let member = i64::try_from(member).expect("bounded member");
            let (minimum, maximum) = if group_index == 0 && member == 0 {
                (root, root + 100)
            } else if group_index == 0 {
                (root + 20 + member, root + 40 + member)
            } else if group_index == 1 && member == 0 {
                (root - 100, root + 15)
            } else if group_index == 1 {
                (root - 90 + member, root + 10 - member)
            } else if member == 0 {
                (group_base, group_base + 900)
            } else {
                (group_base + member * 20, group_base + 900 - member * 17)
            };
            let observations = if (group_index + usize::try_from(member).expect("member")) % 2 == 0
            {
                vec![maximum, minimum]
            } else {
                vec![minimum, maximum]
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

fn sum_episode(
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

fn triple_sum_episode(
    domain: &str,
    cardinalities: &[usize],
    sum_offset: i64,
    value_base: i64,
) -> PrimitivePairTrace {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for (group_index, cardinality) in cardinalities.iter().copied().enumerate() {
        let sum = sum_offset + i64::try_from(group_index).expect("bounded group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            let member_i64 = i64::try_from(member).expect("bounded member");
            let first = sum_offset * 3 + member_i64 * 37 + 11;
            let second = sum_offset * -2 + member_i64 * 19 - 7;
            let values = [first, second, sum - first - second];
            let permutation = permutations[(group_index + member) % permutations.len()];
            entities.push(PrimitivePairEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + member_i64,
                observations: permutation.iter().map(|index| values[*index]).collect(),
            });
        }
    }
    PrimitivePairTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}
