use cairn_field::{FiniteField, LocalFieldLearner};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;

fn field(rows: u32, columns: u32, marks: &[(usize, i64)]) -> FiniteField {
    let mut values = vec![0; usize::try_from(rows * columns).expect("small")];
    for (index, value) in marks {
        values[*index] = *value;
    }
    FiniteField {
        shape: vec![rows, columns],
        values,
    }
}

fn shift_right(input: &FiniteField) -> FiniteField {
    let rows = usize::try_from(input.shape[0]).expect("small");
    let columns = usize::try_from(input.shape[1]).expect("small");
    let mut values = vec![0; input.values.len()];
    for row in 0..rows {
        for column in 1..columns {
            values[row * columns + column] = input.values[row * columns + column - 1];
        }
    }
    FiniteField {
        shape: input.shape.clone(),
        values,
    }
}

fn visible_west(input: &FiniteField) -> FiniteField {
    let rows = usize::try_from(input.shape[0]).expect("small");
    let columns = usize::try_from(input.shape[1]).expect("small");
    let mut values = vec![0; input.values.len()];
    for row in 0..rows {
        let mut visible = 0;
        for column in 0..columns {
            values[row * columns + column] = visible;
            let current = input.values[row * columns + column];
            if current != 0 {
                visible = current;
            }
        }
    }
    FiniteField {
        shape: input.shape.clone(),
        values,
    }
}

fn row_occupancy(input: &FiniteField) -> FiniteField {
    let rows = usize::try_from(input.shape[0]).expect("small");
    let columns = usize::try_from(input.shape[1]).expect("small");
    let mut values = vec![0; input.values.len()];
    for row in 0..rows {
        let occupied = input.values[row * columns..(row + 1) * columns]
            .iter()
            .filter(|value| **value != 0)
            .count();
        for column in 0..columns {
            values[row * columns + column] = i64::try_from(occupied).expect("small");
        }
    }
    FiniteField {
        shape: input.shape.clone(),
        values,
    }
}

#[test]
fn invents_local_law_and_transfers_geometry_values_and_positions() {
    let first = field(3, 5, &[(6, 2), (8, 4)]);
    let second = field(4, 6, &[(2, 7), (19, 5)]);
    let theory = LocalFieldLearner
        .learn(
            &[
                (first.clone(), shift_right(&first)),
                (second.clone(), shift_right(&second)),
            ],
            InventionLimits {
                maximum_cost: 5,
                maximum_candidates: 10_000,
                maximum_expression_nodes: 1_000,
                execution: ExecLimits::default(),
            },
        )
        .expect("invented local transition");
    let held_out = field(5, 7, &[(0, 9), (17, 3), (33, 8)]);
    assert_eq!(
        LocalFieldLearner
            .predict(&theory, &held_out, ExecLimits::default())
            .expect("held-out transition"),
        shift_right(&held_out)
    );
    assert_eq!(
        format!("{:?}", theory.proposal.definition.body),
        "Var(VarName(\"west\"))"
    );
}

#[test]
fn invents_long_range_relational_propagation() {
    let first = field(3, 7, &[(1, 4), (11, 6)]);
    let second = field(4, 8, &[(9, 3), (26, 8)]);
    let limits = InventionLimits {
        maximum_cost: 4,
        maximum_candidates: 10_000,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits::default(),
    };
    let theory = LocalFieldLearner
        .learn(
            &[
                (first.clone(), visible_west(&first)),
                (second.clone(), visible_west(&second)),
            ],
            limits,
        )
        .expect("invented relational law");
    let held_out = field(5, 9, &[(3, 7), (37, 2)]);
    assert_eq!(
        LocalFieldLearner
            .predict(&theory, &held_out, ExecLimits::default())
            .expect("held-out relational prediction"),
        visible_west(&held_out)
    );
    assert_eq!(
        format!("{:?}", theory.proposal.definition.body),
        "Var(VarName(\"visible_west\"))"
    );
}

#[test]
fn escalates_when_basic_observations_are_not_causally_sufficient() {
    let first = field(5, 10, &[(0, 7), (20, 4), (29, 5)]);
    let second = field(5, 10, &[(0, 3), (9, 8), (20, 6)]);
    let pairs = [
        (first.clone(), row_occupancy(&first)),
        (second.clone(), row_occupancy(&second)),
    ];
    let limits = InventionLimits {
        maximum_cost: 3,
        maximum_candidates: 10_000,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits::default(),
    };
    let problem = LocalFieldLearner
        .learning_problem(&pairs, limits)
        .expect("relational problem");
    let row_count = cairn_rkl::VarName("row_occupied_count".to_owned());
    assert!(problem
        .demonstrations
        .iter()
        .all(|example| example.inputs.get(&row_count) == Some(&example.output)));
    let theory = LocalFieldLearner
        .learn(&pairs, limits)
        .expect("relational evidence resolves basic observation collision");
    let held_out = field(6, 10, &[(2, 9), (31, 2), (35, 4), (57, 7)]);
    assert_eq!(
        LocalFieldLearner
            .predict(&theory, &held_out, ExecLimits::default())
            .expect("held-out relational prediction"),
        row_occupancy(&held_out)
    );
    assert_eq!(
        format!("{:?}", theory.proposal.definition.body),
        "Var(VarName(\"row_occupied_count\"))"
    );
}
