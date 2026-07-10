use cairn_bench::{g0_hidden_law_families, SupportAxis};
use std::collections::BTreeSet;

#[test]
fn frozen_g0_generator_covers_unrelated_support_dimensions() {
    let families = g0_hidden_law_families();
    assert_eq!(families.len(), 5);
    assert!(families
        .iter()
        .all(|family| { family.source_problems.len() == 2 && family.held_out_tasks.len() == 2 }));
    let domains = families
        .iter()
        .map(|family| family.domain.as_str())
        .collect::<BTreeSet<_>>();
    assert!(domains.len() >= 4);
    let axes = families
        .iter()
        .flat_map(|family| family.varied_axes.iter().copied())
        .collect::<BTreeSet<_>>();
    assert!(axes.contains(&SupportAxis::Cardinalities));
    assert!(axes.contains(&SupportAxis::CategoricalValues));
    assert!(axes.contains(&SupportAxis::Layouts));
    assert!(axes.contains(&SupportAxis::Positions));
}
