use cairn_cegis::{CegisLimits, CegisLoop, TheoryCandidate};
use cairn_core::Hash32;
use cairn_proof::DemonstrationReplayCase;
use cairn_rkl::{
    ConflictPolicy, Effect, ExecLimits, Expr, ReferenceTheoryExecutor, Renderer, RewriteRule,
    RuntimeEnvironment, Schedule, Theory, TheoryChecker, TheoryExecLimits, TheoryExecutionRequest,
    TypeChecker, TypeEnvironment,
};
use std::collections::{BTreeMap, BTreeSet};

fn theory(atom: u16) -> cairn_rkl::CheckedTheory {
    TypeChecker::new(1_000)
        .check_theory(&Theory {
            inputs: Vec::new(),
            definitions: Vec::new(),
            rules: vec![RewriteRule {
                name: "write".to_owned(),
                binders: Vec::new(),
                guard: Expr::Bool(true),
                effects: vec![Effect::Write {
                    target: Expr::Coord2 { row: 0, column: 0 },
                    value: Expr::Atom(atom),
                }],
                priority: 0,
            }],
            schedule: Schedule::Parallel,
            renderer: Renderer {
                canvas: Expr::Int(1),
                layers: Vec::new(),
                conflict_policy: ConflictPolicy::Reject,
                background: Expr::Atom(0),
            },
            assumptions: vec![Expr::Bool(true)],
            proof_annotations: Vec::new(),
        })
        .expect("theory")
}

#[test]
fn counterexample_blocks_wrong_semantics_before_verified_candidate() {
    let correct = theory(7);
    let request = TheoryExecutionRequest {
        types: TypeEnvironment::new(),
        values: RuntimeEnvironment::new(),
        initial_facts: BTreeSet::new(),
        initial_state: BTreeMap::new(),
    };
    let execution = TheoryExecLimits {
        expression: ExecLimits::default(),
        maximum_bindings: 100,
        maximum_rule_firings: 100,
    };
    let expected = ReferenceTheoryExecutor
        .execute(&correct, &request, execution)
        .expect("expected")
        .rendered;
    let outcome = CegisLoop
        .search(
            vec![
                TheoryCandidate {
                    theory: theory(3),
                    structural_cost: 1,
                    origin: "wrong".to_owned(),
                },
                TheoryCandidate {
                    theory: correct,
                    structural_cost: 2,
                    origin: "learned".to_owned(),
                },
            ],
            &[DemonstrationReplayCase {
                evidence_hash: Hash32::digest(b"demo"),
                request,
                expected,
            }],
            CegisLimits {
                maximum_candidates: 10,
                execution,
            },
        )
        .expect("search");
    assert_eq!(outcome.candidates_executed, 2);
    assert_eq!(outcome.counterexamples.len(), 1);
    assert_eq!(outcome.verification.verified_cases, 1);
}
