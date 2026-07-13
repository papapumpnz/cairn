use cairn_core::{EntityId, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use cairn_memory::{GroupingMergeStore, ModuleStatus};
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, GroupingMergeMemoryRuntimeError};
use cairn_theory::{
    RawGroupingError, RawGroupingEvidence, RawGroupingLimits, RawGroupingTrace,
    RawRelationGroupingInventor,
};
use cairn_verifier::{
    GroupingMergeVerifier, GroupingPromotionConsolidator, GroupingPromotionError,
};
use std::collections::BTreeMap;

#[test]
#[allow(clippy::too_many_lines)] // Preserve the raw-edge trust boundary and promotion gate together.
fn raw_relation_edges_become_a_persistent_grouping_mechanism() {
    let limits = RawGroupingLimits::default();
    let first = [
        episode("vector-raw", &[1, 4, 10]),
        episode("sequence-raw", &[2, 5, 11, 20]),
    ];
    let second = [
        episode("table-raw", &[3, 8, 16, 27, 41]),
        episode("workflow-raw", &[4, 9, 18, 31, 47, 66]),
    ];

    let lowered = RawRelationGroupingInventor
        .lower(&first[1].evidence, limits)
        .expect("raw lowering");
    assert_eq!(lowered.len(), 2);
    assert_eq!(
        lowered[&RelationRole("structural-membership".to_owned())].len(),
        3
    );
    assert!(lowered[&RelationRole("appearance-cluster".to_owned())]
        .windows(2)
        .all(|window| window[0] == window[1]));

    let mut reordered = first[1].evidence.clone();
    reordered.relations.reverse();
    for relation in &mut reordered.relations {
        std::mem::swap(&mut relation.subject, &mut relation.object);
    }
    assert_eq!(
        RawRelationGroupingInventor.lower(&reordered, limits),
        Ok(lowered)
    );

    let first_learned = RawRelationGroupingInventor
        .invent(&first, limits, InventionLimits::default())
        .expect("first raw invention");
    let second_learned = RawRelationGroupingInventor
        .invent(&second, limits, InventionLimits::default())
        .expect("second raw invention");
    assert_eq!(
        first_learned.relation,
        RelationRole("structural-membership".to_owned())
    );
    let id = first_learned.semantic_hash().expect("identity");
    assert_eq!(id, second_learned.semantic_hash().expect("same identity"));
    let blank_candidates = first_learned
        .merge
        .constructor
        .proposal
        .candidates_evaluated
        .saturating_add(u64::try_from(first_learned.relations_evaluated).expect("roles"));
    let first_receipt = GroupingMergeVerifier
        .verify_raw_reuse(
            &first_learned,
            &first,
            blank_candidates,
            1,
            limits,
            ExecLimits::default(),
        )
        .expect("first independent raw replay");
    let second_receipt = GroupingMergeVerifier
        .verify_raw_reuse(
            &second_learned,
            &second,
            blank_candidates,
            1,
            limits,
            ExecLimits::default(),
        )
        .expect("second independent raw replay");
    let certificate = GroupingPromotionConsolidator
        .consolidate(
            &first_learned,
            &[first_receipt.clone(), second_receipt.clone()],
        )
        .expect("raw promotion certificate");
    let mut store = GroupingMergeStore::new();
    assert_eq!(store.insert_provisional(first_learned.clone()), Ok(id));
    store
        .record_verified_reuse(id, first_receipt)
        .expect("first receipt");
    store
        .record_verified_reuse(id, second_receipt)
        .expect("second receipt");
    assert!(store.apply_verified_promotion(id, &certificate));
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );

    let held_out = [
        episode("graphics-raw-held-out", &[5, 11, 22, 36, 53, 73, 96]),
        episode(
            "operations-raw-held-out",
            &[6, 13, 25, 40, 58, 79, 103, 130],
        ),
    ];
    let runtime = CognitionRuntime::with_grouping_merge_memory(store);
    let outcome = runtime
        .propose_raw_grouping_merges_from_memory(&held_out[0].evidence, limits)
        .expect("raw memory proposal");
    assert_eq!(outcome.persistent_mechanisms_attempted, 1);
    assert_eq!(outcome.proposals, vec![held_out[0].outputs.clone()]);
    let held_out_receipt = GroupingMergeVerifier
        .verify_raw_reuse(
            &second_learned,
            &held_out,
            blank_candidates,
            1,
            limits,
            ExecLimits::default(),
        )
        .expect("held-out raw replay");
    assert_eq!(held_out_receipt.replayed_groups(), 13);
    assert!(matches!(
        CognitionRuntime::new()
            .propose_raw_grouping_merges_from_memory(&held_out[0].evidence, limits),
        Err(CognitionError::GroupingMergeMemory(
            GroupingMergeMemoryRuntimeError::NoVerifiedProposal
        ))
    ));

    let mut unknown = first[0].evidence.clone();
    unknown.relations.push(RelationRecord {
        role: RelationRole("structural-membership".to_owned()),
        subject: unknown.entities.keys().next().expect("entity").clone(),
        object: EntityId(Hash32::digest(b"unknown-entity")),
    });
    assert!(matches!(
        RawRelationGroupingInventor.lower(&unknown, limits),
        Err(RawGroupingError::UnknownEntity(_))
    ));
    let malformed_traces = [
        RawGroupingTrace {
            episode: first[0].episode,
            evidence: unknown,
            outputs: first[0].outputs.clone(),
        },
        first[1].clone(),
    ];
    assert_eq!(
        GroupingMergeVerifier.verify_raw_reuse(
            &second_learned,
            &malformed_traces,
            blank_candidates,
            1,
            limits,
            ExecLimits::default(),
        ),
        Err(GroupingPromotionError::InvalidRawEvidence)
    );
    let mut duplicate = first[0].evidence.clone();
    duplicate
        .relations
        .push(duplicate.relations.first().expect("relation").clone());
    assert_eq!(
        RawRelationGroupingInventor.lower(&duplicate, limits),
        Err(RawGroupingError::DuplicateRelation)
    );
    assert_eq!(
        RawRelationGroupingInventor.lower(
            &first[0].evidence,
            RawGroupingLimits {
                maximum_entities: first[0].evidence.entities.len() - 1,
                ..limits
            }
        ),
        Err(RawGroupingError::Capacity)
    );
    eprintln!(
        "RAW_GROUPING_METRICS raw_relations=2 selected_partitions_supplied=0 candidate_executions={blank_candidates}->1 examples=4->0 heldout_groups=13 exact=13 malformed_accepts=0"
    );
}

fn episode(domain: &str, levels: &[i64]) -> RawGroupingTrace {
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

    let mut entities = BTreeMap::new();
    let mut correct_groups = Vec::new();
    let mut distractor_groups = vec![Vec::new(); group_count];
    let mut occurrences = BTreeMap::<i64, usize>::new();
    for (group_index, values) in groups.iter().enumerate() {
        let mut ids = Vec::new();
        for (value_index, value) in values.iter().enumerate() {
            let id = EntityId(Hash32::digest(
                format!("{domain}:{group_index}:{value_index}").as_bytes(),
            ));
            entities.insert(id.clone(), *value);
            ids.push(id.clone());
            let occurrence = occurrences.entry(*value).or_default();
            distractor_groups[*occurrence].push(id);
            *occurrence += 1;
        }
        correct_groups.push(ids);
    }
    assert!(occurrences.values().all(|count| *count == group_count));

    let mut relations = Vec::new();
    add_chains(&mut relations, "structural-membership", &correct_groups);
    add_chains(&mut relations, "appearance-cluster", &distractor_groups);
    RawGroupingTrace {
        episode: Hash32::digest(domain.as_bytes()),
        evidence: RawGroupingEvidence {
            entities,
            relations,
        },
        outputs: levels
            .iter()
            .skip(1)
            .map(|maximum| maximum - levels[0])
            .collect(),
    }
}

fn add_chains(relations: &mut Vec<RelationRecord>, role: &str, groups: &[Vec<EntityId>]) {
    for group in groups {
        for edge in group.windows(2) {
            relations.push(RelationRecord {
                role: RelationRole(role.to_owned()),
                subject: edge[0].clone(),
                object: edge[1].clone(),
            });
        }
    }
}
