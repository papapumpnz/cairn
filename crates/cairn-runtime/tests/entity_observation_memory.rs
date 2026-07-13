use cairn_core::{EntityId, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use cairn_memory::ObservationPredicateStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, EntityObservationMemoryRuntimeError};
use cairn_theory::{
    EntityObservationInventor, EntityObservationLimits, LearnedEntityObservationGrouping,
    ObservationPredicateLimits, PredicateGroupingLimits, RawEntityObservationEvidence,
    RawEntityObservationTrace, RawObservationPrimitive,
};
use cairn_verifier::{
    EntityObservationVerifier, ObservationPredicatePromotionConsolidator,
    ObservationPredicateVerifier,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Covers learning, promotion, rebinding, and acceptance boundaries.
fn runtime_rebinds_entity_interpretations_without_relearning_feature_structure() {
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let cohorts = [
        [
            episode("memory-entity-sequence-a", &[2, 3, 4], 100),
            episode("memory-entity-table-a", &[2, 3, 4, 5], 300),
        ],
        [
            episode("memory-entity-vector-b", &[3, 4, 5], -500),
            episode("memory-entity-world-b", &[2, 3, 5, 6], 700),
        ],
    ];
    let learned = cohorts
        .iter()
        .map(|cohort| {
            EntityObservationInventor
                .invent(
                    cohort,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("independent joint invention")
        })
        .collect::<Vec<_>>();
    assert_eq!(learned[0].mechanism_hash(), learned[1].mechanism_hash());
    let lowered = cohorts
        .iter()
        .zip(&learned)
        .map(|(cohort, mechanism)| {
            cohort
                .iter()
                .map(|trace| {
                    EntityObservationInventor
                        .lower(
                            trace,
                            mechanism
                                .selected_relations
                                .get(&trace.episode)
                                .expect("binding"),
                            entity_limits,
                            observation_limits,
                        )
                        .expect("selected lowering")
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(&lowered)
        .map(|(mechanism, traces)| {
            ObservationPredicateVerifier
                .verify_reuse(
                    &mechanism.mechanism,
                    traces,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("independent reducing receipt")
        })
        .collect::<Vec<_>>();
    let fresh_feature_work = receipts[0].candidate_executions_without();
    let promotion = ObservationPredicatePromotionConsolidator
        .consolidate(&learned[0].mechanism, &receipts)
        .expect("promotion certificate");
    let mut store = ObservationPredicateStore::new();
    let id = store
        .insert_provisional(learned[0].mechanism.clone())
        .expect("provisional mechanism");
    for receipt in receipts {
        store
            .record_verified_reuse(id, receipt)
            .expect("disjoint receipt");
    }
    assert!(store.apply_verified_promotion(id, &promotion));

    let held_out = [
        episode("memory-entity-graphics", &[2, 3, 4, 5], 1_100),
        episode("memory-entity-control", &[2, 3, 4, 5, 6], -900),
    ];
    assert_eq!(
        CognitionRuntime::new().propose_entity_observations_from_memory(
            &held_out[0],
            entity_limits,
            observation_limits,
            predicate_limits,
        ),
        Err(CognitionError::EntityObservationMemory(
            EntityObservationMemoryRuntimeError::NoVerifiedProposal
        ))
    );
    let runtime = CognitionRuntime::with_observation_predicate_memory(store);
    let mut bindings = BTreeMap::new();
    for trace in &held_out {
        let outcome = runtime
            .propose_entity_observations_from_memory(
                trace,
                entity_limits,
                observation_limits,
                predicate_limits,
            )
            .expect("memory-assisted entity proposals");
        assert_eq!(outcome.persistent_mechanisms_considered, 1);
        assert_eq!(outcome.entity_bindings_attempted, 2);
        let coherent = coherent_role(trace);
        let accepted_candidate = outcome
            .proposals
            .iter()
            .find(|proposal| proposal.relation == coherent)
            .expect("coherent binding proposal");
        assert_eq!(accepted_candidate.output, trace.outputs);
        bindings.insert(trace.episode, coherent);
    }
    let rebound = LearnedEntityObservationGrouping {
        selected_relations: bindings,
        mechanism: runtime
            .observation_predicate_memory()
            .persistent_mechanisms()
            .next()
            .expect("persistent mechanism")
            .clone(),
        binding_sets_evaluated: 4,
    };
    EntityObservationVerifier
        .verify(
            &rebound,
            &held_out,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent current-evidence acceptance");
    eprintln!(
        "ENTITY_MEMORY feature_candidates={fresh_feature_work}->0 source_examples=4->0 current_relation_bindings=2 heldout_groups=9 exact=9 blank_proposals=0"
    );
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
) -> RawEntityObservationTrace {
    let coherent = RelationRole(format!("{domain}:coherent"));
    let distractor = RelationRole(format!("{domain}:distractor"));
    let mut primitives = BTreeMap::new();
    let mut components = Vec::new();
    let mut outputs = BTreeSet::new();
    let mut component_index = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first =
                signature_offset * 3 + i64::try_from(component_index).expect("component") * 37 + 11;
            let second =
                signature_offset * -2 + i64::try_from(component_index).expect("component") * 19 - 7;
            let values = [first, second, signature - first - second];
            let mut members = Vec::new();
            for (member, observation) in values.into_iter().enumerate() {
                let id = EntityId(Hash32::digest(
                    format!("{domain}:{component_index}:{member}").as_bytes(),
                ));
                primitives.insert(id.clone(), RawObservationPrimitive { observation });
                members.push(id);
            }
            components.push(members);
            component_index += 1;
        }
    }
    let mut relations = Vec::new();
    add_paths(&mut relations, &coherent, &components);
    let wrong = (0..components.len())
        .map(|index| {
            vec![
                components[index][0].clone(),
                components[(index + 1) % components.len()][1].clone(),
                components[(index + 2) % components.len()][2].clone(),
            ]
        })
        .collect::<Vec<_>>();
    add_paths(&mut relations, &distractor, &wrong);
    RawEntityObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        evidence: RawEntityObservationEvidence {
            primitives,
            relations,
        },
        outputs,
    }
}

fn add_paths(relations: &mut Vec<RelationRecord>, role: &RelationRole, groups: &[Vec<EntityId>]) {
    for group in groups {
        for edge in group.windows(2) {
            relations.push(RelationRecord {
                role: role.clone(),
                subject: edge[0].clone(),
                object: edge[1].clone(),
            });
        }
    }
}

fn coherent_role(trace: &RawEntityObservationTrace) -> RelationRole {
    let domain = trace
        .evidence
        .relations
        .first()
        .and_then(|relation| relation.role.0.split(':').next())
        .unwrap_or_default();
    RelationRole(format!("{domain}:coherent"))
}
