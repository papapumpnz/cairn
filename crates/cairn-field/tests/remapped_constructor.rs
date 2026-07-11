use cairn_field::{FieldError, FiniteField, RemappedFieldLearner};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;

fn field(rows: u32, columns: u32, offset: i64) -> FiniteField {
    FiniteField {
        shape: vec![rows, columns],
        values: (0..i64::from(rows * columns))
            .map(|value| value + offset)
            .collect(),
    }
}

fn transpose(input: &FiniteField) -> FiniteField {
    let rows = usize::try_from(input.shape[0]).expect("small");
    let columns = usize::try_from(input.shape[1]).expect("small");
    let mut values = Vec::with_capacity(input.values.len());
    for output_row in 0..columns {
        for output_column in 0..rows {
            values.push(input.values[output_column * columns + output_row]);
        }
    }
    FiniteField {
        shape: vec![input.shape[1], input.shape[0]],
        values,
    }
}

fn embedded_pattern(
    rows: u32,
    columns: u32,
    top: u32,
    left: u32,
    content_rows: u32,
    content_columns: u32,
) -> (FiniteField, FiniteField) {
    let mut input = FiniteField {
        shape: vec![rows, columns],
        values: vec![0; usize::try_from(rows * columns).expect("small")],
    };
    let mut output = FiniteField {
        shape: vec![content_rows, content_columns],
        values: Vec::new(),
    };
    for row in 0..content_rows {
        for column in 0..content_columns {
            let value = 1 + i64::from((row + column) % 2);
            input.values[usize::try_from((top + row) * columns + left + column).expect("small")] =
                value;
            output.values.push(value);
        }
    }
    (input, output)
}

#[test]
fn invents_shape_and_coordinate_mapping_then_transfers_geometry() {
    let first = field(2, 3, 10);
    let second = field(3, 4, 100);
    let theory = RemappedFieldLearner
        .learn(
            &[
                (first.clone(), transpose(&first)),
                (second.clone(), transpose(&second)),
            ],
            InventionLimits {
                maximum_cost: 7,
                maximum_candidates: 100_000,
                maximum_expression_nodes: 1_000,
                execution: ExecLimits::default(),
            },
        )
        .expect("invented remapping");

    let held_out = field(4, 5, 1_000);
    assert_eq!(
        RemappedFieldLearner
            .predict(&theory, &held_out, ExecLimits::default())
            .expect("held-out prediction"),
        transpose(&held_out)
    );
    assert_eq!(theory.binding.training_correspondences, 18);
    assert!(theory.binding.output_extents[0]
        .definition
        .name
        .starts_with("invented_output_extent"));
}

#[test]
fn ambiguous_uniform_fields_fail_closed() {
    let input = FiniteField {
        shape: vec![3, 4],
        values: vec![1; 12],
    };
    let output = FiniteField {
        shape: vec![4, 3],
        values: vec![1; 12],
    };
    assert_eq!(
        RemappedFieldLearner.learn(&[(input, output)], InventionLimits::default(),),
        Err(FieldError::NoCorrespondence)
    );
}

#[test]
fn invents_content_relative_crop_despite_repeated_values() {
    let first = embedded_pattern(6, 7, 1, 2, 2, 3);
    let second = embedded_pattern(7, 8, 3, 1, 3, 2);
    let limits = InventionLimits {
        maximum_cost: 9,
        maximum_candidates: 10_000,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits::default(),
    };
    let theory = RemappedFieldLearner
        .learn(&[first, second], limits)
        .expect("content-relative constructor");
    let (held_out_input, held_out_output) = embedded_pattern(8, 9, 2, 3, 2, 4);
    assert_eq!(
        RemappedFieldLearner
            .predict(&theory, &held_out_input, ExecLimits::default())
            .expect("changed layout and cardinality"),
        held_out_output
    );
}
