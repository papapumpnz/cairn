use cairn_field::{FiniteField, PartitionBroadcastLearner, PartitionControllerRole};

#[test]
fn learned_partition_controller_transfers_across_geometry_and_separator_values() {
    let first = partitioned(2, 9, &[1, 2, 3, 0], 11, 0, 99);
    let second = partitioned(2, 8, &[4, 0, 2, 6], 21, 3, 77);
    let binding = PartitionBroadcastLearner
        .learn(&[
            (first.clone(), broadcast(&first, 2)),
            (second.clone(), broadcast(&second, 2)),
        ])
        .expect("non-ARC dashboard binding");
    assert_eq!(
        binding.controller_role,
        PartitionControllerRole::OmitsSharedMarker
    );
    let held_out = partitioned(3, 5, &[2, 4, 0, 6, 3, 0, 0, 0, 8], 31, 4, 66);
    assert_eq!(
        PartitionBroadcastLearner
            .predict(&binding, &held_out)
            .expect("changed geometry"),
        broadcast(&held_out, 3)
    );
}

fn partitioned(
    size: usize,
    separator: i64,
    controller: &[i64],
    seed: i64,
    controller_panel: usize,
    marker: i64,
) -> FiniteField {
    let extent = size * size + size - 1;
    let mut values = vec![separator; extent * extent];
    for panel_row in 0..size {
        for panel_column in 0..size {
            for local_row in 0..size {
                for local_column in 0..size {
                    let row = panel_row * (size + 1) + local_row;
                    let column = panel_column * (size + 1) + local_column;
                    values[row * extent + column] =
                        if panel_row * size + panel_column == controller_panel {
                            controller[local_row * size + local_column]
                        } else if local_row == 0 && local_column == 0 {
                            marker
                        } else {
                            seed + i64::try_from(
                                panel_row * 17 + panel_column * 11 + local_row * 3 + local_column,
                            )
                            .expect("small")
                        };
                }
            }
        }
    }
    FiniteField {
        shape: vec![u32::try_from(extent).expect("small"); 2],
        values,
    }
}

fn broadcast(input: &FiniteField, size: usize) -> FiniteField {
    let extent = usize::try_from(input.shape[0]).expect("small");
    let separator = input.values[size];
    let mut values = vec![separator; input.values.len()];
    let marker = (0..size * size)
        .flat_map(|panel| {
            let row = (panel / size) * (size + 1);
            let column = (panel % size) * (size + 1);
            std::iter::once(input.values[row * extent + column])
        })
        .find(|value| {
            (0..size * size)
                .filter(|panel| {
                    let row = (panel / size) * (size + 1);
                    let column = (panel % size) * (size + 1);
                    input.values[row * extent + column] == *value
                })
                .count()
                == size * size - 1
        })
        .expect("shared marker");
    let controller_panel = (0..size * size)
        .find(|panel| {
            let row_start = (panel / size) * (size + 1);
            let column_start = (panel % size) * (size + 1);
            !(0..size)
                .flat_map(|row| {
                    (0..size).map(move |column| {
                        input.values[(row_start + row) * extent + column_start + column]
                    })
                })
                .any(|value| value == marker)
        })
        .expect("controller");
    let controller_row = (controller_panel / size) * (size + 1);
    let controller_column = (controller_panel % size) * (size + 1);
    for panel_row in 0..size {
        for panel_column in 0..size {
            let control = input.values
                [(controller_row + panel_row) * extent + controller_column + panel_column];
            for local_row in 0..size {
                for local_column in 0..size {
                    let row = panel_row * (size + 1) + local_row;
                    let column = panel_column * (size + 1) + local_column;
                    values[row * extent + column] = control;
                }
            }
        }
    }
    FiniteField {
        shape: input.shape.clone(),
        values,
    }
}
