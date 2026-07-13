use cairn_core::{EntityId, Hash32, RelationRecord, RelationRole};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_sequence::{SequenceAdapter, SequencePrimitiveObservation};
use cairn_theory::{
    EntityObservationInventor, EntityObservationLimits, ObservationPredicateLimits,
    PredicateGroupingLimits, RawEntityObservationTrace,
};
use cairn_vector::{VectorAdapter, VectorPrimitiveObservation};
use cairn_verifier::EntityObservationVerifier;
use std::collections::BTreeSet;

#[test]
fn one_entity_invention_mechanism_rebinds_from_sequence_to_vector_evidence() {
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let source = [
        episode(Frontend::Sequence, "adapter-sequence-a", &[2, 3, 4], 100),
        episode(Frontend::Sequence, "adapter-sequence-b", &[2, 3, 4, 5], 300),
    ];
    let learned = EntityObservationInventor
        .invent(
            &source,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("sequence entity interpretation invention");
    let held_out = [
        episode(Frontend::Vector, "adapter-vector-a", &[2, 3, 4, 5], -500),
        episode(Frontend::Vector, "adapter-vector-b", &[2, 3, 4, 5, 6], 700),
    ];
    let rebound = EntityObservationInventor
        .invent(
            &held_out,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("vector entity interpretation invention");
    assert_eq!(learned.mechanism_hash(), rebound.mechanism_hash());
    let verified = EntityObservationVerifier
        .verify(
            &rebound,
            &held_out,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent vector reconstruction");
    assert_eq!(verified.replayed_groups(), 9);
    eprintln!(
        "G5_ENTITY_INTERPRETATION source=sequence target=vector supplied_entities=0 core_changes=0 relation_roles_per_episode=2 heldout_groups=9 exact=9"
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
) -> RawEntityObservationTrace {
    let coherent = RelationRole(format!("{domain}:coherent"));
    let distractor = RelationRole(format!("{domain}:distractor"));
    let mut primitives = Vec::new();
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
                primitives.push((id.clone(), observation));
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
    let evidence = match frontend {
        Frontend::Sequence => SequenceAdapter.lower_entity_candidates(
            &primitives
                .iter()
                .map(|(id, observation)| SequencePrimitiveObservation {
                    id: id.clone(),
                    observation: *observation,
                })
                .collect::<Vec<_>>(),
            &relations,
        ),
        Frontend::Vector => VectorAdapter.lower_entity_candidates(
            &primitives
                .iter()
                .map(|(id, observation)| VectorPrimitiveObservation {
                    id: id.clone(),
                    observation: *observation,
                })
                .collect::<Vec<_>>(),
            &relations,
        ),
    };
    RawEntityObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        evidence,
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
