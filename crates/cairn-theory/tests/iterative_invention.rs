use cairn_core::Hash32;
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
use cairn_theory::{IterativeMechanismInventor, IterativeTrace};
use cairn_verifier::PromotionVerifier;
use std::collections::BTreeSet;

#[test]
fn transition_traces_are_lifted_into_a_bounded_recursive_mechanism() {
    let traces = vec![
        trace("episode-a", 10, 3, &[10, 7, 4, 1, 1]),
        trace("episode-b", 14, 4, &[14, 10, 6, 2, 2]),
        trace("episode-c", 9, 2, &[9, 7, 5, 3, 1, 1]),
    ];
    let learned = IterativeMechanismInventor
        .invent(&traces, 100, InventionLimits::default())
        .expect("iterative mechanism");
    let body = format!("{:?}", learned.mechanism.proposal.definition.body);
    assert!(body.contains("BoundedFixpoint"));
    for (initial, parameter, expected) in [(23, 5, 3), (97, 9, 7), (64, 8, 0), (7, 20, 7)] {
        let mut inputs = RuntimeEnvironment::new();
        inputs.insert(VarName("initial".to_owned()), Value::Int(initial));
        inputs.insert(VarName("parameter".to_owned()), Value::Int(parameter));
        assert_eq!(
            DefinitionInventor
                .predict(&learned.mechanism.proposal, &inputs, ExecLimits::default(),)
                .expect("held-out iteration"),
            Value::Int(expected)
        );
    }
    assert!(learned.mechanism.proposal.structural_cost < 15);
    eprintln!(
        "ITERATIVE_METRICS episodes=3 transitions=13 step_candidates={} step_cost={} mechanism_cost={} held_out=4 exact=4 body={:?}",
        learned.step.proposal.candidates_evaluated,
        learned.step.proposal.structural_cost,
        learned.mechanism.proposal.structural_cost,
        learned.mechanism.proposal.definition.body
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Keep iterative promotion and held-out replay visible.
fn iterative_mechanism_is_promoted_and_reused_without_an_unrolled_trace() {
    let first = IterativeMechanismInventor
        .invent(
            &[
                trace("first-a", 10, 3, &[10, 7, 4, 1, 1]),
                trace("first-b", 14, 4, &[14, 10, 6, 2, 2]),
                trace("first-c", 9, 2, &[9, 7, 5, 3, 1, 1]),
            ],
            100,
            InventionLimits::default(),
        )
        .expect("first iterative family");
    let second = IterativeMechanismInventor
        .invent(
            &[
                trace("second-a", 31, 6, &[31, 25, 19, 13, 7, 1, 1]),
                trace("second-b", 20, 7, &[20, 13, 6, 6]),
                trace("second-c", 18, 5, &[18, 13, 8, 3, 3]),
            ],
            100,
            InventionLimits::default(),
        )
        .expect("second iterative family");
    let module = PromotionRunner
        .promote(
            &[
                first.mechanism.proposal.clone(),
                second.mechanism.proposal.clone(),
            ],
            MemorySupport {
                independent_tasks: 2,
                held_out_rebindings: 4,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::NumericRanges,
                    VariationDimension::Counts,
                ]),
            },
        )
        .expect("iterative module");
    let token = PromotionVerifier
        .verify(
            &mechanism(&module),
            &InventionEvidence {
                target_absent_from_named_vocabulary: true,
                source_code_changes: 0,
                independent_experiences: 2,
                initial_success_parts_per_million: 0,
                post_experience_success_parts_per_million: 1_000_000,
                uncompressed_cost: u64::from(first.mechanism.proposal.structural_cost) * 2,
                compressed_cost: u64::from(first.mechanism.proposal.structural_cost),
                held_out_verified_cases: 4,
                candidate_executions_without: first.step.proposal.candidates_evaluated,
                candidate_executions_with: 1,
                examples_without: 3,
                examples_with: 1,
                false_acceptances_without: 0,
                false_acceptances_with: 0,
                actual_reuse_count: 1,
                necessary_reuse_count: 1,
            },
        )
        .expect("iterative verification");
    let mut store = MechanismStore::new();
    store.insert(module.clone());
    assert!(store.apply_verified_promotion(module.id(), &token));
    assert_eq!(
        store.reusable_modules().next().expect("module").status(),
        ModuleStatus::Persistent
    );

    let target = one_shot_iteration("starting_value", "period", 23, 5, 3);
    let reused = MemoryAssistedInventor
        .invent(&target, &store)
        .expect("reused iteration");
    assert_eq!(reused.origin, SolveOrigin::MemoryProposal);
    assert_eq!(reused.metrics.total_search_work, 1);
    for (initial, parameter, expected) in [(97, 9, 7), (64, 8, 0), (7, 20, 7), (101, 10, 1)] {
        let mut inputs = RuntimeEnvironment::new();
        inputs.insert(VarName("starting_value".to_owned()), Value::Int(initial));
        inputs.insert(VarName("period".to_owned()), Value::Int(parameter));
        assert_eq!(
            DefinitionInventor
                .predict(&reused.invention.proposal, &inputs, ExecLimits::default(),)
                .expect("held-out iterative reuse"),
            Value::Int(expected)
        );
    }
    assert!(format!("{:?}", reused.invention.proposal.definition.body).contains("BoundedFixpoint"));
    eprintln!(
        "ITERATIVE_PROMOTION_METRICS sources=2 persistent=1 candidates={}->1 examples=3->1 held_out=4 exact=4",
        first.step.proposal.candidates_evaluated
    );
}

fn one_shot_iteration(
    initial_name: &str,
    parameter_name: &str,
    initial: i64,
    parameter: i64,
    output: i64,
) -> InventionProblem {
    let integer = Type::Int(IntRange {
        lower: 0,
        upper: i64::from(u16::MAX),
    });
    let parameters = vec![
        Binder {
            name: VarName(initial_name.to_owned()),
            ty: integer.clone(),
        },
        Binder {
            name: VarName(parameter_name.to_owned()),
            ty: integer.clone(),
        },
    ];
    let mut inputs = RuntimeEnvironment::new();
    inputs.insert(parameters[0].name.clone(), Value::Int(initial));
    inputs.insert(parameters[1].name.clone(), Value::Int(parameter));
    InventionProblem {
        definition_name: "rebound_iteration".to_owned(),
        parameters,
        return_type: integer,
        literals: Vec::new(),
        demonstrations: vec![BindingExample {
            inputs,
            output: Value::Int(output),
        }],
        limits: InventionLimits::default(),
    }
}

fn mechanism(module: &cairn_memory::MechanismModule) -> Mechanism {
    let definition = module.definition().clone();
    Mechanism {
        signature: Signature {
            inputs: definition
                .parameters
                .iter()
                .map(|item| item.ty.clone())
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
            observed_domains: BTreeSet::from([
                DomainClass("numeric-sequence".to_owned()),
                DomainClass("bounded-control-loop".to_owned()),
            ]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 3,
                maximum: 7,
            }]),
            varied_variables: definition
                .parameters
                .iter()
                .map(|parameter| VariableId(parameter.name.0.clone()))
                .collect(),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::new(),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: module.lineage().iter().copied().collect(),
            verifier_semantics: "dual-rkl-reference-bytecode-replay".to_owned(),
            verified_cases: 4,
        },
    }
}

fn trace(episode: &str, initial: i64, parameter: i64, states: &[i64]) -> IterativeTrace {
    IterativeTrace {
        episode: Hash32::digest(episode.as_bytes()),
        initial,
        parameter,
        states: states.to_vec(),
    }
}
