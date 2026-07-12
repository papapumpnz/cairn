use cairn_core::Hash32;
use cairn_field::{EntityCorrespondenceFieldAdapter, FiniteField};
use cairn_memory::{EntityCorrespondencePromotion, EntityCorrespondenceStore, ModuleStatus};
use cairn_runtime::{CognitionError, CognitionRuntime, EntityCorrespondenceMemoryRuntimeError};
use cairn_theory::{
    EntityCorrespondenceExample, EntityCorrespondenceLimits, EntityGeometryRender,
    EntityGeometryScene, GeometricEntity, RenderedEntity,
};
use cairn_vector::VectorAdapter;
use cairn_verifier::{EntityCorrespondenceVerifier, EntityCorrespondenceVerifyError};
use std::collections::BTreeSet;

#[test]
fn persistent_entity_correspondence_reduces_work_and_crosses_frontends() {
    let first = [family(12, 15, 2, 3, 2), family(16, 13, 4, 20, 3)];
    let second = [family(18, 20, 5, 40, 3), family(21, 17, 7, 60, 4)];
    let learned = VectorAdapter
        .learn_entity_correspondence(&first, EntityCorrespondenceLimits::default())
        .expect("source learning");
    let independently_learned = VectorAdapter
        .learn_entity_correspondence(&second, EntityCorrespondenceLimits::default())
        .expect("independent source learning");
    assert_eq!(independently_learned.mechanism, learned.mechanism);
    let mut store = EntityCorrespondenceStore::new();
    let first_receipt = EntityCorrespondenceVerifier
        .verify(&learned.mechanism, &first, 1_000)
        .expect("first independent replay");
    let second_receipt = EntityCorrespondenceVerifier
        .verify(&learned.mechanism, &second, 1_000)
        .expect("second independent replay");
    assert_eq!(
        store
            .record_verified_cohort(&learned.mechanism, first_receipt)
            .expect("first cohort"),
        EntityCorrespondencePromotion::Provisional {
            verified_cohorts: 1
        }
    );
    assert_eq!(
        store
            .record_verified_cohort(&learned.mechanism, second_receipt)
            .expect("second cohort"),
        EntityCorrespondencePromotion::Persistent {
            verified_cohorts: 2
        }
    );
    let id = learned.mechanism.semantic_hash().expect("identity");
    let module = store.module(id).expect("module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.verified_experiences(), 4);
    assert_eq!(module.support().observed_extents.len(), 4);
    assert_eq!(
        module.support().observed_entity_counts,
        BTreeSet::from([2, 3, 4])
    );

    let held_out = [family(23, 25, 8, 80, 4), family(26, 22, 9, 100, 5)];
    let fresh = VectorAdapter
        .learn_entity_correspondence(&held_out, EntityCorrespondenceLimits::default())
        .expect("fresh held-out learning");
    let runtime = CognitionRuntime::with_entity_correspondence_memory(store);
    let mut memory_candidates = 0_u64;
    for example in &held_out {
        let outcome = runtime
            .propose_entity_constructions_from_memory(
                &example.source,
                EntityCorrespondenceLimits::default(),
            )
            .expect("memory proposal");
        memory_candidates += outcome.persistent_mechanisms_attempted;
        assert_eq!(outcome.proposals.len(), 1);
        assert_eq!(outcome.proposals[0].construction.render, example.expected);
    }
    EntityCorrespondenceVerifier
        .verify(&learned.mechanism, &held_out, 1_000)
        .expect("held-out verification");
    assert!(u64::try_from(fresh.candidates_evaluated).expect("small") >= memory_candidates * 4);
    assert_eq!(fresh.examples_required, 2);

    let field = field_scene();
    let predicted = EntityCorrespondenceFieldAdapter
        .predict(
            &learned.mechanism,
            &field,
            EntityCorrespondenceLimits::default(),
        )
        .expect("field transfer");
    assert_eq!(predicted, reflected_tight_field(&field));

    assert!(matches!(
        CognitionRuntime::new().propose_entity_constructions_from_memory(
            &held_out[0].source,
            EntityCorrespondenceLimits::default()
        ),
        Err(CognitionError::EntityCorrespondenceMemory(
            EntityCorrespondenceMemoryRuntimeError::NoVerifiedProposal
        ))
    ));

    let wrong_family = [
        transpose_family(11, 15, 2, 120),
        transpose_family(14, 18, 3, 140),
    ];
    assert_eq!(
        EntityCorrespondenceVerifier.verify(&learned.mechanism, &wrong_family, 1_000),
        Err(EntityCorrespondenceVerifyError::ReplayMismatch)
    );
    eprintln!(
        "G5_ENTITY_CORRESPONDENCE cohorts=2 experiences=4 fresh_candidates={} memory_candidates={} fresh_examples=2 memory_examples=0 heldout=2 field=1 negative_accepts=0",
        fresh.candidates_evaluated, memory_candidates
    );
}

#[test]
fn changed_truth_and_duplicate_evidence_fail_closed() {
    let examples = [family(12, 15, 2, 3, 2), family(16, 13, 4, 20, 3)];
    let learned = VectorAdapter
        .learn_entity_correspondence(&examples, EntityCorrespondenceLimits::default())
        .expect("learned");
    let mut changed = examples.clone();
    changed[1].expected.entities[0].value += 1;
    assert_eq!(
        EntityCorrespondenceVerifier.verify(&learned.mechanism, &changed, 1_000),
        Err(EntityCorrespondenceVerifyError::ReplayMismatch)
    );
    assert_eq!(
        EntityCorrespondenceVerifier.verify(
            &learned.mechanism,
            &[examples[0].clone(), examples[0].clone()],
            1_000,
        ),
        Err(EntityCorrespondenceVerifyError::DuplicateExperience)
    );
}

fn family(
    rows: u32,
    columns: u32,
    shift: i64,
    value: i64,
    count: usize,
) -> EntityCorrespondenceExample {
    let mut entities = vec![
        entity(
            value,
            &[(shift, shift), (shift + 1, shift), (shift + 1, shift + 1)],
            &format!("hook:{rows}"),
        ),
        entity(
            value + 1,
            &[(shift + 3, shift + 4), (shift + 3, shift + 5)],
            &format!("bar:{columns}"),
        ),
    ];
    for index in 2..count {
        let offset = i64::try_from(index).expect("small");
        entities.push(entity(
            value + offset,
            &[(shift + 4 + offset, shift + offset)],
            &format!("extra:{rows}:{index}"),
        ));
    }
    let source = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities,
    };
    let minimum_row = all_coordinates(&source, 0).min().expect("row");
    let maximum_row = all_coordinates(&source, 0).max().expect("row");
    let minimum_column = all_coordinates(&source, 1).min().expect("column");
    let maximum_column = all_coordinates(&source, 1).max().expect("column");
    let expected = render(
        &source,
        vec![
            u32::try_from(maximum_row - minimum_row + 1).expect("extent"),
            u32::try_from(maximum_column - minimum_column + 1).expect("extent"),
        ],
        |point| vec![point[0] - minimum_row, maximum_column - point[1]],
    );
    EntityCorrespondenceExample { source, expected }
}

fn transpose_family(
    rows: u32,
    columns: u32,
    shift: i64,
    value: i64,
) -> EntityCorrespondenceExample {
    let source = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: vec![entity(
            value,
            &[
                (shift, shift + 1),
                (shift + 1, shift + 1),
                (shift + 2, shift + 2),
            ],
            &format!("transpose:{rows}"),
        )],
    };
    let expected = render(&source, vec![columns, rows], |point| {
        vec![point[1], point[0]]
    });
    EntityCorrespondenceExample { source, expected }
}

fn all_coordinates(scene: &EntityGeometryScene, axis: usize) -> impl Iterator<Item = i64> + '_ {
    scene
        .entities
        .iter()
        .flat_map(|entity| &entity.points)
        .map(move |point| point[axis])
}

fn render(
    source: &EntityGeometryScene,
    extents: Vec<u32>,
    transform: impl Fn(&Vec<i64>) -> Vec<i64>,
) -> EntityGeometryRender {
    let mut entities = source
        .entities
        .iter()
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity.points.iter().map(&transform).collect(),
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| {
        left.value
            .cmp(&right.value)
            .then_with(|| left.points.cmp(&right.points))
    });
    EntityGeometryRender {
        extents,
        background: source.background,
        entities,
    }
}

fn entity(value: i64, geometry: &[(i64, i64)], name: &str) -> GeometricEntity {
    GeometricEntity {
        identity: Hash32::digest(name.as_bytes()),
        value,
        points: geometry
            .iter()
            .map(|(row, column)| vec![*row, *column])
            .collect(),
    }
}

fn field_scene() -> FiniteField {
    let mut values = vec![0_i64; 12 * 15];
    for (value, points) in [(3, vec![(2, 2), (3, 2), (3, 3)]), (7, vec![(5, 7), (5, 8)])] {
        for (row, column) in points {
            values[row * 15 + column] = value;
        }
    }
    FiniteField {
        shape: vec![12, 15],
        values,
    }
}

fn reflected_tight_field(source: &FiniteField) -> FiniteField {
    let occupied = source
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (*value != 0).then_some((index / 15, index % 15, *value)))
        .collect::<Vec<_>>();
    let min_row = occupied.iter().map(|(row, _, _)| *row).min().expect("row");
    let max_row = occupied.iter().map(|(row, _, _)| *row).max().expect("row");
    let min_column = occupied
        .iter()
        .map(|(_, column, _)| *column)
        .min()
        .expect("column");
    let max_column = occupied
        .iter()
        .map(|(_, column, _)| *column)
        .max()
        .expect("column");
    let rows = max_row - min_row + 1;
    let columns = max_column - min_column + 1;
    let mut values = vec![0; rows * columns];
    for (row, column, value) in occupied {
        values[(row - min_row) * columns + (max_column - column)] = value;
    }
    FiniteField {
        shape: vec![
            u32::try_from(rows).expect("rows"),
            u32::try_from(columns).expect("columns"),
        ],
        values,
    }
}
