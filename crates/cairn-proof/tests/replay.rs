use cairn_core::Hash32;
use cairn_proof::{
    DemonstrationReplayCase, IndependentVerifier, ReplayVerifier, VerifyError,
    VERIFIER_SEMANTICS_VERSION,
};
use cairn_rkl::{
    ConflictPolicy, Effect, ExecLimits, Expr, ReferenceTheoryExecutor, Renderer, RewriteRule,
    RuntimeEnvironment, Schedule, Theory, TheoryChecker, TheoryExecLimits, TheoryExecutionRequest,
    TypeChecker, TypeEnvironment, Value,
};
use std::collections::{BTreeMap, BTreeSet};

fn fixture() -> (
    cairn_rkl::CheckedTheory,
    TheoryExecutionRequest,
    TheoryExecLimits,
) {
    let theory = TypeChecker::new(1_000)
        .check_theory(&Theory {
            inputs: Vec::new(),
            definitions: Vec::new(),
            rules: vec![RewriteRule {
                name: "emit".to_owned(),
                binders: Vec::new(),
                guard: Expr::Bool(true),
                effects: vec![Effect::Write {
                    target: Expr::Coord2 { row: 1, column: 2 },
                    value: Expr::Atom(4),
                }],
                priority: 0,
            }],
            schedule: Schedule::Parallel,
            renderer: Renderer {
                canvas: Expr::Int(3),
                layers: Vec::new(),
                conflict_policy: ConflictPolicy::Reject,
                background: Expr::Atom(0),
            },
            assumptions: vec![Expr::Bool(true)],
            proof_annotations: Vec::new(),
        })
        .expect("theory");
    let request = TheoryExecutionRequest {
        types: TypeEnvironment::new(),
        values: RuntimeEnvironment::new(),
        initial_facts: BTreeSet::new(),
        initial_state: BTreeMap::new(),
    };
    let limits = TheoryExecLimits {
        expression: ExecLimits::default(),
        maximum_bindings: 100,
        maximum_rule_firings: 100,
    };
    (theory, request, limits)
}

#[test]
fn independently_replays_demonstrations_and_predictions() {
    let (theory, request, limits) = fixture();
    let expected = ReferenceTheoryExecutor
        .execute(&theory, &request, limits)
        .expect("expected")
        .rendered;
    let evidence = Hash32::digest(b"demonstration");
    let (certificate, report) = ReplayVerifier
        .verify_demonstrations(
            &theory,
            &[DemonstrationReplayCase {
                evidence_hash: evidence,
                request: request.clone(),
                expected,
            }],
            limits,
        )
        .expect("verified demonstrations");
    assert_eq!(report.verified_cases, 1);
    assert_eq!(certificate.verifier_semantics, VERIFIER_SEMANTICS_VERSION);

    let prediction = ReplayVerifier
        .certify_prediction(&theory, evidence, &request, limits)
        .expect("certificate");
    let report = ReplayVerifier
        .verify_prediction(&theory, evidence, &request, &prediction, limits)
        .expect("verified prediction");
    assert_eq!(report.verified_cases, 1);
}

#[test]
fn planted_certificate_corruptions_fail_closed() {
    let (theory, request, limits) = fixture();
    let evidence = Hash32::digest(b"evidence");
    let certificate = ReplayVerifier
        .certify_prediction(&theory, evidence, &request, limits)
        .expect("certificate");

    let mut corrupt = certificate.clone();
    corrupt.output_hash = Hash32::digest(b"wrong output");
    assert_eq!(
        ReplayVerifier.verify_prediction(&theory, evidence, &request, &corrupt, limits),
        Err(VerifyError::PredictionMismatch)
    );

    let mut corrupt = certificate.clone();
    corrupt.verifier_semantics = "0.0.0".to_owned();
    assert_eq!(
        ReplayVerifier.verify_prediction(&theory, evidence, &request, &corrupt, limits),
        Err(VerifyError::SemanticsMismatch)
    );

    assert_eq!(
        ReplayVerifier.verify_prediction(
            &theory,
            Hash32::digest(b"other evidence"),
            &request,
            &certificate,
            limits,
        ),
        Err(VerifyError::EvidenceMismatch)
    );
}

#[test]
fn wrong_demonstration_output_is_rejected() {
    let (theory, request, limits) = fixture();
    let mut expected = ReferenceTheoryExecutor
        .execute(&theory, &request, limits)
        .expect("expected")
        .rendered;
    expected.background = Value::Atom(99);
    let result = ReplayVerifier.verify_demonstrations(
        &theory,
        &[DemonstrationReplayCase {
            evidence_hash: Hash32::digest(b"demo"),
            request,
            expected,
        }],
        limits,
    );
    assert_eq!(result, Err(VerifyError::PredictionMismatch));
}
