use cairn_core::Hash32;
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    CorrespondenceFrame, EntityConstructionProgramError, EntityConstructionProgramSynthesizer,
    EntityConstructionTrace, EntityCorrespondenceLearner, EntityCorrespondenceLimits,
    EntityCorrespondenceMechanism, EntityGeometryRender, EntityGeometryScene, GeometricEntity,
    RenderedEntity,
};
use std::collections::BTreeSet;

#[test]
fn synthesizes_reflection_as_rkl_and_replays_without_v1_mechanism() {
    let training = [family(12, 15, 2, 3), family(16, 18, 4, 20)];
    let teacher = EntityCorrespondenceMechanism {
        output_axis_sources: vec![0, 1],
        reversed_output_axes: BTreeSet::from([1]),
        frame: CorrespondenceFrame::TightCollection,
    };
    let traces = training
        .iter()
        .map(|example| trace_from_teacher(&teacher, example))
        .collect::<Vec<_>>();
    let report = EntityConstructionProgramSynthesizer
        .synthesize(&traces, invention_limits())
        .expect("synthesized generic program");
    let held_out = family(21, 24, 7, 80);
    let construction = EntityConstructionProgramSynthesizer
        .execute(&report.program, &held_out.source, ExecLimits::default())
        .expect("program-only held-out execution");
    assert_eq!(construction.render, held_out.expected);
    assert_eq!(
        construction.provenance.len(),
        held_out.source.entities.len()
    );
    let program = format!("{:?}", report.program);
    for forbidden in ["reflection", "rotate", "crop", "axis_map", "arc"] {
        assert!(!program.to_lowercase().contains(forbidden));
    }
    assert_eq!(report.trace_count, 2);
    assert!(report.point_examples >= 10);
}

#[test]
fn same_synthesizer_invents_preserved_axis_permutation() {
    let training = [
        transpose_family(9, 14, 1, 5),
        transpose_family(13, 17, 3, 40),
    ];
    let teacher = EntityCorrespondenceMechanism {
        output_axis_sources: vec![1, 0],
        reversed_output_axes: BTreeSet::new(),
        frame: CorrespondenceFrame::PreserveObservation,
    };
    let traces = training
        .iter()
        .map(|example| trace_from_teacher(&teacher, example))
        .collect::<Vec<_>>();
    let report = EntityConstructionProgramSynthesizer
        .synthesize(&traces, invention_limits())
        .expect("synthesized transpose program");
    let held_out = transpose_family(18, 23, 5, 90);
    assert_eq!(
        EntityConstructionProgramSynthesizer
            .execute(&report.program, &held_out.source, ExecLimits::default())
            .expect("program-only transpose")
            .render,
        held_out.expected
    );
}

#[test]
fn incomplete_provenance_is_not_training_data() {
    let example = family(12, 15, 2, 3);
    let teacher = EntityCorrespondenceMechanism {
        output_axis_sources: vec![0, 1],
        reversed_output_axes: BTreeSet::from([1]),
        frame: CorrespondenceFrame::TightCollection,
    };
    let mut trace = trace_from_teacher(&teacher, &example);
    trace.provenance[0].points.pop();
    assert_eq!(
        EntityConstructionProgramSynthesizer.synthesize(&[trace], invention_limits()),
        Err(EntityConstructionProgramError::IncompleteProvenance)
    );
}

fn trace_from_teacher(
    teacher: &EntityCorrespondenceMechanism,
    example: &Example,
) -> EntityConstructionTrace {
    let construction = EntityCorrespondenceLearner
        .execute(
            teacher,
            &example.source,
            EntityCorrespondenceLimits::default(),
        )
        .expect("verified teacher trace");
    assert_eq!(construction.render, example.expected);
    EntityConstructionTrace {
        source: example.source.clone(),
        expected: example.expected.clone(),
        provenance: construction.provenance,
    }
}

#[derive(Clone)]
struct Example {
    source: EntityGeometryScene,
    expected: EntityGeometryRender,
}

fn family(rows: u32, columns: u32, shift: i64, value: i64) -> Example {
    let source = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: vec![
            entity(
                value,
                &[(shift, shift), (shift + 1, shift), (shift + 1, shift + 1)],
                &format!("a:{rows}"),
            ),
            entity(
                value + 1,
                &[
                    (shift + 3, shift + 4),
                    (shift + 3, shift + 5),
                    (shift + 4, shift + 5),
                ],
                &format!("b:{columns}"),
            ),
        ],
    };
    let min_row = coordinates(&source, 0).min().expect("row");
    let max_row = coordinates(&source, 0).max().expect("row");
    let min_column = coordinates(&source, 1).min().expect("column");
    let max_column = coordinates(&source, 1).max().expect("column");
    let expected = render(
        &source,
        vec![
            u32::try_from(max_row - min_row + 1).expect("rows"),
            u32::try_from(max_column - min_column + 1).expect("columns"),
        ],
        |point| vec![point[0] - min_row, max_column - point[1]],
    );
    Example { source, expected }
}

fn transpose_family(rows: u32, columns: u32, shift: i64, value: i64) -> Example {
    let source = EntityGeometryScene {
        extents: vec![rows, columns],
        background: 0,
        entities: vec![
            entity(
                value,
                &[
                    (shift, shift + 1),
                    (shift + 1, shift + 1),
                    (shift + 2, shift + 2),
                ],
                &format!("x:{rows}"),
            ),
            entity(
                value + 2,
                &[(shift + 3, shift + 5), (shift + 4, shift + 5)],
                &format!("y:{columns}"),
            ),
        ],
    };
    let expected = render(&source, vec![columns, rows], |point| {
        vec![point[1], point[0]]
    });
    Example { source, expected }
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

fn coordinates(scene: &EntityGeometryScene, axis: usize) -> impl Iterator<Item = i64> + '_ {
    scene
        .entities
        .iter()
        .flat_map(|entity| &entity.points)
        .map(move |point| point[axis])
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

fn invention_limits() -> InventionLimits {
    InventionLimits {
        maximum_cost: 7,
        maximum_candidates: 2_000_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    }
}
