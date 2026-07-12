use cairn_core::Hash32;
use cairn_theory::{
    CorrespondenceFrame, EntityCorrespondenceError, EntityCorrespondenceExample,
    EntityCorrespondenceLearner, EntityCorrespondenceLimits, EntityCorrespondenceMechanism,
    EntityGeometryRender, EntityGeometryScene, GeometricEntity, RenderedEntity,
};
use cairn_vector::VectorAdapter;
use std::collections::BTreeSet;

type BorrowedEntity<'a> = (i64, &'a [(i64, i64)], &'a str);
type OwnedEntity<'a> = (i64, Vec<(i64, i64)>, &'a str);

#[test]
fn learns_multi_entity_tight_reflection_and_complete_provenance() {
    let examples = [
        tight_reflection_family(12, 15, 2, 3, 2),
        tight_reflection_family(16, 13, 4, 20, 3),
    ];
    let learned = VectorAdapter
        .learn_entity_correspondence(&examples, EntityCorrespondenceLimits::default())
        .expect("learn correspondence");
    assert_eq!(learned.mechanism.output_axis_sources, vec![0, 1]);
    assert_eq!(learned.mechanism.reversed_output_axes, BTreeSet::from([1]));
    assert_eq!(
        learned.mechanism.frame,
        CorrespondenceFrame::TightCollection
    );
    let held_out = tight_reflection_family(19, 21, 6, 70, 4);
    let construction = EntityCorrespondenceLearner
        .execute(
            &learned.mechanism,
            &held_out.source,
            EntityCorrespondenceLimits::default(),
        )
        .expect("held-out construction");
    assert_eq!(construction.render, held_out.expected);
    assert_eq!(
        construction.provenance.len(),
        held_out.source.entities.len()
    );
    assert_eq!(
        construction
            .provenance
            .iter()
            .map(|item| item.points.len())
            .sum::<usize>(),
        held_out
            .source
            .entities
            .iter()
            .map(|entity| entity.points.len())
            .sum::<usize>()
    );
}

#[test]
fn learns_preserved_axis_permutation_across_changed_extents() {
    let examples = [
        preserved_transpose_family(8, 13, 1, 9),
        preserved_transpose_family(11, 16, 3, 30),
    ];
    let learned = VectorAdapter
        .learn_entity_correspondence(&examples, EntityCorrespondenceLimits::default())
        .expect("learn transpose");
    assert_eq!(learned.mechanism.output_axis_sources, vec![1, 0]);
    assert!(learned.mechanism.reversed_output_axes.is_empty());
    assert_eq!(
        learned.mechanism.frame,
        CorrespondenceFrame::PreserveObservation
    );
    let held_out = preserved_transpose_family(14, 19, 5, 80);
    assert_eq!(
        EntityCorrespondenceLearner
            .execute(
                &learned.mechanism,
                &held_out.source,
                EntityCorrespondenceLimits::default(),
            )
            .expect("held-out")
            .render,
        held_out.expected
    );
}

#[test]
fn ambiguity_collision_and_caps_fail_closed() {
    let symmetric = EntityCorrespondenceExample {
        source: scene(5, 5, vec![(1, &[(2, 2)], "s")]),
        expected: EntityGeometryRender {
            extents: vec![1, 1],
            background: 0,
            entities: vec![RenderedEntity {
                value: 1,
                points: points(&[(0, 0)]),
            }],
        },
    };
    assert_eq!(
        VectorAdapter
            .learn_entity_correspondence(&[symmetric], EntityCorrespondenceLimits::default()),
        Err(EntityCorrespondenceError::AmbiguousMechanism)
    );

    let examples = [
        tight_reflection_family(12, 15, 2, 3, 2),
        tight_reflection_family(16, 13, 4, 20, 3),
    ];
    assert_eq!(
        VectorAdapter.learn_entity_correspondence(
            &examples,
            EntityCorrespondenceLimits {
                maximum_candidates: 1,
                ..EntityCorrespondenceLimits::default()
            }
        ),
        Err(EntityCorrespondenceError::CandidateCapacity)
    );

    let collision_scene = EntityGeometryScene {
        extents: vec![3, 3],
        background: 0,
        entities: vec![
            GeometricEntity {
                identity: id("a"),
                value: 1,
                points: points(&[(1, 1)]),
            },
            GeometricEntity {
                identity: id("b"),
                value: 2,
                points: points(&[(1, 1)]),
            },
        ],
    };
    assert_eq!(
        EntityCorrespondenceLearner.execute(
            &EntityCorrespondenceMechanism {
                output_axis_sources: vec![0, 1],
                reversed_output_axes: BTreeSet::new(),
                frame: CorrespondenceFrame::PreserveObservation,
            },
            &collision_scene,
            EntityCorrespondenceLimits::default(),
        ),
        Err(EntityCorrespondenceError::ConstructionConflict)
    );
}

fn tight_reflection_family(
    rows: u32,
    columns: u32,
    shift: i64,
    value: i64,
    count: usize,
) -> EntityCorrespondenceExample {
    let mut entities = vec![
        (
            value,
            vec![(shift, shift), (shift + 1, shift), (shift + 1, shift + 1)],
            "hook",
        ),
        (
            value + 1,
            vec![(shift + 3, shift + 4), (shift + 3, shift + 5)],
            "bar",
        ),
    ];
    for index in 2..count {
        let offset = i64::try_from(index).expect("small");
        entities.push((
            value + offset,
            vec![(shift + 4 + offset, shift + offset)],
            "extra",
        ));
    }
    let source = scene_owned(rows, columns, &entities);
    let min_row = source
        .entities
        .iter()
        .flat_map(|e| &e.points)
        .map(|p| p[0])
        .min()
        .expect("point");
    let max_column = source
        .entities
        .iter()
        .flat_map(|e| &e.points)
        .map(|p| p[1])
        .max()
        .expect("point");
    let max_row = source
        .entities
        .iter()
        .flat_map(|e| &e.points)
        .map(|p| p[0])
        .max()
        .expect("point");
    let min_column = source
        .entities
        .iter()
        .flat_map(|e| &e.points)
        .map(|p| p[1])
        .min()
        .expect("point");
    let expected = render_from(
        &source,
        vec![
            u32::try_from(max_row - min_row + 1).expect("extent"),
            u32::try_from(max_column - min_column + 1).expect("extent"),
        ],
        |p| (p[0] - min_row, max_column - p[1]),
    );
    EntityCorrespondenceExample { source, expected }
}

fn preserved_transpose_family(
    rows: u32,
    columns: u32,
    shift: i64,
    value: i64,
) -> EntityCorrespondenceExample {
    let source = scene_owned(
        rows,
        columns,
        &[
            (
                value,
                vec![
                    (shift, shift + 1),
                    (shift + 1, shift + 1),
                    (shift + 2, shift + 2),
                ],
                "zig",
            ),
            (
                value + 3,
                vec![(shift + 3, shift + 5), (shift + 4, shift + 5)],
                "stem",
            ),
        ],
    );
    let expected = render_from(&source, vec![columns, rows], |p| (p[1], p[0]));
    EntityCorrespondenceExample { source, expected }
}

fn scene(rows: u32, columns: u32, entities: Vec<BorrowedEntity<'_>>) -> EntityGeometryScene {
    EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: entities
            .into_iter()
            .map(|(value, geometry, name)| GeometricEntity {
                identity: id(name),
                value,
                points: points(geometry),
            })
            .collect(),
    }
}

fn scene_owned(rows: u32, columns: u32, entities: &[OwnedEntity<'_>]) -> EntityGeometryScene {
    EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: entities
            .iter()
            .enumerate()
            .map(|(index, (value, geometry, name))| GeometricEntity {
                identity: id(&format!("{name}:{rows}:{columns}:{index}")),
                value: *value,
                points: points(geometry),
            })
            .collect(),
    }
}

fn render_from(
    source: &EntityGeometryScene,
    extents: Vec<u32>,
    transform: impl Fn(&Vec<i64>) -> (i64, i64),
) -> EntityGeometryRender {
    let mut entities = source
        .entities
        .iter()
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity
                .points
                .iter()
                .map(|point| {
                    let (row, column) = transform(point);
                    vec![row, column]
                })
                .collect(),
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

fn points(values: &[(i64, i64)]) -> BTreeSet<Vec<i64>> {
    values
        .iter()
        .map(|(row, column)| vec![*row, *column])
        .collect()
}

fn id(value: &str) -> Hash32 {
    Hash32::digest(value.as_bytes())
}
