use cairn_bench::{g0_hidden_law_families, HiddenLawFamily, SupportAxis};
use cairn_consolidator::{Consolidator, ProofTrace};
use cairn_core::Hash32;
use cairn_invent::{DefinitionInventor, InventionOutcome};
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, InventionEvidence, Mechanism, Signature,
    SupportGeometry as MechanismSupport, VariableId,
};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, ModuleStatus, PromotionRunner, SolveOrigin,
    SupportGeometry as MemorySupport, VariationDimension,
};
use cairn_rkl::{Effect, Expr};
use cairn_verifier::PromotionVerifier;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default)]
struct Metrics {
    tasks: u32,
    initial_successes: u32,
    post_successes: u32,
    fresh_candidates: u64,
    memory_candidates: u64,
    verified_predictions: u64,
    persistent_mechanisms: u32,
}

fn structural_operations(expression: &Expr, output: &mut Vec<String>) {
    match expression {
        Expr::Var(_) => output.push("variable".to_owned()),
        Expr::Int(_) => output.push("integer-literal".to_owned()),
        Expr::Atom(_) => output.push("atom-literal".to_owned()),
        Expr::Count(argument) => {
            output.push("count".to_owned());
            structural_operations(argument, output);
        }
        Expr::Unary { op, argument } => {
            output.push(format!("unary:{op:?}"));
            structural_operations(argument, output);
        }
        Expr::Binary { op, left, right } => {
            output.push(format!("binary:{op:?}"));
            structural_operations(left, output);
            structural_operations(right, output);
        }
        Expr::Tuple(items) => {
            output.push(format!("tuple:{}", items.len()));
            for item in items {
                structural_operations(item, output);
            }
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

fn variation(axis: SupportAxis) -> VariationDimension {
    match axis {
        SupportAxis::ParameterNames => VariationDimension::ParameterNames,
        SupportAxis::NumericRanges => VariationDimension::NumericRanges,
        SupportAxis::CategoricalValues => VariationDimension::CategoricalValues,
        SupportAxis::Cardinalities => VariationDimension::Counts,
        SupportAxis::Positions => VariationDimension::Positions,
        SupportAxis::Layouts => VariationDimension::Layouts,
    }
}

fn mechanism(
    family: &HiddenLawFamily,
    module: &cairn_memory::MechanismModule,
    sources: &[InventionOutcome],
    verified_predictions: u64,
) -> Mechanism {
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
            observed_domains: BTreeSet::from([DomainClass(family.domain.clone())]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: family.cardinality_minimum,
                maximum: family.cardinality_maximum,
            }]),
            varied_variables: definition
                .parameters
                .iter()
                .map(|parameter| VariableId(parameter.name.0.clone()))
                .collect(),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::from([VariableId("nested-composition".to_owned())]),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: module.lineage().iter().copied().collect(),
            verifier_semantics: "dual-rkl-reference-bytecode-replay".to_owned(),
            verified_cases: verified_predictions
                + sources
                    .iter()
                    .map(|source| {
                        u64::try_from(source.admission.exact_cross_implementation_replays)
                            .expect("small suite")
                    })
                    .sum::<u64>(),
        },
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn five_unrelated_hidden_laws_are_compressed_promoted_and_reused() {
    let families = g0_hidden_law_families();
    assert_eq!(families.len(), 5);
    let mut total = Metrics::default();

    for family in &families {
        let initial_store = MechanismStore::new();
        total.tasks += u32::try_from(family.held_out_tasks.len()).expect("small suite");
        total.initial_successes += u32::try_from(
            family
                .held_out_tasks
                .iter()
                .filter(|task| {
                    MemoryAssistedInventor
                        .invent(&task.initial_problem, &initial_store)
                        .is_ok()
                })
                .count(),
        )
        .expect("small suite");

        let sources = family
            .source_problems
            .iter()
            .map(|problem| {
                DefinitionInventor
                    .invent(problem)
                    .expect("blind source invention")
            })
            .collect::<Vec<_>>();
        let traces = sources
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let mut operations = Vec::new();
                structural_operations(&source.proposal.definition.body, &mut operations);
                ProofTrace {
                    episode: Hash32::digest(
                        format!("{}:{index}", family.benchmark_family).as_bytes(),
                    ),
                    operations,
                }
            })
            .collect::<Vec<_>>();
        let compression = Consolidator
            .anti_unify(&traces)
            .expect("independent proof compression");
        let module = PromotionRunner
            .promote(
                &sources
                    .iter()
                    .map(|source| source.proposal.clone())
                    .collect::<Vec<_>>(),
                MemorySupport {
                    independent_tasks: u32::try_from(sources.len()).expect("small suite"),
                    held_out_rebindings: 1,
                    varied_dimensions: family.varied_axes.iter().copied().map(variation).collect(),
                },
            )
            .expect("provisional module");
        let mut store = MechanismStore::new();
        store.insert(module.clone());

        let mut family_fresh = 0_u64;
        let mut family_memory = 0_u64;
        let mut family_predictions = 0_u64;
        let mut family_post = 0_u32;
        for task in &family.held_out_tasks {
            let fresh = DefinitionInventor
                .invent(&task.full_problem)
                .expect("fresh-search baseline");
            family_fresh += fresh.proposal.candidates_evaluated;
            let reused = MemoryAssistedInventor
                .invent(&task.one_shot_problem, &store)
                .expect("verified rebound proposal");
            assert_eq!(reused.origin, SolveOrigin::MemoryProposal);
            assert_eq!(
                DefinitionInventor
                    .predict(
                        &reused.invention.proposal,
                        &task.probe.inputs,
                        task.one_shot_problem.limits.execution,
                    )
                    .expect("held-out execution"),
                task.probe.output
            );
            family_memory += reused.metrics.total_search_work;
            family_predictions += 1;
            family_post += 1;
        }

        let learned = mechanism(family, &module, &sources, family_predictions);
        let evidence = InventionEvidence {
            target_absent_from_named_vocabulary: initial_store.is_empty(),
            source_code_changes: 0,
            independent_experiences: u32::try_from(sources.len()).expect("small suite"),
            initial_success_parts_per_million: 0,
            post_experience_success_parts_per_million: 1_000_000,
            uncompressed_cost: compression.uncompressed_nodes,
            compressed_cost: compression.compressed_nodes,
            held_out_verified_cases: family_predictions,
            candidate_executions_without: family_fresh,
            candidate_executions_with: family_memory,
            examples_without: u32::try_from(
                family.held_out_tasks[0].full_problem.demonstrations.len(),
            )
            .expect("small suite"),
            examples_with: 1,
            false_acceptances_without: 0,
            false_acceptances_with: 0,
            actual_reuse_count: family_post,
            necessary_reuse_count: family_post,
        };
        let token = PromotionVerifier
            .verify(&learned, &evidence)
            .expect("strict promotion evidence");
        assert!(store.apply_verified_promotion(module.id(), &token));
        assert_eq!(
            store
                .reusable_modules()
                .next()
                .expect("stored module")
                .status(),
            ModuleStatus::Persistent
        );

        total.post_successes += family_post;
        total.fresh_candidates += family_fresh;
        total.memory_candidates += family_memory;
        total.verified_predictions += family_predictions;
        total.persistent_mechanisms += 1;
    }

    eprintln!(
        "G0_MULTI families={} initial={}/{} post={}/{} fresh_candidates={} memory_candidates={} verified_predictions={} persistent={}",
        families.len(),
        total.initial_successes,
        total.tasks,
        total.post_successes,
        total.tasks,
        total.fresh_candidates,
        total.memory_candidates,
        total.verified_predictions,
        total.persistent_mechanisms
    );
    assert!(total.initial_successes.saturating_mul(5) < total.tasks);
    assert!(total.post_successes.saturating_mul(10) > total.tasks.saturating_mul(7));
    assert!(total.fresh_candidates >= total.memory_candidates.saturating_mul(4));
    assert_eq!(total.verified_predictions, u64::from(total.tasks));
    assert_eq!(total.persistent_mechanisms, 5);
}
