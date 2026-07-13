use cairn_core::Hash32;
use cairn_invent::InventionLimits;
use cairn_theory::{CollectionInventionError, CollectionMechanismInventor, CollectionTrace};
use std::collections::BTreeSet;

#[test]
fn invents_unordered_one_to_many_construction_and_transfers_cardinality() {
    let traces = [
        trace("sequence", &[3, 8], &[6, 7, 16, 17]),
        trace("vector", &[5, 11, 14], &[10, 11, 22, 23, 28, 29]),
        trace("table", &[2, 9, 13, 18], &[4, 5, 18, 19, 26, 27, 36, 37]),
    ];
    let learned = CollectionMechanismInventor
        .invent_expansion(&traces, 16, 4, InventionLimits::default())
        .expect("invent expansion");
    assert_eq!(learned.multiplicity, 2);
    assert_eq!(
        learned.observed_input_cardinalities,
        BTreeSet::from([2, 3, 4])
    );
    assert_eq!(
        CollectionMechanismInventor
            .expand(&learned, &[1, 7, 12, 20, 25])
            .expect("held-out expansion"),
        BTreeSet::from([2, 3, 14, 15, 24, 25, 40, 41, 50, 51])
    );

    let reordered = [
        trace("sequence", &[8, 3], &[17, 6, 16, 7]),
        trace("vector", &[14, 5, 11], &[29, 10, 23, 28, 11, 22]),
        trace("table", &[18, 2, 13, 9], &[37, 4, 27, 18, 36, 5, 26, 19]),
    ];
    let reordered = CollectionMechanismInventor
        .invent_expansion(&reordered, 16, 4, InventionLimits::default())
        .expect("order-invariant expansion");
    assert_eq!(
        learned.constructor.proposal.semantic_hash,
        reordered.constructor.proposal.semantic_hash
    );

    assert_eq!(
        CollectionMechanismInventor.invent_expansion(&traces, 16, 1, InventionLimits::default()),
        Err(CollectionInventionError::InvalidTrace)
    );
}

fn trace(domain: &str, inputs: &[i64], outputs: &[i64]) -> CollectionTrace {
    CollectionTrace {
        episode: Hash32::digest(domain.as_bytes()),
        inputs: inputs.to_vec(),
        outputs: outputs.to_vec(),
    }
}
