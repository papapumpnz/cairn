use cairn_arc::{FieldSchemaLearner, FieldTransformation};
use cairn_field::FiniteField;

fn field(rows: u32, columns: u32, values: &[i64]) -> FiniteField {
    FiniteField {
        shape: vec![rows, columns],
        values: values.to_vec(),
    }
}

fn from_rows(rows: &[&[i64]]) -> FiniteField {
    field(
        u32::try_from(rows.len()).expect("small fixture"),
        u32::try_from(rows[0].len()).expect("small fixture"),
        &rows
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect::<Vec<_>>(),
    )
}

fn learns(pairs: &[(FiniteField, FiniteField)]) -> cairn_arc::LearnedFieldTheory {
    FieldSchemaLearner.learn(pairs).expect("learned schema")
}

fn transpose(input: &FiniteField) -> FiniteField {
    let rows = input.shape[0] as usize;
    let columns = input.shape[1] as usize;
    let mut values = Vec::with_capacity(input.values.len());
    for column in 0..columns {
        for row in 0..rows {
            values.push(input.values[row * columns + column]);
        }
    }
    field(input.shape[1], input.shape[0], &values)
}

#[test]
fn learns_tensor_mask_construction() {
    let input = field(2, 2, &[0, 3, 3, 0]);
    let output = field(4, 4, &[0, 0, 0, 3, 0, 0, 3, 0, 0, 3, 0, 0, 3, 0, 0, 0]);
    let theory = learns(&[(input, output.clone())]);
    assert!(matches!(
        theory.transformation,
        FieldTransformation::TensorMask { .. }
    ));
}

#[test]
fn learns_enclosed_region_fill() {
    let input = field(
        5,
        5,
        &[
            0, 0, 0, 0, 0, 0, 3, 3, 3, 0, 0, 3, 0, 3, 0, 0, 3, 3, 3, 0, 0, 0, 0, 0, 0,
        ],
    );
    let output = field(
        5,
        5,
        &[
            0, 0, 0, 0, 0, 0, 3, 3, 3, 0, 0, 3, 4, 3, 0, 0, 3, 3, 3, 0, 0, 0, 0, 0, 0,
        ],
    );
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::FillEnclosed { fill: 4, .. }
    ));
}

#[test]
fn learns_periodic_extension_from_partial_field() {
    let input = field(
        4,
        5,
        &[2, 8, 0, 0, 0, 8, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    let output = field(
        4,
        5,
        &[2, 8, 2, 8, 2, 8, 3, 8, 3, 8, 2, 8, 2, 8, 2, 8, 3, 8, 3, 8],
    );
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::PeriodicTile {
            period_rows: 2,
            period_columns: 2
        }
    ));
}

#[test]
fn learns_gravity_between_independent_entities() {
    let input = field(
        6,
        3,
        &[0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 8, 0],
    );
    let output = field(
        6,
        3,
        &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 8, 0, 0, 8, 0],
    );
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::Gravity { .. }
    ));
}

#[test]
fn learns_background_only_span_completion() {
    let input = field(1, 7, &[2, 0, 9, 0, 9, 0, 2]);
    let output = field(1, 7, &[2, 2, 9, 2, 9, 2, 2]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::FillSpans {
            horizontal: true,
            ..
        }
    ));
}

#[test]
fn learns_component_size_ranking() {
    let input = field(
        4,
        5,
        &[5, 0, 5, 0, 5, 5, 0, 0, 0, 5, 0, 0, 0, 0, 5, 0, 0, 0, 0, 5],
    );
    let output = field(
        4,
        5,
        &[2, 0, 3, 0, 1, 2, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    );
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::ComponentSizeRank { .. }
    ));
}

#[test]
fn learns_panel_intersection_without_panel_vocabulary() {
    let input = field(2, 5, &[1, 0, 9, 0, 1, 1, 1, 9, 0, 1]);
    let output = field(2, 2, &[0, 0, 0, 2]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::PanelIntersection { .. }
    ));
}

#[test]
fn learns_periodic_row_extrapolation_and_rebinding() {
    let input = field(4, 2, &[1, 0, 1, 1, 0, 1, 1, 1]);
    let output = field(6, 2, &[2, 0, 2, 2, 0, 2, 2, 2, 2, 0, 2, 2]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::PeriodicRows { .. }
    ));
}

#[test]
fn learns_diagonal_phase_completion_from_sparse_evidence() {
    let input = field(4, 4, &[2, 8, 3, 0, 8, 3, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0]);
    let output = field(4, 4, &[2, 8, 3, 2, 8, 3, 2, 8, 3, 2, 8, 3, 2, 8, 3, 2]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::DiagonalPeriodic { .. }
    ));
}

#[test]
fn learns_attraction_toward_a_persistent_entity() {
    let input = field(1, 7, &[2, 2, 0, 0, 0, 8, 8]);
    let output = field(1, 7, &[0, 0, 0, 2, 2, 8, 8]);
    let vertical_input = field(7, 1, &[2, 2, 0, 0, 0, 8, 8]);
    let vertical_output = field(7, 1, &[0, 0, 0, 2, 2, 8, 8]);
    let theory = learns(&[(input, output), (vertical_input, vertical_output)]);
    assert!(
        matches!(theory.transformation, FieldTransformation::Attract { .. }),
        "{:?}",
        theory.transformation
    );
}

#[test]
fn learns_span_completion_over_frontend_supplied_panels() {
    let input = field(
        5,
        8,
        &[
            2, 2, 9, 0, 0, 9, 2, 2, 2, 2, 9, 0, 0, 9, 2, 2, 9, 9, 9, 9, 9, 9, 9, 9, 0, 0, 9, 0, 0,
            9, 0, 0, 0, 0, 9, 0, 0, 9, 0, 0,
        ],
    );
    let output = field(
        5,
        8,
        &[
            2, 2, 9, 2, 2, 9, 2, 2, 2, 2, 9, 2, 2, 9, 2, 2, 9, 9, 9, 9, 9, 9, 9, 9, 0, 0, 9, 0, 0,
            9, 0, 0, 0, 0, 9, 0, 0, 9, 0, 0,
        ],
    );
    let transposed_input = transpose(&input);
    let transposed_output = transpose(&output);
    let direct = cairn_arc::LearnedFieldTheory {
        transformation: FieldTransformation::PanelSpanCompletion { background: 0 },
        demonstrations_replayed: 0,
        candidates_evaluated: 0,
    };
    assert_eq!(
        FieldSchemaLearner
            .predict(&direct, &input)
            .expect("direct panel span"),
        output
    );
    assert_eq!(
        FieldSchemaLearner
            .predict(&direct, &transposed_input)
            .expect("transposed panel span"),
        transposed_output
    );
    let theory = learns(&[(input, output), (transposed_input, transposed_output)]);
    assert!(
        matches!(
            theory.transformation,
            FieldTransformation::PanelSpanCompletion { .. }
        ),
        "{:?}",
        theory.transformation
    );
}

#[test]
fn learns_boundary_anchored_shear_for_multiple_shapes() {
    let input = from_rows(&[
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 6, 6, 6, 0, 0, 0],
        &[0, 6, 0, 0, 6, 0, 0],
        &[0, 0, 6, 0, 0, 6, 0],
        &[0, 0, 0, 6, 6, 6, 0],
    ]);
    let output = from_rows(&[
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 0, 6, 6, 6, 0, 0],
        &[0, 0, 6, 0, 0, 6, 0],
        &[0, 0, 0, 6, 0, 6, 0],
        &[0, 0, 0, 6, 6, 6, 0],
    ]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::BoundaryAnchoredShear { .. }
    ));
}

#[test]
fn learns_template_completion_and_directional_repetition() {
    let input = field(
        5,
        13,
        &[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0,
            8, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ],
    );
    let output = field(
        5,
        13,
        &[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 0, 3, 3, 3, 0, 3, 3, 3, 0, 0, 8, 0,
            8, 0, 3, 0, 3, 0, 3, 0, 3, 0, 0, 8, 8, 8, 0, 3, 3, 3, 0, 3, 3, 3, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ],
    );
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::TemplateRayRepeat { .. }
    ));
}

#[test]
fn learns_bijective_categorical_law_across_demonstrations() {
    let pairs = [
        (field(1, 2, &[1, 2]), field(1, 2, &[5, 6])),
        (field(1, 2, &[5, 3]), field(1, 2, &[1, 4])),
        (field(1, 2, &[4, 6]), field(1, 2, &[3, 2])),
    ];
    assert!(matches!(
        learns(&pairs).transformation,
        FieldTransformation::CategoricalMap { .. }
    ));
}

#[test]
fn learns_periodic_repair_while_ignoring_sparse_noise() {
    let input = field(4, 4, &[0, 2, 1, 0, 3, 4, 3, 4, 1, 0, 1, 2, 3, 4, 0, 4]);
    let output = field(4, 4, &[1, 2, 1, 2, 3, 4, 3, 4, 1, 2, 1, 2, 3, 4, 3, 4]);
    let theory = learns(&[(input, output)]);
    assert!(
        matches!(
            theory.transformation,
            FieldTransformation::RobustPeriodicRepair { noise: 0, .. }
        ),
        "{:?}",
        theory.transformation
    );
}

#[test]
fn learns_to_select_the_only_panel_with_a_distinct_signature() {
    let input = field(
        2,
        10,
        &[8, 8, 0, 0, 8, 8, 0, 0, 2, 2, 8, 0, 0, 0, 8, 0, 0, 0, 2, 0],
    );
    let output = field(2, 2, &[2, 2, 2, 0]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::UniquePanel { background: 0 }
    ));
}

#[test]
fn learns_marker_spacing_and_broadcast_axis() {
    let input = field(
        4,
        7,
        &[
            0, 0, 2, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    );
    let output = field(
        4,
        7,
        &[
            0, 0, 2, 0, 8, 0, 2, 0, 0, 2, 0, 8, 0, 2, 0, 0, 2, 0, 8, 0, 2, 0, 0, 2, 0, 8, 0, 2,
        ],
    );
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::MarkerRayBroadcast { .. }
    ));
}

#[test]
fn learns_fourfold_rotational_completion() {
    let input = field(
        5,
        5,
        &[
            0, 0, 0, 0, 0, 0, 3, 3, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    );
    let output = field(
        5,
        5,
        &[
            0, 0, 0, 0, 0, 0, 3, 3, 0, 0, 0, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    );
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::RotationalCompletion { .. }
    ));
}

#[test]
fn learns_tiling_with_diagonal_halo_effects() {
    let input = field(1, 2, &[2, 0]);
    let output = field(2, 4, &[2, 8, 2, 8, 2, 8, 2, 8]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::TileDiagonalHalo { halo: 8, .. }
    ));
}

#[test]
fn learns_symbol_bound_local_effects() {
    let input = from_rows(&[
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 1, 0, 0, 0, 2, 0],
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0, 0, 0],
    ]);
    let output = from_rows(&[
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 7, 0, 0, 4, 0, 4],
        &[7, 1, 7, 0, 0, 2, 0],
        &[0, 7, 0, 0, 4, 0, 4],
        &[0, 0, 0, 0, 0, 0, 0],
    ]);
    assert!(matches!(
        learns(&[(input, output)]).transformation,
        FieldTransformation::SymbolNeighborhood { .. }
    ));
}
