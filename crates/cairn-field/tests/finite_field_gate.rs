use cairn_field::{generated_source, FiniteField, PointwiseFieldLearner};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;

fn transformed(field: &FiniteField, delta: i64) -> FiniteField {
    FiniteField {
        shape: field.shape.clone(),
        values: field.values.iter().map(|value| value + delta).collect(),
    }
}

#[test]
fn omitted_cell_constructor_is_learned_and_generalizes_geometry_and_values() {
    let training = [
        FiniteField {
            shape: vec![2, 2],
            values: vec![0, 1, 3, 4],
        },
        FiniteField {
            shape: vec![1, 3],
            values: vec![-2, 5, 8],
        },
    ];
    let pairs = training
        .iter()
        .cloned()
        .map(|field| {
            let output = transformed(&field, 2);
            (field, output)
        })
        .collect::<Vec<_>>();
    let learned = PointwiseFieldLearner
        .learn(
            &pairs,
            InventionLimits {
                maximum_cost: 5,
                maximum_candidates: 100_000,
                maximum_expression_nodes: 10_000,
                execution: ExecLimits::default(),
            },
        )
        .expect("learned constructor");
    let held_out = FiniteField {
        shape: vec![3, 4],
        values: vec![9, -3, 7, 0, 4, 2, -1, 8, 6, 5, 3, 1],
    };
    let predicted = PointwiseFieldLearner
        .predict(&learned, &held_out, ExecLimits::default())
        .expect("held-out prediction");
    assert_eq!(predicted, transformed(&held_out, 2));
    assert_eq!(learned.training_cells, 7);
    held_out
        .observation(generated_source("held-out"))
        .expect("generic observation")
        .verify()
        .expect("observation identity");
}
