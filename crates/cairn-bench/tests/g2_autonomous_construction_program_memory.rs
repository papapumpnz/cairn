use cairn_core::Hash32;
use cairn_field::{EntityConstructionProgramFieldAdapter, FiniteField};
use cairn_invent::InventionLimits;
use cairn_memory::{ConstructionProgramPromotion, ConstructionProgramStore, ModuleStatus};
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, ConstructionProgramMemoryRuntimeError};
use cairn_theory::{
    EntityConstructionProgramSynthesizer, EntityCorrespondenceExample, EntityGeometryRender,
    EntityGeometryScene, GeometricEntity, RenderedEntity,
};
use cairn_verifier::{EntityConstructionProgramVerifier, EntityConstructionProgramVerifyError};

#[test]
fn unordered_experience_promotes_a_v1_free_program_and_reduces_future_learning() {
    let first = [
        family(14, 17, 2, 3, 2),
        family(18, 21, 4, 20, 3),
        family(22, 25, 3, 45, 4),
        family(26, 23, 6, 65, 5),
    ];
    let second = [
        family(19, 24, 5, 100, 2),
        family(24, 28, 7, 130, 3),
        family(28, 26, 4, 160, 4),
        family(32, 30, 8, 190, 5),
    ];
    let first_program = EntityConstructionProgramSynthesizer
        .synthesize_unordered(&first, invention_limits())
        .expect("first autonomous cohort");
    let second_program = EntityConstructionProgramSynthesizer
        .synthesize_unordered(&second, invention_limits())
        .expect("second autonomous cohort");
    assert_eq!(
        first_program.program.semantic_hash().expect("first id"),
        second_program.program.semantic_hash().expect("second id")
    );
    let first_receipt = EntityConstructionProgramVerifier
        .verify(&first_program.program, &first, ExecLimits::default())
        .expect("first independent receipt");
    let second_receipt = EntityConstructionProgramVerifier
        .verify(&second_program.program, &second, ExecLimits::default())
        .expect("second independent receipt");
    let mut store = ConstructionProgramStore::new();
    assert_eq!(
        store
            .record_verified_cohort(&first_program.program, first_receipt)
            .expect("provisional"),
        ConstructionProgramPromotion::Provisional {
            verified_cohorts: 1
        }
    );
    assert_eq!(
        store
            .record_verified_cohort(&second_program.program, second_receipt)
            .expect("persistent"),
        ConstructionProgramPromotion::Persistent {
            verified_cohorts: 2
        }
    );
    let id = first_program.program.semantic_hash().expect("program id");
    let module = store.module(id).expect("module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.verified_experiences(), 8);
    assert_eq!(module.support().observed_extents.len(), 8);

    let held_out = [family(35, 34, 9, 230, 6), family(38, 36, 11, 270, 7)];
    let runtime = CognitionRuntime::with_construction_program_memory(store);
    let mut memory_attempts = 0_u64;
    for example in &held_out {
        let outcome = runtime
            .propose_construction_programs_from_memory(&example.source, ExecLimits::default())
            .expect("persistent program proposal");
        memory_attempts += outcome.persistent_programs_attempted;
        assert_eq!(outcome.proposals.len(), 1);
        assert_eq!(outcome.proposals[0].construction.render, example.expected);
    }
    EntityConstructionProgramVerifier
        .verify(&first_program.program, &held_out, ExecLimits::default())
        .expect("held-out independent replay");
    assert!(first_program.candidates_evaluated > memory_attempts * 100);
    assert_eq!(memory_attempts, 2);

    assert!(matches!(
        CognitionRuntime::new()
            .propose_construction_programs_from_memory(&held_out[0].source, ExecLimits::default()),
        Err(CognitionError::ConstructionProgramMemory(
            ConstructionProgramMemoryRuntimeError::NoVerifiedProposal
        ))
    ));

    let field = field_scene();
    assert_eq!(
        EntityConstructionProgramFieldAdapter
            .predict(&first_program.program, &field, ExecLimits::default())
            .expect("field program transfer"),
        reflected_tight_field(&field)
    );

    let wrong = [
        transpose_family(16, 21, 3, 310),
        transpose_family(20, 25, 5, 340),
    ];
    assert_eq!(
        EntityConstructionProgramVerifier.verify(
            &first_program.program,
            &wrong,
            ExecLimits::default()
        ),
        Err(EntityConstructionProgramVerifyError::ReplayMismatch)
    );
    eprintln!(
        "G2_AUTONOMOUS_PROGRAM cohorts=2 experiences=8 v1_traces=0 synthesis_candidates={} memory_attempts={} memory_examples=0 heldout=2 field=1 negative_accepts=0",
        first_program.candidates_evaluated, memory_attempts
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
    let min_row = coordinates(&source, 0).min().expect("row");
    let max_row = coordinates(&source, 0).max().expect("row");
    let min_column = coordinates(&source, 1).min().expect("column");
    let max_column = coordinates(&source, 1).max().expect("column");
    let mut entities = source
        .entities
        .iter()
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity
                .points
                .iter()
                .map(|point| vec![point[0] - min_row, max_column - point[1]])
                .collect(),
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| {
        left.value
            .cmp(&right.value)
            .then_with(|| left.points.cmp(&right.points))
    });
    EntityCorrespondenceExample {
        source,
        expected: EntityGeometryRender {
            extents: vec![
                u32::try_from(max_row - min_row + 1).expect("rows"),
                u32::try_from(max_column - min_column + 1).expect("columns"),
            ],
            background: 0,
            entities,
        },
    }
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
                value + 1,
                &[(shift + 4, shift + 6), (shift + 5, shift + 6)],
                &format!("y:{columns}"),
            ),
        ],
    };
    let mut entities = source
        .entities
        .iter()
        .map(|entity| RenderedEntity {
            value: entity.value,
            points: entity
                .points
                .iter()
                .map(|point| vec![point[1], point[0]])
                .collect(),
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| {
        left.value
            .cmp(&right.value)
            .then_with(|| left.points.cmp(&right.points))
    });
    EntityCorrespondenceExample {
        source,
        expected: EntityGeometryRender {
            extents: vec![columns, rows],
            background: 0,
            entities,
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
    let min_row = occupied.iter().map(|(r, _, _)| *r).min().expect("r");
    let max_row = occupied.iter().map(|(r, _, _)| *r).max().expect("r");
    let min_col = occupied.iter().map(|(_, c, _)| *c).min().expect("c");
    let max_col = occupied.iter().map(|(_, c, _)| *c).max().expect("c");
    let rows = max_row - min_row + 1;
    let columns = max_col - min_col + 1;
    let mut values = vec![0; rows * columns];
    for (row, column, value) in occupied {
        values[(row - min_row) * columns + (max_col - column)] = value;
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
