use cairn_field::{EntityFrameLearner, FiniteField};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;

fn family(square: usize, color: i64, shift: usize) -> (FiniteField, FiniteField) {
    let rows = 12_usize;
    let columns = 14_usize;
    let mut input = FiniteField {
        shape: vec![12, 14],
        values: vec![0; rows * columns],
    };
    let top = 1 + shift;
    let left = 1;
    for row in 0..square {
        for column in 0..square {
            input.values[(top + row) * columns + left + column] = color;
        }
    }
    for row in 0..2 {
        for column in 0..4 {
            input.values[(8 + row) * columns + 2 + column] = color + 1;
        }
    }
    for row in 0..3 {
        for column in 0..2 {
            input.values[(4 + row) * columns + 10 + column] = color + 2;
        }
    }
    (
        input,
        FiniteField {
            shape: vec![u32::try_from(square).expect("small"); 2],
            values: vec![color; square * square],
        },
    )
}

#[test]
fn invents_entity_selector_and_transfers_to_new_size_color_and_layout() {
    let first = family(2, 3, 0);
    let second = family(3, 6, 1);
    let theory = EntityFrameLearner
        .learn(
            &[first, second],
            InventionLimits {
                maximum_cost: 7,
                maximum_candidates: 1_000_000,
                maximum_expression_nodes: 2_000,
                execution: ExecLimits::default(),
            },
        )
        .expect("invented selector");
    let (held_out_input, held_out_output) = family(4, 9, 0);
    assert_eq!(
        EntityFrameLearner
            .predict(&theory, &held_out_input, ExecLimits::default())
            .expect("selected held-out entity"),
        held_out_output
    );
    assert_eq!(theory.positive_frames, 2);
    assert_eq!(theory.negative_frames, 4);
    assert!(!format!("{:?}", theory.selector.definition.body).contains("square"));
}

#[test]
fn boundary_touching_entity_generation_is_bounded() {
    let mut input = FiniteField {
        shape: vec![4, 5],
        values: vec![0; 20],
    };
    input.values[0] = 3;
    input.values[1] = 3;
    let output = FiniteField {
        shape: vec![1, 2],
        values: vec![3, 3],
    };
    let error = EntityFrameLearner
        .learn(&[(input, output)], InventionLimits::default())
        .expect_err("one candidate has no contrast");
    assert_eq!(error, cairn_field::FieldError::NoEntity);
}
