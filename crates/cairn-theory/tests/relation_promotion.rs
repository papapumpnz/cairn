use cairn_invent::{BindingExample, DefinitionInventor, InventionLimits, InventionProblem};
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, InventionEvidence, Mechanism, Signature,
    SupportGeometry as MechanismSupport, VariableId,
};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, ModuleStatus, PromotionRunner, SolveOrigin,
    SupportGeometry as MemorySupport, VariationDimension,
};
use cairn_rkl::{Effect, Expr, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_theory::{GuardLimits, InventedRelation, RelationExample, RelationalPredicateInventor};
use cairn_verifier::PromotionVerifier;
use std::collections::{BTreeMap, BTreeSet};

#[test]
#[allow(clippy::too_many_lines)] // Keep the full verifier-gated promotion protocol visible.
fn independently_invented_enclosure_is_promoted_and_rebound() {
    let geometry = [
        ([0, 10, 0, 10], [2, 8, 3, 7], true),
        ([-5, 20, 10, 30], [0, 15, 12, 25], true),
        ([100, 200, -50, 50], [110, 190, -40, 40], true),
        ([0, 10, 0, 10], [-1, 8, 3, 7], false),
        ([0, 10, 0, 10], [2, 11, 3, 7], false),
        ([0, 10, 0, 10], [2, 8, -1, 7], false),
        ([0, 10, 0, 10], [2, 8, 3, 11], false),
        ([0, 10, 0, 10], [0, 8, 3, 7], false),
        ([0, 10, 0, 10], [2, 10, 3, 7], false),
    ];
    let first = invent("outer", "inner", &geometry);
    let shifted = geometry.map(|(outer, inner, related)| {
        (
            outer.map(|value| value + 300),
            inner.map(|value| value + 300),
            related,
        )
    });
    let second = invent("frame", "item", &shifted);
    let module = PromotionRunner
        .promote(
            &[
                first.invention.proposal.clone(),
                second.invention.proposal.clone(),
            ],
            MemorySupport {
                independent_tasks: 2,
                held_out_rebindings: 5,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::ParameterNames,
                    VariationDimension::NumericRanges,
                    VariationDimension::Positions,
                    VariationDimension::Layouts,
                ]),
            },
        )
        .expect("provisional relation");
    let token = PromotionVerifier
        .verify(
            &mechanism(&module, &first),
            &InventionEvidence {
                target_absent_from_named_vocabulary: true,
                source_code_changes: 0,
                independent_experiences: 2,
                initial_success_parts_per_million: 0,
                post_experience_success_parts_per_million: 1_000_000,
                uncompressed_cost: u64::from(first.guard.structural_cost) * 2,
                compressed_cost: u64::from(first.guard.structural_cost),
                held_out_verified_cases: 5,
                candidate_executions_without: first.guard.candidates_evaluated,
                candidate_executions_with: 1,
                examples_without: 9,
                examples_with: 1,
                false_acceptances_without: 0,
                false_acceptances_with: 0,
                actual_reuse_count: 1,
                necessary_reuse_count: 1,
            },
        )
        .expect("strict relation verification");
    let mut store = MechanismStore::new();
    store.insert(module.clone());
    assert!(store.apply_verified_promotion(module.id(), &token));
    assert_eq!(
        store.reusable_modules().next().expect("module").status(),
        ModuleStatus::Persistent
    );

    let target_features = features("container")
        .into_iter()
        .chain(features("object"))
        .collect::<Vec<_>>();
    let target_cases = [
        ([-100, 100, -100, 100], [-90, 90, -80, 80], true),
        ([-100, 100, -100, 100], [-101, 90, -80, 80], false),
        ([-100, 100, -100, 100], [-90, 101, -80, 80], false),
        ([-100, 100, -100, 100], [-90, 90, -101, 80], false),
        ([-100, 100, -100, 100], [-90, 90, -80, 101], false),
    ];
    let one_shot = relation_problem(&target_features, &target_cases[..1]);
    let reused = MemoryAssistedInventor
        .invent(&one_shot, &store)
        .expect("rebound persistent relation");
    assert_eq!(reused.origin, SolveOrigin::MemoryProposal);
    assert_eq!(reused.metrics.total_search_work, 1);
    for (outer, inner, expected) in target_cases {
        let inputs = environment(&target_features, outer, inner);
        assert_eq!(
            DefinitionInventor
                .predict(
                    &reused.invention.proposal,
                    &inputs,
                    one_shot.limits.execution,
                )
                .expect("relation prediction"),
            Value::Bool(expected)
        );
    }
    eprintln!(
        "RELATION_PROMOTION_METRICS sources=2 persistent=1 fresh_candidates={} memory_candidates=1 examples=9->1 held_out=5 exact=5",
        first.guard.candidates_evaluated
    );
}

fn invent(
    prefix_a: &str,
    prefix_b: &str,
    cases: &[([i64; 4], [i64; 4], bool)],
) -> InventedRelation {
    RelationalPredicateInventor
        .invent(
            &features(prefix_a),
            &features(prefix_b),
            &cases
                .iter()
                .map(|(left, right, related)| RelationExample {
                    bindings: raw_bindings(prefix_a, prefix_b, *left, *right),
                    related: *related,
                })
                .collect::<Vec<_>>(),
            GuardLimits {
                maximum_conjuncts: 4,
                maximum_candidates: 200_000,
            },
        )
        .expect("relation invention")
}

fn mechanism(module: &cairn_memory::MechanismModule, learned: &InventedRelation) -> Mechanism {
    let definition = module.definition().clone();
    Mechanism {
        signature: Signature {
            inputs: definition
                .parameters
                .iter()
                .map(|item| item.ty.clone())
                .collect(),
            output: Type::Bool,
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
                DomainClass("relational-diagram".to_owned()),
                DomainClass("interactive-bounds".to_owned()),
            ]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 2,
                maximum: 2,
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
            verified_cases: u64::try_from(learned.invention.admission.training_examples)
                .expect("small suite")
                + 5,
        },
    }
}

fn relation_problem(
    features: &[(VarName, Type)],
    cases: &[([i64; 4], [i64; 4], bool)],
) -> InventionProblem {
    InventionProblem {
        definition_name: "rebound_relation".to_owned(),
        parameters: features
            .iter()
            .map(|(name, ty)| cairn_rkl::Binder {
                name: name.clone(),
                ty: ty.clone(),
            })
            .collect(),
        return_type: Type::Bool,
        literals: Vec::new(),
        demonstrations: cases
            .iter()
            .map(|(outer, inner, expected)| BindingExample {
                inputs: environment(features, *outer, *inner),
                output: Value::Bool(*expected),
            })
            .collect(),
        limits: InventionLimits::default(),
    }
}

fn environment(
    features: &[(VarName, Type)],
    left: [i64; 4],
    right: [i64; 4],
) -> RuntimeEnvironment {
    let mut environment = RuntimeEnvironment::new();
    for ((name, _), value) in features.iter().zip(left.into_iter().chain(right)) {
        environment.insert(name.clone(), Value::Int(value));
    }
    environment
}

fn features(prefix: &str) -> Vec<(VarName, Type)> {
    ["min_row", "max_row", "min_column", "max_column"]
        .into_iter()
        .map(|feature| {
            (
                VarName(format!("{prefix}_{feature}")),
                Type::Int(IntRange {
                    lower: -1_000,
                    upper: 1_000,
                }),
            )
        })
        .collect()
}

fn raw_bindings(
    left_prefix: &str,
    right_prefix: &str,
    left: [i64; 4],
    right: [i64; 4],
) -> BTreeMap<VarName, i64> {
    features(left_prefix)
        .into_iter()
        .map(|(name, _)| name)
        .zip(left)
        .chain(
            features(right_prefix)
                .into_iter()
                .map(|(name, _)| name)
                .zip(right),
        )
        .collect()
}
