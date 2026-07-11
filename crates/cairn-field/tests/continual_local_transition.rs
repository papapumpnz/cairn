use cairn_field::{FiniteField, LocalFieldLearner};
use cairn_invent::{DefinitionInventor, InventionLimits};
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, InventionEvidence, Mechanism, Signature,
    SupportGeometry as MechanismSupport, VariableId,
};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, ModuleStatus, PromotionRunner, SolveOrigin,
    SupportGeometry as MemorySupport, VariationDimension,
};
use cairn_rkl::{Effect, ExecLimits, Expr, VarName};
use cairn_verifier::PromotionVerifier;
use std::collections::BTreeSet;

fn field(rows: u32, columns: u32, marks: &[(usize, i64)]) -> FiniteField {
    let mut values = vec![0; usize::try_from(rows * columns).expect("small field")];
    for (index, value) in marks {
        values[*index] = *value;
    }
    FiniteField {
        shape: vec![rows, columns],
        values,
    }
}

fn shift_right(input: &FiniteField) -> FiniteField {
    let rows = usize::try_from(input.shape[0]).expect("small field");
    let columns = usize::try_from(input.shape[1]).expect("small field");
    let mut values = vec![0; input.values.len()];
    for row in 0..rows {
        for column in 1..columns {
            values[row * columns + column] = input.values[row * columns + column - 1];
        }
    }
    FiniteField {
        shape: input.shape.clone(),
        values,
    }
}

fn shift_left(input: &FiniteField) -> FiniteField {
    let rows = usize::try_from(input.shape[0]).expect("small field");
    let columns = usize::try_from(input.shape[1]).expect("small field");
    let mut values = vec![0; input.values.len()];
    for row in 0..rows {
        for column in 0..columns.saturating_sub(1) {
            values[row * columns + column] = input.values[row * columns + column + 1];
        }
    }
    FiniteField {
        shape: input.shape.clone(),
        values,
    }
}

fn limits(maximum_candidates: u64) -> InventionLimits {
    InventionLimits {
        maximum_cost: 5,
        maximum_candidates,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits::default(),
    }
}

#[test]
#[allow(clippy::too_many_lines)] // The complete promotion and ablation are one experiment.
fn verified_local_law_reuse_reduces_search_and_examples() {
    let first = field(3, 5, &[(6, 2), (8, 4)]);
    let second = field(4, 6, &[(2, 7), (19, 5)]);
    let first_problem = LocalFieldLearner
        .learning_problem(&[(first.clone(), shift_right(&first))], limits(10_000))
        .expect("first independent problem");
    let second_problem = LocalFieldLearner
        .learning_problem(&[(second.clone(), shift_right(&second))], limits(10_000))
        .expect("second independent problem");
    let first_outcome = DefinitionInventor
        .invent(&first_problem)
        .expect("first independent invention");
    let second_outcome = DefinitionInventor
        .invent(&second_problem)
        .expect("second independent invention");
    assert_eq!(
        first_outcome.proposal.definition.body,
        Expr::Var(VarName("west".to_owned()))
    );
    assert_eq!(
        second_outcome.proposal.definition.body,
        Expr::Var(VarName("west".to_owned()))
    );

    let module = PromotionRunner
        .promote(
            &[
                first_outcome.proposal.clone(),
                second_outcome.proposal.clone(),
            ],
            MemorySupport {
                independent_tasks: 2,
                held_out_rebindings: 35,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::NumericRanges,
                    VariationDimension::CategoricalValues,
                    VariationDimension::Counts,
                    VariationDimension::Positions,
                    VariationDimension::Layouts,
                ]),
            },
        )
        .expect("provisional mechanism");

    let frozen = field(3, 7, &[(1, 6), (18, 8)]);
    let frozen_problem = LocalFieldLearner
        .learning_problem(&[(frozen.clone(), shift_right(&frozen))], limits(10_000))
        .expect("frozen counterfactual");
    let fresh = DefinitionInventor
        .invent(&frozen_problem)
        .expect("fresh frozen solve");
    assert_eq!(fresh.proposal.candidates_evaluated, 4);

    let token = PromotionVerifier
        .verify(
            &mechanism(&module),
            &InventionEvidence {
                target_absent_from_named_vocabulary: true,
                source_code_changes: 0,
                independent_experiences: 2,
                initial_success_parts_per_million: 0,
                post_experience_success_parts_per_million: 1_000_000,
                uncompressed_cost: u64::from(first_outcome.proposal.structural_cost) * 2,
                compressed_cost: u64::from(first_outcome.proposal.structural_cost),
                held_out_verified_cases: 35,
                candidate_executions_without: fresh.proposal.candidates_evaluated,
                candidate_executions_with: 1,
                examples_without: u32::try_from(frozen_problem.demonstrations.len())
                    .expect("small field"),
                examples_with: 3,
                false_acceptances_without: 0,
                false_acceptances_with: 0,
                actual_reuse_count: 1,
                necessary_reuse_count: 1,
            },
        )
        .expect("strict promotion evidence");
    let mut store = MechanismStore::new();
    store.insert(module.clone());
    assert!(store.apply_verified_promotion(module.id(), &token));
    assert_eq!(
        store
            .reusable_modules()
            .next()
            .expect("stored module")
            .status(),
        ModuleStatus::Persistent
    );

    let one_shot = field(1, 3, &[(0, 9)]);
    let one_shot_problem = LocalFieldLearner
        .learning_problem(&[(one_shot.clone(), shift_right(&one_shot))], limits(1))
        .expect("one-shot problem");
    assert!(DefinitionInventor.invent(&one_shot_problem).is_err());
    let reused = MemoryAssistedInventor
        .invent(&one_shot_problem, &store)
        .expect("rebound proposal is independently replayed");
    assert_eq!(reused.origin, SolveOrigin::MemoryProposal);
    assert_eq!(reused.metrics.total_search_work, 1);
    let learned = LocalFieldLearner.from_invention(reused.invention);
    let held_out = field(5, 7, &[(0, 3), (17, 8), (33, 4)]);
    assert_eq!(
        LocalFieldLearner
            .predict(&learned, &held_out, ExecLimits::default())
            .expect("held-out prediction"),
        shift_right(&held_out)
    );
    let contrary = field(3, 6, &[(4, 7), (13, 2)]);
    let contrary_problem = LocalFieldLearner
        .learning_problem(&[(contrary.clone(), shift_left(&contrary))], limits(10_000))
        .expect("contrary local law");
    let contrary = MemoryAssistedInventor
        .invent(&contrary_problem, &store)
        .expect("fresh fallback after contextual refutation");
    assert_eq!(contrary.origin, SolveOrigin::FreshSearch);
    assert_eq!(contrary.metrics.memory_proposals_evaluated, 0);
    assert_eq!(contrary.metrics.rejected_memory_proposals, 0);
    eprintln!("CONTINUAL_LOCAL_LAW candidates=4->1 examples=21->3 held_out_cells=35 exact=35");
}

fn mechanism(module: &cairn_memory::MechanismModule) -> Mechanism {
    let definition = module.definition().clone();
    Mechanism {
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
            observed_domains: BTreeSet::from([DomainClass("finite-field".to_owned())]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 3,
                maximum: 35,
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
            verified_cases: 35,
        },
    }
}
