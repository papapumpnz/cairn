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
    assert_eq!(first_learned.relation_candidates_evaluated, 2);
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
        "PREDICATE_GROUPING_METRICS pair_labels=0 relation_edges=0 partitions=0 relation_candidates=2 fresh_candidates={fresh_candidates} reuse_attempts=1 examples=4->0 heldout_groups=13 exact=13 ablation_accepts=0"
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
