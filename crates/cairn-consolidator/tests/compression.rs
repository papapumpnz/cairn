use cairn_consolidator::*;
use cairn_core::Hash32;

#[test]
fn repeated_proof_structure_becomes_a_parameterized_proposal() {
    let traces = (0_u8..4)
        .map(|value| ProofTrace {
            episode: Hash32::digest(&[value]),
            operations: vec![
                "select".to_owned(),
                format!("role-{value}"),
                "order".to_owned(),
                "transform".to_owned(),
                "render".to_owned(),
            ],
        })
        .collect::<Vec<_>>();
    let proposal = Consolidator.anti_unify(&traces).expect("compressed");
    assert!(proposal.compressed_nodes < proposal.uncompressed_nodes);
    assert_eq!(proposal.pattern[1], PatternElement::Parameter(0));
}

#[test]
fn unrelated_traces_do_not_create_a_mechanism() {
    let traces = [
        ProofTrace {
            episode: Hash32::digest(b"a"),
            operations: vec!["one".to_owned(), "two".to_owned()],
        },
        ProofTrace {
            episode: Hash32::digest(b"b"),
            operations: vec!["three".to_owned(), "four".to_owned()],
        },
    ];
    assert_eq!(
        Consolidator.anti_unify(&traces),
        Err(ConsolidationError::NoSharedStructure)
    );
}
