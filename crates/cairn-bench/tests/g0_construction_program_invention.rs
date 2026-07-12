use cairn_core::Hash32;
use cairn_field::{EntityConstructionProgramFieldAdapter, FiniteField};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;
use cairn_theory::{
    CorrespondenceFrame, EntityConstructionProgramSynthesizer, EntityConstructionTrace,
    EntityCorrespondenceExample, EntityCorrespondenceLearner, EntityCorrespondenceLimits,
    EntityCorrespondenceMechanism, EntityGeometryRender, EntityGeometryScene, GeometricEntity,
    RenderedEntity,
};
use cairn_verifier::{
    EntityConstructionProgramVerifier, EntityConstructionProgramVerifyError,
    EntityCorrespondenceVerifier,
};
use std::collections::BTreeSet;

#[test]
fn verified_traces_become_a_source_independent_rkl_construction_program() {
    let training = [
        family(12, 15, 2, 3, 2),
        family(16, 18, 4, 20, 3),
        family(19, 23, 3, 45, 4),
        family(24, 20, 6, 65, 5),
    ];
    let teacher = EntityCorrespondenceMechanism {
        output_axis_sources: vec![0, 1],
        reversed_output_axes: BTreeSet::from([1]),
        frame: CorrespondenceFrame::TightCollection,
    };
    EntityCorrespondenceVerifier
        .verify(&teacher, &training, 65_536)
        .expect("independently verified source traces");
    let traces = training
        .iter()
        .map(|example| teacher_trace(&teacher, example))
        .collect::<Vec<_>>();
    let synthesized = EntityConstructionProgramSynthesizer
        .synthesize(&traces, invention_limits())
        .expect("program invention");
    let program_text = format!("{:?}", synthesized.program).to_lowercase();
    for forbidden in ["reflection", "rotate", "crop", "axis_map", "arc"] {
        assert!(!program_text.contains(forbidden));
    }

    // The old mechanism is no longer in the execution or verification path.
    drop(teacher);
    let held_out = [family(27, 29, 8, 90, 6), family(30, 26, 10, 130, 7)];
    let receipt = EntityConstructionProgramVerifier
        .verify(&synthesized.program, &held_out, ExecLimits::default())
        .expect("independent held-out program replay");
    assert_eq!(receipt.replayed_entities(), 13);
    assert!(receipt.replayed_points() > receipt.replayed_entities());
    for example in &held_out {
        assert_eq!(
            EntityConstructionProgramSynthesizer
                .execute(&synthesized.program, &example.source, ExecLimits::default())
                .expect("program-only execution")
                .render,
            example.expected
        );
    }

    let field = field_scene();
    assert_eq!(
        EntityConstructionProgramFieldAdapter
            .predict(&synthesized.program, &field, ExecLimits::default())
            .expect("cross-domain program"),
        reflected_tight_field(&field)
    );

    let mut ablated = synthesized.program.clone();
    ablated.output_coordinates.pop();
    assert!(EntityConstructionProgramSynthesizer
        .execute(&ablated, &held_out[0].source, ExecLimits::default())
        .is_err());

    let mut changed = held_out.clone();
    changed[1].expected.entities[0].value += 1;
    assert_eq!(
        EntityConstructionProgramVerifier.verify(
            &synthesized.program,
            &changed,
            ExecLimits::default()
        ),
        Err(EntityConstructionProgramVerifyError::ReplayMismatch)
    );
    eprintln!(
        "G0_CONSTRUCTION_PROGRAM traces={} point_examples={} synthesis_candidates={} heldout=2 field=1 v1_runtime=0 ablation=fail",
        synthesized.trace_count, synthesized.point_examples, synthesized.candidates_evaluated
    );
}

fn teacher_trace(
    teacher: &EntityCorrespondenceMechanism,
    example: &EntityCorrespondenceExample,
) -> EntityConstructionTrace {
    let construction = EntityCorrespondenceLearner
        .execute(
            teacher,
            &example.source,
            EntityCorrespondenceLimits::default(),
        )
        .expect("teacher execution");
    assert_eq!(construction.render, example.expected);
    EntityConstructionTrace {
        source: example.source.clone(),
        expected: example.expected.clone(),
        provenance: construction.provenance,
    }
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
            &[
                (shift + 3, shift + 4),
                (shift + 3, shift + 5),
                (shift + 4, shift + 5),
            ],
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
    let minimum_row = coordinates(&source, 0).min().expect("row");
    let maximum_row = coordinates(&source, 0).max().expect("row");
    let minimum_column = coordinates(&source, 1).min().expect("column");
    let maximum_column = coordinates(&source, 1).max().expect("column");
    let mut expected_entities = source
        .entities
        .iter()
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity
                .points
                .iter()
                .map(|point| vec![point[0] - minimum_row, maximum_column - point[1]])
                .collect(),
        })
        .collect::<Vec<_>>();
    expected_entities.sort_by(|left, right| {
        left.value
            .cmp(&right.value)
            .then_with(|| left.points.cmp(&right.points))
    });
    EntityCorrespondenceExample {
        source,
        expected: EntityGeometryRender {
            extents: vec![
                u32::try_from(maximum_row - minimum_row + 1).expect("rows"),
                u32::try_from(maximum_column - minimum_column + 1).expect("columns"),
            ],
            background: 0,
            entities: expected_entities,
        },
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

fn field_scene() -> FiniteField {
    let mut values = vec![0_i64; 12 * 15];
    for (value, points) in [
        (3, vec![(2, 2), (3, 2), (3, 3)]),
        (7, vec![(5, 7), (5, 8), (6, 8)]),
    ] {
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

fn invention_limits() -> InventionLimits {
    InventionLimits {
        maximum_cost: 7,
        maximum_candidates: 2_000_000,
        maximum_expression_nodes: 10_000,
        execution: ExecLimits::default(),
    }
}
