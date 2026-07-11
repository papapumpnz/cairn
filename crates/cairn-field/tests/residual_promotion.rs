use cairn_field::{FiniteField, LearnedResidualConstructor, ResidualConstructorInventor};
use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
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

#[test]
#[allow(clippy::too_many_lines)] // Keep strict promotion and effect replay together.
fn residual_emission_rule_promotes_and_rebinds_to_new_geometry_and_value() {
    let first = ResidualConstructorInventor
        .invent(
            &[(blank(3), diagonal(3, 4)), (blank(5), diagonal(5, 4))],
            0,
            InventionLimits::default(),
        )
        .expect("first residual family");
    let second = ResidualConstructorInventor
        .invent(
            &[(blank(4), diagonal(4, 9)), (blank(6), diagonal(6, 9))],
            0,
            InventionLimits::default(),
        )
        .expect("second residual family");
    let module = PromotionRunner
        .promote(
            &[first.proposal.clone(), second.proposal.clone()],
            MemorySupport {
                independent_tasks: 2,
                held_out_rebindings: 49,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::NumericRanges,
                    VariationDimension::CategoricalValues,
                    VariationDimension::Counts,
                    VariationDimension::Positions,
                    VariationDimension::Layouts,
                ]),
            },
        )
        .expect("residual module");
    let token = PromotionVerifier
        .verify(
            &mechanism(&module),
            &InventionEvidence {
                target_absent_from_named_vocabulary: true,
                source_code_changes: 0,
                independent_experiences: 2,
                initial_success_parts_per_million: 0,
                post_experience_success_parts_per_million: 1_000_000,
                uncompressed_cost: u64::from(first.proposal.structural_cost) * 2,
                compressed_cost: u64::from(first.proposal.structural_cost),
                held_out_verified_cases: 49,
                candidate_executions_without: first.proposal.candidates_evaluated,
                candidate_executions_with: 1,
                examples_without: u32::try_from(first.training_cells).expect("small suite"),
                examples_with: 1,
                false_acceptances_without: 0,
                false_acceptances_with: 0,
                actual_reuse_count: 1,
                necessary_reuse_count: 1,
            },
        )
        .expect("strict residual verification");
    let mut store = MechanismStore::new();
    store.insert(module.clone());
    assert!(store.apply_verified_promotion(module.id(), &token));
    assert_eq!(
        store.reusable_modules().next().expect("module").status(),
        ModuleStatus::Persistent
    );

    let one_shot = one_shot_problem(7);
    let reused = MemoryAssistedInventor
        .invent(&one_shot, &store)
        .expect("one-shot residual predicate");
    assert_eq!(reused.origin, SolveOrigin::MemoryProposal);
    assert_eq!(reused.metrics.total_search_work, 1);
    let rebound = LearnedResidualConstructor {
        proposal: reused.invention.proposal,
        background: 0,
        paint: 6,
        training_cells: 1,
        residual_cells: 1,
    };
    let prediction = ResidualConstructorInventor
        .predict(&rebound, &blank(7), ExecLimits::default())
        .expect("rebound residual constructor");
    assert_eq!(prediction, diagonal(7, 6));
    eprintln!(
        "RESIDUAL_PROMOTION_METRICS sources=2 persistent=1 candidates={}->1 examples={}->1 held_out_cells=49 exact=49",
        first.proposal.candidates_evaluated,
        first.training_cells
    );
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
                DomainClass("vector-canvas".to_owned()),
                DomainClass("construction-world".to_owned()),
            ]),
            observed_cardinalities: BTreeSet::from([
                CardinalityRange {
                    minimum: 3,
                    maximum: 5,
                },
                CardinalityRange {
                    minimum: 4,
                    maximum: 6,
                },
            ]),
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
            verified_cases: 49,
        },
    }
}

fn one_shot_problem(size: u32) -> InventionProblem {
    let parameters = ["row", "column", "rows", "columns"]
        .into_iter()
        .map(|name| Binder {
            name: VarName(name.to_owned()),
            ty: Type::Int(IntRange {
                lower: 0,
                upper: i64::from(u16::MAX),
            }),
        })
        .collect::<Vec<_>>();
    let mut inputs = RuntimeEnvironment::new();
    for (parameter, value) in parameters.iter().zip([0, 0, size, size]) {
        inputs.insert(parameter.name.clone(), Value::Int(i64::from(value)));
    }
    InventionProblem {
        definition_name: "rebound_residual".to_owned(),
        parameters,
        return_type: Type::Bool,
        literals: vec![Value::Int(0), Value::Int(1)],
        demonstrations: vec![BindingExample {
            inputs,
            output: Value::Bool(true),
        }],
        limits: InventionLimits::default(),
    }
}

fn blank(size: u32) -> FiniteField {
    FiniteField {
        shape: vec![size, size],
        values: vec![0; usize::try_from(size * size).expect("small fixture")],
    }
}

fn diagonal(size: u32, paint: i64) -> FiniteField {
    let mut field = blank(size);
    let width = usize::try_from(size).expect("small fixture");
    for index in 0..width {
        field.values[index * width + index] = paint;
    }
    field
}
