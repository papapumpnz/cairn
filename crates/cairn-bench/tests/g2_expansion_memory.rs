use cairn_core::Hash32;
use cairn_invent::InventionLimits;
use cairn_memory::{ExpansionPromotion, ExpansionStore, ModuleStatus};
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, ExpansionMemoryRuntimeError};
use cairn_theory::{CollectionMechanismInventor, CollectionTrace};
use cairn_verifier::{ExpansionMechanismVerifier, ExpansionVerifyError};
use std::collections::BTreeSet;

#[test]
fn unordered_one_to_many_constructor_promotes_and_removes_future_examples() {
    let first = [
        trace("sequence-a", &[3, 8]),
        trace("vector-a", &[5, 11, 14]),
    ];
    let second = [
        trace("table-a", &[2, 9, 13, 18]),
        trace("workflow-a", &[4, 7, 12, 16, 21]),
    ];
    let first_learned = CollectionMechanismInventor
        .invent_expansion(&first, 16, 4, InventionLimits::default())
        .expect("first cohort");
    let second_learned = CollectionMechanismInventor
        .invent_expansion(&second, 16, 4, InventionLimits::default())
        .expect("second cohort");
    assert_eq!(
        first_learned.semantic_hash().expect("id"),
        second_learned.semantic_hash().expect("id")
    );
    let first_receipt = ExpansionMechanismVerifier
        .verify(&first_learned, &first, ExecLimits::default())
        .expect("verify first");
    let second_receipt = ExpansionMechanismVerifier
        .verify(&second_learned, &second, ExecLimits::default())
        .expect("verify second");
    let mut store = ExpansionStore::new();
    assert_eq!(
        store
            .record_verified_cohort(&first_learned, first_receipt)
            .expect("provisional"),
        ExpansionPromotion::Provisional {
            verified_cohorts: 1
        }
    );
    assert_eq!(
        store
            .record_verified_cohort(&second_learned, second_receipt)
            .expect("persistent"),
        ExpansionPromotion::Persistent {
            verified_cohorts: 2
        }
    );
    let id = first_learned.semantic_hash().expect("id");
    assert_eq!(
        store.module(id).expect("module").status(),
        ModuleStatus::Persistent
    );
    assert_eq!(store.module(id).expect("module").verified_experiences(), 4);

    let runtime = CognitionRuntime::with_expansion_memory(store);
    let held_out = [1, 6, 10, 17, 25, 31];
    let outcome = runtime
        .propose_expansions_from_memory(&held_out)
        .expect("zero-example reuse");
    assert_eq!(outcome.persistent_mechanisms_attempted, 1);
    assert_eq!(outcome.proposals, vec![expected(&held_out)]);
    assert!(
        first_learned.constructor.proposal.candidates_evaluated
            > outcome.persistent_mechanisms_attempted * 10
    );
    assert!(matches!(
        CognitionRuntime::new().propose_expansions_from_memory(&held_out),
        Err(CognitionError::ExpansionMemory(
            ExpansionMemoryRuntimeError::NoVerifiedProposal
        ))
    ));

    let wrong = [
        CollectionTrace {
            episode: Hash32::digest(b"wrong-a"),
            inputs: vec![2, 5],
            outputs: vec![4, 5, 10, 12],
        },
        CollectionTrace {
            episode: Hash32::digest(b"wrong-b"),
            inputs: vec![3, 7, 9],
            outputs: vec![6, 7, 14, 15, 18, 20],
        },
    ];
    assert_eq!(
        ExpansionMechanismVerifier.verify(&first_learned, &wrong, ExecLimits::default()),
        Err(ExpansionVerifyError::ReplayMismatch)
    );
    eprintln!("G2_EXPANSION cohorts=2 experiences=4 multiplicity=2 fresh_candidates={} memory_attempts=1 memory_examples=0 heldout_inputs=6 heldout_outputs=12 wrong_accepts=0", first_learned.constructor.proposal.candidates_evaluated);
}

fn trace(domain: &str, inputs: &[i64]) -> CollectionTrace {
    CollectionTrace {
        episode: Hash32::digest(domain.as_bytes()),
        inputs: inputs.to_vec(),
        outputs: expected(inputs).into_iter().collect(),
    }
}

fn expected(inputs: &[i64]) -> BTreeSet<i64> {
    inputs
        .iter()
        .flat_map(|value| [value * 2, value * 2 + 1])
        .collect()
}
