use cairn_rkl::{
    BinaryOp, Binder, ConflictPolicy, Effect, ExecLimits, Expr, Quantifier,
    ReferenceTheoryExecutor, Renderer, RewriteRule, RuleBinder, RuntimeEnvironment, Schedule,
    Theory, TheoryChecker, TheoryExecError, TheoryExecLimits, TheoryExecutionRequest, Type,
    TypeChecker, TypeEnvironment, Value, VarName,
};
use std::collections::{BTreeMap, BTreeSet};

fn limits() -> TheoryExecLimits {
    TheoryExecLimits {
        expression: ExecLimits {
            fuel: 100_000,
            maximum_call_depth: 32,
            maximum_trace_events: 10_000,
        },
        maximum_bindings: 1_000,
        maximum_rule_firings: 1_000,
    }
}

fn request() -> TheoryExecutionRequest {
    TheoryExecutionRequest {
        types: TypeEnvironment::new(),
        values: RuntimeEnvironment::new(),
        initial_facts: BTreeSet::new(),
        initial_state: BTreeMap::new(),
    }
}

fn two_coordinates() -> Expr {
    Expr::Binary {
        op: BinaryOp::Union,
        left: Box::new(Expr::SingletonSet {
            value: Box::new(Expr::Coord2 { row: 0, column: 0 }),
            maximum: 1,
        }),
        right: Box::new(Expr::SingletonSet {
            value: Box::new(Expr::Coord2 { row: 0, column: 1 }),
            maximum: 1,
        }),
    }
}

fn rule(name: &str, priority: u32, guard: bool, effects: Vec<Effect>) -> RewriteRule {
    RewriteRule {
        name: name.to_owned(),
        binders: Vec::new(),
        guard: Expr::Bool(guard),
        effects,
        priority,
    }
}

fn checked_theory(
    rules: Vec<RewriteRule>,
    schedule: Schedule,
    conflict_policy: ConflictPolicy,
    layers: Vec<Expr>,
) -> cairn_rkl::CheckedTheory {
    TypeChecker::new(10_000)
        .check_theory(&Theory {
            inputs: Vec::new(),
            definitions: Vec::new(),
            rules,
            schedule,
            renderer: Renderer {
                canvas: Expr::Int(0),
                layers,
                conflict_policy,
                background: Expr::Atom(0),
            },
            assumptions: vec![Expr::Bool(true)],
            proof_annotations: Vec::new(),
        })
        .expect("checked theory")
}

#[test]
fn foreach_rule_writes_every_member_with_provenance() {
    let cell = VarName("cell".to_owned());
    let theory = checked_theory(
        vec![RewriteRule {
            name: "paint".to_owned(),
            binders: vec![RuleBinder {
                variable: Binder {
                    name: cell.clone(),
                    ty: Type::Coord2,
                },
                domain: two_coordinates(),
                quantifier: Quantifier::ForEach,
            }],
            guard: Expr::Bool(true),
            effects: vec![Effect::Write {
                target: Expr::Var(cell),
                value: Expr::Atom(7),
            }],
            priority: 0,
        }],
        Schedule::Parallel,
        ConflictPolicy::Reject,
        Vec::new(),
    );
    let trace = ReferenceTheoryExecutor
        .execute(&theory, &request(), limits())
        .expect("execute foreach");
    assert_eq!(trace.rule_firings.len(), 2);
    assert_eq!(trace.rendered.writes.len(), 2);
    assert!(trace
        .rendered
        .writes
        .values()
        .all(|value| value == &Value::Atom(7)));
}

#[test]
fn parallel_conflicts_reject_or_resolve_by_explicit_policy() {
    let rules = vec![
        rule(
            "low",
            1,
            true,
            vec![Effect::Write {
                target: Expr::Atom(9),
                value: Expr::Atom(1),
            }],
        ),
        rule(
            "high",
            2,
            true,
            vec![Effect::Write {
                target: Expr::Atom(9),
                value: Expr::Atom(2),
            }],
        ),
    ];
    let rejected = checked_theory(
        rules.clone(),
        Schedule::Parallel,
        ConflictPolicy::Reject,
        Vec::new(),
    );
    assert!(matches!(
        ReferenceTheoryExecutor.execute(&rejected, &request(), limits()),
        Err(TheoryExecError::Conflict {
            target: Value::Atom(9)
        })
    ));

    let priority = checked_theory(
        rules.clone(),
        Schedule::Parallel,
        ConflictPolicy::Priority,
        Vec::new(),
    );
    let trace = ReferenceTheoryExecutor
        .execute(&priority, &request(), limits())
        .expect("priority resolution");
    assert_eq!(trace.rendered.writes[&Value::Atom(9)], Value::Atom(2));

    let last = checked_theory(
        rules,
        Schedule::Sequence(vec!["low".to_owned(), "high".to_owned()]),
        ConflictPolicy::LastWrite,
        Vec::new(),
    );
    let trace = ReferenceTheoryExecutor
        .execute(&last, &request(), limits())
        .expect("last-write resolution");
    assert_eq!(trace.rendered.writes[&Value::Atom(9)], Value::Atom(2));
}

#[test]
fn quantifiers_have_distinct_exact_cardinality_semantics() {
    let member = VarName("member".to_owned());
    let unique = checked_theory(
        vec![RewriteRule {
            name: "unique".to_owned(),
            binders: vec![RuleBinder {
                variable: Binder {
                    name: member.clone(),
                    ty: Type::Coord2,
                },
                domain: two_coordinates(),
                quantifier: Quantifier::Unique,
            }],
            guard: Expr::Bool(true),
            effects: vec![Effect::Emit(Expr::Var(member))],
            priority: 0,
        }],
        Schedule::Parallel,
        ConflictPolicy::SameValueIdempotent,
        Vec::new(),
    );
    assert_eq!(
        ReferenceTheoryExecutor.execute(&unique, &request(), limits()),
        Err(TheoryExecError::Cardinality {
            quantifier: Quantifier::Unique,
            actual: 2
        })
    );

    let group = VarName("group".to_owned());
    let grouped = checked_theory(
        vec![RewriteRule {
            name: "group".to_owned(),
            binders: vec![RuleBinder {
                variable: Binder {
                    name: group.clone(),
                    ty: Type::CellSet { maximum: 2 },
                },
                domain: two_coordinates(),
                quantifier: Quantifier::Group,
            }],
            guard: Expr::Bool(true),
            effects: vec![Effect::Emit(Expr::Count(Box::new(Expr::Var(group))))],
            priority: 0,
        }],
        Schedule::Parallel,
        ConflictPolicy::SameValueIdempotent,
        Vec::new(),
    );
    let trace = ReferenceTheoryExecutor
        .execute(&grouped, &request(), limits())
        .expect("group execution");
    assert!(trace.rendered.facts.contains(&Value::Int(2)));
}

#[test]
fn decision_fixpoint_and_renderer_layers_are_deterministic() {
    let decision = checked_theory(
        vec![
            rule("skip", 0, false, vec![Effect::Emit(Expr::Atom(1))]),
            rule("take", 0, true, vec![Effect::Emit(Expr::Atom(2))]),
        ],
        Schedule::DecisionList(vec!["skip".to_owned(), "take".to_owned()]),
        ConflictPolicy::SameValueIdempotent,
        Vec::new(),
    );
    let trace = ReferenceTheoryExecutor
        .execute(&decision, &request(), limits())
        .expect("decision list");
    assert!(!trace.rendered.facts.contains(&Value::Atom(1)));
    assert!(trace.rendered.facts.contains(&Value::Atom(2)));

    let fixpoint = checked_theory(
        vec![rule("stable", 0, true, vec![Effect::Emit(Expr::Atom(4))])],
        Schedule::BoundedFixpoint {
            rules: vec!["stable".to_owned()],
            maximum_steps: 4,
        },
        ConflictPolicy::SameValueIdempotent,
        vec![
            Expr::Tuple(vec![Expr::Atom(8), Expr::Atom(1)]),
            Expr::Tuple(vec![Expr::Atom(8), Expr::Atom(2)]),
        ],
    );
    let mut fixpoint = fixpoint.theory().clone();
    fixpoint.renderer.conflict_policy = ConflictPolicy::LastWrite;
    let fixpoint = TypeChecker::new(10_000)
        .check_theory(&fixpoint)
        .expect("rechecked");
    let trace = ReferenceTheoryExecutor
        .execute(&fixpoint, &request(), limits())
        .expect("fixpoint and layers");
    assert_eq!(trace.fixpoint_iterations, 2);
    assert_eq!(trace.rendered.writes[&Value::Atom(8)], Value::Atom(2));
}

#[test]
fn dependency_dag_uses_canonical_topological_order() {
    let theory = checked_theory(
        vec![
            rule(
                "later",
                0,
                true,
                vec![Effect::Write {
                    target: Expr::Atom(5),
                    value: Expr::Atom(2),
                }],
            ),
            rule(
                "earlier",
                0,
                true,
                vec![Effect::Write {
                    target: Expr::Atom(5),
                    value: Expr::Atom(1),
                }],
            ),
        ],
        Schedule::DependencyDag(vec![("earlier".to_owned(), "later".to_owned())]),
        ConflictPolicy::LastWrite,
        Vec::new(),
    );
    let trace = ReferenceTheoryExecutor
        .execute(&theory, &request(), limits())
        .expect("dag execution");
    assert_eq!(
        trace
            .rule_firings
            .iter()
            .map(|firing| firing.rule.as_str())
            .collect::<Vec<_>>(),
        ["earlier", "later"]
    );
    assert_eq!(trace.rendered.writes[&Value::Atom(5)], Value::Atom(2));
}
