use cairn_core::{EntityId, Hash32};
use cairn_invent::InventionLimits;
use cairn_rkl::{BinaryOp, ExecLimits, Expr, VarName};
use cairn_theory::{
    AttributedGroupingEntity, AttributedGroupingTrace, PredicateGroupingInventor,
    PredicateGroupingLimits,
};
use cairn_verifier::{
    GroupMergeVerifyError, PredicateGroupingVerifier, PredicateGroupingVerifyError,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Keep joint invention, cross-domain replay, and ablation visible.
fn invents_pair_relation_and_collections_without_edges_or_pair_labels() {
    let limits = PredicateGroupingLimits::default();
    let first = [
        episode("vector-attributed", &[1, 4, 10], 100),
        episode("sequence-attributed", &[2, 5, 11, 20], 200),
    ];
    let second = [
        episode("table-attributed", &[3, 8, 16, 27, 41], -50),
        episode("workflow-attributed", &[4, 9, 18, 31, 47, 66], 700),
    ];
    let first_learned = PredicateGroupingInventor
        .invent(&first, limits, InventionLimits::default())
        .expect("first predicate/grouping theory");
    let second_learned = PredicateGroupingInventor
        .invent(&second, limits, InventionLimits::default())
        .expect("second predicate/grouping theory");
    assert_eq!(first_learned.relation_candidates_evaluated, 3);
    assert_eq!(
        first_learned.relation.body,
        Expr::Binary {
            op: BinaryOp::Equal,
            left: Box::new(Expr::Var(VarName("left_feature_0".to_owned()))),
            right: Box::new(Expr::Var(VarName("right_feature_0".to_owned()))),
        }
    );
    let id = first_learned.semantic_hash().expect("first identity");
    assert_eq!(id, second_learned.semantic_hash().expect("second identity"));
    let first_receipt = PredicateGroupingVerifier
        .verify(&first_learned, &first, limits, ExecLimits::default())
        .expect("first independent replay");
    let second_receipt = PredicateGroupingVerifier
        .verify(&second_learned, &second, limits, ExecLimits::default())
        .expect("second independent replay");
    assert_eq!(first_receipt.mechanism(), id);
    assert_eq!(first_receipt.replayed_groups(), 5);
    assert_eq!(second_receipt.replayed_groups(), 9);

    let held_out = [
        episode(
            "graphics-attributed-held-out",
            &[5, 11, 22, 36, 53, 73, 96],
            -900,
        ),
        episode(
            "operations-attributed-held-out",
            &[6, 13, 25, 40, 58, 79, 103, 130],
            2_000,
        ),
    ];
    let prediction = PredicateGroupingInventor
        .predict(&second_learned, &held_out[0].entities, limits)
        .expect("zero-example predicate reuse");
    assert_eq!(prediction, held_out[0].outputs);
    let held_out_receipt = PredicateGroupingVerifier
        .verify(&second_learned, &held_out, limits, ExecLimits::default())
        .expect("held-out independent replay");
    assert_eq!(held_out_receipt.replayed_groups(), 13);

    let reordered = held_out.clone().map(|mut trace| {
        trace.entities.reverse();
        trace
    });
    assert_eq!(
        PredicateGroupingVerifier
            .verify(&second_learned, &reordered, limits, ExecLimits::default(),)
            .expect("entity-order invariance")
            .mechanism(),
        id
    );

    let swapped = held_out.clone().map(|mut trace| {
        for entity in &mut trace.entities {
            entity.features.swap(0, 1);
        }
        trace
    });
    assert!(matches!(
        PredicateGroupingVerifier.verify(&second_learned, &swapped, limits, ExecLimits::default(),),
        Err(PredicateGroupingVerifyError::Merge(
            GroupMergeVerifyError::ReplayMismatch
        ))
    ));

    let mut ablated = second_learned;
    ablated.relation.body = Expr::Binary {
        op: BinaryOp::Equal,
        left: Box::new(Expr::Var(VarName("left_feature_1".to_owned()))),
        right: Box::new(Expr::Var(VarName("right_feature_1".to_owned()))),
    };
    assert_ne!(ablated.semantic_hash().expect("ablated identity"), id);
    assert!(PredicateGroupingVerifier
        .verify(&ablated, &held_out, limits, ExecLimits::default())
        .is_err());
    let fresh_candidates = first_learned
        .merge
        .constructor
        .proposal
        .candidates_evaluated
        .saturating_add(
            u64::try_from(first_learned.relation_candidates_evaluated).expect("candidates"),
        );
    eprintln!(
        "PREDICATE_GROUPING_METRICS pair_labels=0 relation_edges=0 partitions=0 relation_candidates=3 fresh_candidates={fresh_candidates} reuse_attempts=1 examples=4->0 heldout_groups=13 exact=13 ablation_accepts=0"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Keep invention, transfer, and causal ablation together.
fn invents_a_conjunction_when_every_atomic_relation_is_wrong() {
    let limits = PredicateGroupingLimits::default();
    let first = [
        conjunctive_episode("vector-conjunction-a", &[(0, 0), (0, 1), (1, 0)], 1_000),
        conjunctive_episode(
            "sequence-conjunction-a",
            &[(2, 2), (2, 3), (3, 2), (3, 3)],
            2_000,
        ),
    ];
    let second = [
        conjunctive_episode(
            "table-conjunction-b",
            &[(10, 20), (10, 21), (11, 20)],
            3_000,
        ),
        conjunctive_episode(
            "workflow-conjunction-b",
            &[(30, 40), (30, 41), (31, 40), (31, 41)],
            4_000,
        ),
    ];
    assert_eq!(
        PredicateGroupingInventor.invent(
            &first,
            PredicateGroupingLimits {
                maximum_conjuncts: 1,
                ..limits
            },
            InventionLimits::default(),
        ),
        Err(cairn_theory::PredicateGroupingError::NoExactTheory)
    );
    assert_eq!(
        PredicateGroupingInventor.invent(
            &first,
            PredicateGroupingLimits {
                maximum_relation_candidates: 2,
                ..limits
            },
            InventionLimits::default(),
        ),
        Err(cairn_theory::PredicateGroupingError::Capacity)
    );
    let first_learned = PredicateGroupingInventor
        .invent(&first, limits, InventionLimits::default())
        .expect("first conjunctive predicate");
    let second_learned = PredicateGroupingInventor
        .invent(&second, limits, InventionLimits::default())
        .expect("second conjunctive predicate");
    let expected = Expr::Binary {
        op: BinaryOp::And,
        left: Box::new(Expr::Binary {
            op: BinaryOp::Equal,
            left: Box::new(Expr::Var(VarName("left_feature_0".to_owned()))),
            right: Box::new(Expr::Var(VarName("right_feature_0".to_owned()))),
        }),
        right: Box::new(Expr::Binary {
            op: BinaryOp::Equal,
            left: Box::new(Expr::Var(VarName("left_feature_1".to_owned()))),
            right: Box::new(Expr::Var(VarName("right_feature_1".to_owned()))),
        }),
    };
    assert_eq!(first_learned.relation.body, expected);
    assert_eq!(first_learned.relation_candidates_evaluated, 6);
    let id = first_learned.semantic_hash().expect("first identity");
    assert_eq!(id, second_learned.semantic_hash().expect("second identity"));
    PredicateGroupingVerifier
        .verify(&first_learned, &first, limits, ExecLimits::default())
        .expect("first independent replay");
    PredicateGroupingVerifier
        .verify(&second_learned, &second, limits, ExecLimits::default())
        .expect("second independent replay");

    let held_out = [
        conjunctive_episode(
            "graphics-conjunction-held-out",
            &[(50, 60), (50, 61), (51, 60), (51, 61), (52, 60)],
            5_000,
        ),
        conjunctive_episode(
            "operations-conjunction-held-out",
            &[(70, 80), (70, 81), (71, 80), (71, 81), (72, 80), (72, 81)],
            6_000,
        ),
    ];
    let receipt = PredicateGroupingVerifier
        .verify(&second_learned, &held_out, limits, ExecLimits::default())
        .expect("changed held-out replay");
    assert_eq!(receipt.replayed_groups(), 11);
    assert_eq!(
        PredicateGroupingInventor
            .predict(&second_learned, &held_out[0].entities, limits)
            .expect("zero-example reuse"),
        held_out[0].outputs
    );

    let reordered = held_out.clone().map(|mut trace| {
        trace.entities.reverse();
        trace
    });
    assert_eq!(
        PredicateGroupingVerifier
            .verify(&second_learned, &reordered, limits, ExecLimits::default())
            .expect("order-independent conjunction")
            .mechanism(),
        id
    );
    let mut atomic_ablation = second_learned;
    atomic_ablation.relation.body = Expr::Binary {
        op: BinaryOp::Equal,
        left: Box::new(Expr::Var(VarName("left_feature_0".to_owned()))),
        right: Box::new(Expr::Var(VarName("right_feature_0".to_owned()))),
    };
    assert!(PredicateGroupingVerifier
        .verify(&atomic_ablation, &held_out, limits, ExecLimits::default())
        .is_err());
    eprintln!(
        "CONJUNCTIVE_PREDICATE_METRICS atomic_exact=0 conjunction_candidates=6 heldout_groups=11 exact=11 examples=4->0 ablation_accepts=0"
    );
}

#[test]
fn invents_interval_enclosure_from_opposing_inequalities() {
    let limits = PredicateGroupingLimits {
        include_order_comparisons: true,
        maximum_relation_candidates: 128,
        ..PredicateGroupingLimits::default()
    };
    let first = [
        interval_episode("vector-interval-a", &[2, 3, 4], 0, 1_000),
        interval_episode("sequence-interval-a", &[2, 3, 4, 5], 10, 2_000),
    ];
    let second = [
        interval_episode("table-interval-b", &[2, 4, 5], 20, 3_000),
        interval_episode("workflow-interval-b", &[2, 3, 5, 6], 30, 4_000),
    ];
    let learned = PredicateGroupingInventor
        .invent(&first, limits, InventionLimits::default())
        .expect("interval relation");
    let corroborating = PredicateGroupingInventor
        .invent(&second, limits, InventionLimits::default())
        .expect("corroborating interval relation");
    let expected = Expr::Binary {
        op: BinaryOp::And,
        left: Box::new(Expr::Binary {
            op: BinaryOp::LessThan,
            left: Box::new(Expr::Var(VarName("left_feature_0".to_owned()))),
            right: Box::new(Expr::Var(VarName("right_feature_0".to_owned()))),
        }),
        right: Box::new(Expr::Binary {
            op: BinaryOp::LessThan,
            left: Box::new(Expr::Var(VarName("right_feature_1".to_owned()))),
            right: Box::new(Expr::Var(VarName("left_feature_1".to_owned()))),
        }),
    };
    assert_eq!(learned.relation.body, expected);
    assert_eq!(learned.semantic_hash(), corroborating.semantic_hash());
    assert_eq!(learned.relation_candidates_evaluated, 31);
    let held_out = [
        interval_episode("graphics-interval-held-out", &[2, 3, 4, 5, 6], 40, 5_000),
        interval_episode(
            "operations-interval-held-out",
            &[2, 3, 4, 5, 6, 7],
            50,
            6_000,
        ),
    ];
    assert_eq!(
        PredicateGroupingVerifier
            .verify(&learned, &held_out, limits, ExecLimits::default())
            .expect("independent interval replay")
            .replayed_groups(),
        11
    );
    let reversed = held_out.clone().map(|mut trace| {
        trace.entities.reverse();
        trace
    });
    PredicateGroupingVerifier
        .verify(&learned, &reversed, limits, ExecLimits::default())
        .expect("directional relation is pair-order invariant");
    eprintln!(
        "INTERVAL_RELATION_METRICS pair_labels=0 edges=0 named_geometry=0 candidates=31 heldout_groups=11 exact=11"
    );
}

fn episode(domain: &str, levels: &[i64], group_label_base: i64) -> AttributedGroupingTrace {
    let group_count = levels.len() - 1;
    let mut groups = Vec::new();
    for group_index in 0..group_count {
        let mut values = vec![levels[0], levels[1]];
        for level in levels.iter().take(group_index + 2).skip(2) {
            values.push(*level);
        }
        if group_index > 0 {
            values.extend(std::iter::repeat_n(levels[group_index + 1], group_index));
        }
        groups.push(values);
    }
    let mut occurrences = BTreeMap::<i64, usize>::new();
    let mut entities = Vec::new();
    for (group_index, values) in groups.iter().enumerate() {
        for (value_index, value) in values.iter().enumerate() {
            let occurrence = occurrences.entry(*value).or_default();
            entities.push(AttributedGroupingEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{value_index}").as_bytes(),
                )),
                value: *value,
                features: vec![
                    group_label_base + i64::try_from(group_index).expect("small group index") * 17,
                    i64::try_from(*occurrence).expect("small occurrence"),
                ],
            });
            *occurrence += 1;
        }
    }
    assert!(occurrences.values().all(|count| *count == group_count));
    AttributedGroupingTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs: levels
            .iter()
            .skip(1)
            .map(|maximum| maximum - levels[0])
            .collect::<BTreeSet<_>>(),
    }
}

fn conjunctive_episode(
    domain: &str,
    keys: &[(i64, i64)],
    value_base: i64,
) -> AttributedGroupingTrace {
    let mut entities = Vec::new();
    let mut outputs = BTreeSet::new();
    for (group_index, (first, second)) in keys.iter().copied().enumerate() {
        let cardinality = group_index + 2;
        outputs.insert(i64::try_from(cardinality - 1).expect("bounded cardinality"));
        for member in 0..cardinality {
            let value = value_base
                + i64::try_from(group_index).expect("bounded group") * 100
                + i64::try_from(member).expect("bounded member");
            entities.push(AttributedGroupingEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value,
                features: vec![
                    first,
                    second,
                    i64::try_from(member).expect("bounded member"),
                ],
            });
        }
    }
    AttributedGroupingTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}

fn interval_episode(
    domain: &str,
    cardinalities: &[usize],
    coordinate_offset: i64,
    value_base: i64,
) -> AttributedGroupingTrace {
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
            entities.push(AttributedGroupingEntity {
                id: EntityId(Hash32::digest(
                    format!("{domain}:{group_index}:{member}").as_bytes(),
                )),
                value: value_base
                    + i64::try_from(group_index).expect("bounded group") * 100
                    + member,
                features: vec![minimum, maximum],
            });
        }
    }
    AttributedGroupingTrace {
        episode: Hash32::digest(domain.as_bytes()),
        entities,
        outputs,
    }
}
