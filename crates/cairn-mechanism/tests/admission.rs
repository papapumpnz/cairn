use cairn_core::Hash32;
use cairn_mechanism::*;
use cairn_rkl::{Binder, Effect, Expr, IntRange, Type, VarName};
use std::collections::BTreeSet;

fn mechanism() -> Mechanism {
    Mechanism {
        signature: Signature {
            inputs: vec![Type::Int(IntRange { lower: 0, upper: 9 })],
            output: Type::Int(IntRange { lower: 0, upper: 9 }),
        },
        variables: vec![Binder {
            name: VarName("item".to_owned()),
            ty: Type::Int(IntRange { lower: 0, upper: 9 }),
        }],
        entity_definitions: Vec::new(),
        preconditions: Expr::Bool(true),
        transition_rules: Vec::new(),
        invariants: vec![Expr::Bool(true)],
        termination: Expr::Bool(true),
        effects: vec![Effect::Emit(Expr::Var(VarName("item".to_owned())))],
        support: SupportGeometry {
            observed_domains: BTreeSet::from([DomainClass("sequence".to_owned())]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 2,
                maximum: 6,
            }]),
            varied_variables: BTreeSet::from([VariableId("item".to_owned())]),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::from([VariableId("nesting".to_owned())]),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: BTreeSet::from([Hash32::digest(b"proof")]),
            verifier_semantics: "test-verifier".to_owned(),
            verified_cases: 20,
        },
    }
}

fn evidence() -> InventionEvidence {
    InventionEvidence {
        target_absent_from_named_vocabulary: true,
        source_code_changes: 0,
        independent_experiences: 4,
        initial_success_parts_per_million: 100_000,
        post_experience_success_parts_per_million: 800_000,
        uncompressed_cost: 100,
        compressed_cost: 30,
        held_out_verified_cases: 20,
        candidate_executions_without: 1_000,
        candidate_executions_with: 100,
        examples_without: 5,
        examples_with: 2,
        false_acceptances_without: 0,
        false_acceptances_with: 0,
        actual_reuse_count: 3,
        necessary_reuse_count: 2,
    }
}

#[test]
fn persistent_admission_requires_learning_and_reuse_evidence() {
    assert_eq!(
        admit_persistent(&mechanism(), &evidence()),
        Ok(MechanismStatus::Persistent)
    );
}

#[test]
fn storage_without_actual_reuse_is_rejected() {
    let mut evidence = evidence();
    evidence.actual_reuse_count = 0;
    assert_eq!(
        admit_persistent(&mechanism(), &evidence),
        Err(AdmissionError::NoActualReuse)
    );
}

#[test]
fn overlapping_support_claims_are_rejected() {
    let mut mechanism = mechanism();
    mechanism
        .support
        .unknown_variables
        .insert(VariableId("item".to_owned()));
    assert_eq!(
        mechanism.validate(),
        Err(AdmissionError::ConflictingSupportAxis(VariableId(
            "item".to_owned()
        )))
    );
}
