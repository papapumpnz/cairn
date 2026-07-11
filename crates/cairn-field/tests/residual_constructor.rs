use cairn_field::{FiniteField, ResidualConstructorInventor};
use cairn_invent::InventionLimits;
use cairn_rkl::ExecLimits;

#[test]
fn unexplained_created_cells_induce_a_transferable_constructor() {
    let pairs = [(3, 4), (5, 4)].map(|(size, paint)| (blank(size), main_diagonal(size, paint)));
    let learned = ResidualConstructorInventor
        .invent(&pairs, 0, InventionLimits::default())
        .expect("residual constructor");
    assert_eq!(learned.residual_cells, 8);
    assert_eq!(learned.training_cells, 34);
    let predicted = ResidualConstructorInventor
        .predict(&learned, &blank(7), ExecLimits::default())
        .expect("changed geometry");
    assert_eq!(predicted, main_diagonal(7, 4));
    assert!(learned.proposal.candidates_evaluated > 1);
    eprintln!(
        "RESIDUAL_METRICS candidates={} cost={} training_cells={} residual_cells={} held_out_cells=49 exact=49 body={:?}",
        learned.proposal.candidates_evaluated,
        learned.proposal.structural_cost,
        learned.training_cells,
        learned.residual_cells,
        learned.proposal.definition.body
    );
}

#[test]
fn mixed_created_values_are_not_hidden_as_one_constructor() {
    let input = blank(2);
    let output = FiniteField {
        shape: vec![2, 2],
        values: vec![3, 0, 0, 4],
    };
    assert!(matches!(
        ResidualConstructorInventor.invent(&[(input, output)], 0, InventionLimits::default()),
        Err(cairn_field::FieldError::ResidualConflict)
    ));
}

fn blank(size: u32) -> FiniteField {
    FiniteField {
        shape: vec![size, size],
        values: vec![0; usize::try_from(size * size).expect("small fixture")],
    }
}

fn main_diagonal(size: u32, paint: i64) -> FiniteField {
    let mut field = blank(size);
    let width = usize::try_from(size).expect("small fixture");
    for index in 0..width {
        field.values[index * width + index] = paint;
    }
    field
}
