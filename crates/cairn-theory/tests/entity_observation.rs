use cairn_core::{EntityId, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    EntityObservationError, EntityObservationInventor, EntityObservationLimits,
    ObservationPredicateLimits, PredicateGroupingLimits, RawEntityObservationEvidence,
    RawEntityObservationTrace, RawObservationPrimitive,
};
use cairn_verifier::EntityObservationVerifier;
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // One causal test spans invention, transfer, replay, and ablation.
fn entity_interpretation_is_selected_only_through_executable_explanation() {
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        episode("entity-sequence-a", &[2, 3, 4], 100, false),
        episode("entity-table-a", &[2, 3, 4, 5], 300, true),
    ];
    let directly_lowered = source
        .iter()
        .map(|trace| {
            EntityObservationInventor
                .lower(
                    trace,
                    &coherent_role(trace),
                    entity_limits,
                    observation_limits,
                )
                .expect("coherent component interpretation")
        })
        .collect::<Vec<_>>();
    cairn_theory::ObservationPredicateInventor
        .invent(
            &directly_lowered,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("coherent interpretation supports an exact mechanism");
    let learned = EntityObservationInventor
        .invent(
            &source,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("joint entity and mechanism invention");
    assert_eq!(learned.binding_sets_evaluated, 4);
    for trace in &source {
        assert_eq!(
            learned
                .selected_relations
                .get(&trace.episode)
                .expect("episode binding"),
            &coherent_role(trace)
        );
    }
    let corroborating = [
        episode("entity-vector-b", &[3, 4, 5], -500, true),
        episode("entity-world-b", &[2, 3, 5, 6], 700, false),
    ];
    let independently_learned = EntityObservationInventor
        .invent(
            &corroborating,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("renamed cross-domain entity invention");
    assert_eq!(
        learned.mechanism_hash(),
        independently_learned.mechanism_hash()
    );
    assert_ne!(
        learned.selected_relations,
        independently_learned.selected_relations
    );

    let held_out = [
        episode("entity-graphics-heldout", &[2, 3, 4, 5], 1_100, true),
        episode("entity-control-heldout", &[2, 3, 4, 5, 6], -900, false),
    ];
    let rebound = EntityObservationInventor
        .invent(
            &held_out,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("held-out binding invention");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
    let receipt = EntityObservationVerifier
        .verify(
            &rebound,
            &held_out,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent union-find reconstruction and replay");
    assert_eq!(receipt.replayed_groups(), 9);
    for trace in &held_out {
        let relation = rebound
            .selected_relations
            .get(&trace.episode)
            .expect("held-out binding");
        assert_eq!(
            EntityObservationInventor
                .predict(
                    &learned.mechanism,
                    trace,
                    relation,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                )
                .expect("source-independent proposal"),
            trace.outputs
        );
    }
    let mut ablated = rebound.clone();
    for trace in &held_out {
        ablated
            .selected_relations
            .insert(trace.episode, distractor_role(trace));
    }
    assert!(EntityObservationVerifier
        .verify(
            &ablated,
            &held_out,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .is_err());
    eprintln!(
        "ENTITY_INTERPRETATION supplied_entities=0 relation_roles=2 binding_sets=4 source_domains=2 heldout_domains=2 heldout_groups=9 exact=9 binding_ablation_accepts=0"
    );
}

#[test]
fn entity_interpretation_ambiguity_and_capacity_fail_closed() {
    let source = [
        episode("entity-negative-a", &[2, 3, 4], 100, false),
        episode("entity-negative-b", &[2, 3, 4, 5], 300, true),
    ];
    assert_eq!(
        EntityObservationInventor
            .invent(
                &source,
                EntityObservationLimits {
                    maximum_binding_sets: 1,
                    ..EntityObservationLimits::default()
                },
                ObservationPredicateLimits::default(),
                PredicateGroupingLimits::default(),
                InventionLimits::default(),
            )
            .expect_err("binding cap must fail explicitly"),
        EntityObservationError::Capacity
    );
    let mut ambiguous = source.clone();
    for trace in &mut ambiguous {
        let coherent = coherent_role(trace);
        let alias = RelationRole(format!("{}:alias", trace_name(trace)));
        let duplicate = trace
            .evidence
            .relations
            .iter()
            .filter(|relation| relation.role == coherent)
            .cloned()
            .map(|mut relation| {
                relation.role = alias.clone();
                relation
            })
            .collect::<Vec<_>>();
        trace.evidence.relations.extend(duplicate);
    }
    assert_eq!(
        EntityObservationInventor
            .invent(
                &ambiguous,
                EntityObservationLimits::default(),
                ObservationPredicateLimits::default(),
                PredicateGroupingLimits::default(),
                InventionLimits::default(),
            )
            .expect_err("equivalent entity interpretations must remain ambiguous"),
        EntityObservationError::Ambiguous
    );
}

fn episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    reverse_relations: bool,
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
            let observations = [first, second, signature - first - second];
            let mut members = Vec::new();
            for (member, observation) in observations.into_iter().enumerate() {
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
    add_paths(&mut relations, &coherent, &components, reverse_relations);
    let wrong = (0..components.len())
        .map(|index| {
            vec![
                components[index][0].clone(),
                components[(index + 1) % components.len()][1].clone(),
                components[(index + 2) % components.len()][2].clone(),
            ]
        })
        .collect::<Vec<_>>();
    add_paths(&mut relations, &distractor, &wrong, !reverse_relations);
    if reverse_relations {
        relations.reverse();
    }
    RawEntityObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        evidence: RawEntityObservationEvidence {
            primitives,
            relations,
        },
        outputs,
    }
}

fn add_paths(
    relations: &mut Vec<RelationRecord>,
    role: &RelationRole,
    groups: &[Vec<EntityId>],
    reverse: bool,
) {
    for group in groups {
        for edge in group.windows(2) {
            let (subject, object) = if reverse {
                (edge[1].clone(), edge[0].clone())
            } else {
                (edge[0].clone(), edge[1].clone())
            };
            relations.push(RelationRecord {
                role: role.clone(),
                subject,
                object,
            });
        }
    }
}

fn coherent_role(trace: &RawEntityObservationTrace) -> RelationRole {
    RelationRole(format!("{}:coherent", trace_name(trace)))
}

fn distractor_role(trace: &RawEntityObservationTrace) -> RelationRole {
    RelationRole(format!("{}:distractor", trace_name(trace)))
}

fn trace_name(trace: &RawEntityObservationTrace) -> String {
    trace
        .evidence
        .relations
        .iter()
        .map(|relation| relation.role.0.split(':').next().unwrap_or_default())
        .find(|name| !name.is_empty())
        .unwrap_or_default()
        .to_owned()
}
