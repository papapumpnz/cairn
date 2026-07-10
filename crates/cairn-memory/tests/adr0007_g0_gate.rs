use cairn_consolidator::{Consolidator, ProofTrace};
use cairn_invent::{BindingExample, DefinitionInventor, InventionLimits, InventionProblem};
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, InventionEvidence, Mechanism, Signature,
    SupportGeometry as MechanismSupport, VariableId,
};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, ModuleStatus, PromotionRunner, SolveOrigin,
    SupportGeometry as MemorySupport, VariationDimension,
};
use cairn_rkl::{
    Binder, Effect, ExecLimits, Expr, IntRange, RuntimeEnvironment, Type, Value, VarName,
};
use cairn_verifier::PromotionVerifier;
use std::collections::BTreeSet;

fn limits(maximum_cost: u16, maximum_candidates: u64) -> InventionLimits {
    InventionLimits {
        maximum_cost,
        maximum_candidates,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits::default(),
    }
}

fn example(name: &str, reading: i64) -> BindingExample {
    let mut inputs = RuntimeEnvironment::new();
    inputs.insert(VarName(name.to_owned()), Value::Int(reading));
    BindingExample {
        inputs,
        output: Value::Int(reading.abs()),
    }
}

fn problem(
    name: &str,
    bound: i64,
    readings: &[i64],
    maximum_cost: u16,
    maximum_candidates: u64,
) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
        parameters: vec![Binder {
            name: VarName(name.to_owned()),
            ty: Type::Int(IntRange {
                lower: -bound,
                upper: bound,
            }),
        }],
        return_type: Type::Int(IntRange {
            lower: 0,
            upper: bound,
        }),
        literals: vec![Value::Int(0)],
        demonstrations: readings.iter().map(|value| example(name, *value)).collect(),
        limits: limits(maximum_cost, maximum_candidates),
    }
}

fn structural_operations(expression: &Expr, output: &mut Vec<String>) {
    match expression {
        Expr::Var(_) => output.push("variable".to_owned()),
        Expr::Int(_) => output.push("integer-literal".to_owned()),
        Expr::Unary { op, argument } => {
            output.push(format!("unary:{op:?}"));
            structural_operations(argument, output);
        }
        Expr::Binary { op, left, right } => {
            output.push(format!("binary:{op:?}"));
            structural_operations(left, output);
            structural_operations(right, output);
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            output.push("if".to_owned());
            structural_operations(condition, output);
            structural_operations(then_branch, output);
            structural_operations(else_branch, output);
        }
        _ => output.push("bounded-rkl-node".to_owned()),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn hidden_law_is_invented_promoted_reused_and_reduces_future_search() {
    let initial_store = MechanismStore::new();
    let held_out_specs = [
        ("a", 18, -17),
        ("b", 20, -19),
        ("c", 22, -21),
        ("d", 24, -23),
        ("e", 26, -25),
        ("f", 28, -27),
        ("g", 30, -29),
        ("h", 32, -31),
        ("i", 34, -33),
        ("j", 36, -35),
    ];

    let initial_successes = held_out_specs
        .iter()
        .filter(|(name, bound, reading)| {
            let underpowered = problem(name, *bound, &[*reading, 2], 1, 1);
            MemoryAssistedInventor
                .invent(&underpowered, &initial_store)
                .is_ok()
        })
        .count();
    assert_eq!(initial_successes, 0);

    let first = DefinitionInventor
        .invent(&problem(
            "reading",
            10,
            &[-9, -4, -1, 0, 2, 7],
            8,
            1_000_000,
        ))
        .expect("first independent invention");
    let second = DefinitionInventor
        .invent(&problem(
            "signal",
            16,
            &[-15, -6, -2, 1, 5, 13],
            8,
            1_000_000,
        ))
        .expect("second independent invention");
    let proof_traces = [&first, &second]
        .into_iter()
        .map(|outcome| {
            let mut operations = Vec::new();
            structural_operations(&outcome.proposal.definition.body, &mut operations);
            ProofTrace {
                episode: outcome.proposal.semantic_hash,
                operations,
            }
        })
        .collect::<Vec<_>>();
    let compression = Consolidator
        .anti_unify(&proof_traces)
        .expect("proof-subgraph compression");
    assert!(compression.compressed_nodes < compression.uncompressed_nodes);
    let module = PromotionRunner
        .promote(
            &[first.proposal.clone(), second.proposal.clone()],
            MemorySupport {
                independent_tasks: 2,
                held_out_rebindings: 1,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::ParameterNames,
                    VariationDimension::NumericRanges,
                ]),
            },
        )
        .expect("provisional module");
    let mut store = MechanismStore::new();
    store.insert(module.clone());
    assert!(!store.set_status(module.id(), ModuleStatus::Persistent));

    let mut post_successes = 0_u32;
    let mut memory_candidates = 0_u64;
    let mut fresh_candidates = 0_u64;
    let mut verified_predictions = 0_u64;
    for (name, bound, reading) in held_out_specs {
        let full = problem(name, bound, &[reading, -7, -2, 0, 3, 11], 8, 1_000_000);
        let fresh = DefinitionInventor.invent(&full).expect("fresh baseline");
        fresh_candidates += fresh.proposal.candidates_evaluated;

        let one_shot = problem(name, bound, &[reading], 8, 1_000_000);
        let reused = MemoryAssistedInventor
            .invent(&one_shot, &store)
            .expect("memory reuse");
        assert_eq!(reused.origin, SolveOrigin::MemoryProposal);
        memory_candidates += reused.metrics.total_search_work;
        let probe = example(name, bound - 1);
        assert_eq!(
            DefinitionInventor
                .predict(
                    &reused.invention.proposal,
                    &probe.inputs,
                    one_shot.limits.execution,
                )
                .expect("held-out execution"),
            probe.output
        );
        post_successes += 1;
        verified_predictions += 1;
    }
    assert_eq!(post_successes, 10);
    eprintln!(
        "G0_METRICS initial={initial_successes}/10 post={post_successes}/10 fresh_candidates={fresh_candidates} memory_candidates={memory_candidates} verified_predictions={verified_predictions}"
    );
    assert!(fresh_candidates >= memory_candidates.saturating_mul(4));

    let definition = module.definition().clone();
    let variable = VariableId("input".to_owned());
    let mechanism = Mechanism {
        signature: Signature {
            inputs: definition
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            output: definition.return_type.clone(),
        },
        variables: definition.parameters.clone(),
        entity_definitions: vec![definition.clone()],
        preconditions: Expr::Bool(true),
        transition_rules: Vec::new(),
        invariants: vec![Expr::Bool(true)],
        termination: Expr::Bool(true),
        effects: vec![Effect::Emit(Expr::Call {
            function: definition.name.clone(),
            arguments: definition
                .parameters
                .iter()
                .map(|parameter| Expr::Var(parameter.name.clone()))
                .collect(),
        })],
        support: MechanismSupport {
            observed_domains: BTreeSet::from([DomainClass("bounded-scalar".to_owned())]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 1,
                maximum: 1,
            }]),
            varied_variables: BTreeSet::from([variable]),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::from([VariableId("collection-shape".to_owned())]),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: module.lineage().iter().copied().collect(),
            verifier_semantics: "dual-rkl-reference-bytecode-replay".to_owned(),
            verified_cases: verified_predictions
                + u64::try_from(first.admission.exact_cross_implementation_replays)
                    .expect("small evidence")
                + u64::try_from(second.admission.exact_cross_implementation_replays)
                    .expect("small evidence"),
        },
    };
    let evidence = InventionEvidence {
        target_absent_from_named_vocabulary: initial_store.is_empty(),
        source_code_changes: 0,
        independent_experiences: 2,
        initial_success_parts_per_million: 0,
        post_experience_success_parts_per_million: 1_000_000,
        uncompressed_cost: u64::from(first.proposal.structural_cost)
            + u64::from(second.proposal.structural_cost),
        compressed_cost: u64::from(first.proposal.structural_cost),
        held_out_verified_cases: verified_predictions,
        candidate_executions_without: fresh_candidates,
        candidate_executions_with: memory_candidates,
        examples_without: 6,
        examples_with: 1,
        false_acceptances_without: 0,
        false_acceptances_with: 0,
        actual_reuse_count: post_successes,
        necessary_reuse_count: post_successes,
    };
    let verified = PromotionVerifier
        .verify(&mechanism, &evidence)
        .expect("strict verifier token");
    assert!(store.apply_verified_promotion(module.id(), &verified));
    assert!(!store.apply_verified_promotion(module.id(), &verified));
    assert_eq!(
        store
            .reusable_modules()
            .find(|stored| stored.id() == module.id())
            .expect("stored module")
            .status(),
        ModuleStatus::Persistent
    );
}
