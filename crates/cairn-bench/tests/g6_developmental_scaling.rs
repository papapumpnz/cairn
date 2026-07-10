use cairn_bench::g3_unseen_skill_compositions;
use cairn_curriculum::{DevelopmentalGate, DevelopmentalPoint};
use cairn_invent::{BindingExample, DefinitionInventor, InventionLimits, InventionProblem};
use cairn_memory::{
    MechanismStore, MemoryAssistedInventor, PromotionRunner, SolveOrigin, SupportGeometry,
    VariationDimension,
};
use cairn_rkl::{Binder, ExecLimits, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_skill::{ResourceModel, SkillInducer, SkillPlanner};
use std::collections::BTreeSet;
use std::time::Instant;

type Law = fn(i64) -> i64;

#[test]
#[allow(clippy::too_many_lines)] // Keep the frozen staged protocol visible as one audit.
fn frozen_g6_curve_grows_capability_while_marginal_learning_cost_falls() {
    let increment: Law = |value| value + 1;
    let double: Law = |value| value + value;
    let square: Law = |value| value * value;
    let depth_two: Law = |value| (value + 1) * (value + 1);
    let depth_three: Law = |value| 2 * (value + 1) * (value + 1);
    let depth_four: Law = |value| 2 * (value + 1) * (value + 1) + 1;

    let mut store = MechanismStore::new();
    let mut base_candidates = 0_u64;
    let mut base_examples = 0_u32;
    for (label, law) in [
        ("increment", increment),
        ("double", double),
        ("square", square),
    ] {
        let (module, work, examples) = learn_fresh_pair(label, law);
        base_candidates = base_candidates.saturating_add(work);
        base_examples = base_examples.saturating_add(examples);
        store.insert(module);
    }
    assert_eq!(store.len(), 3);

    let evaluation = [depth_two, depth_three, depth_four];
    let first_solved = bounded_capability(&store, &evaluation);
    assert_eq!(first_solved, 1);

    let (depth_two_module, depth_two_work, depth_two_examples) =
        learn_composed_pair(&store, "depth_two", depth_two);
    store.insert(depth_two_module);
    let second_solved = bounded_capability(&store, &evaluation);
    assert_eq!(second_solved, 2);

    let (fresh_cpu_ns, composed_cpu_ns) = acquisition_cpu(&store, depth_three);
    assert!(composed_cpu_ns.saturating_mul(2) <= fresh_cpu_ns);
    let (depth_three_module, depth_three_work, depth_three_examples) =
        learn_composed_pair(&store, "depth_three", depth_three);
    store.insert(depth_three_module);
    let final_solved = bounded_capability(&store, &evaluation);
    assert_eq!(final_solved, 3);

    let retained = retained_base_behaviors(&store, [increment, double, square]);
    assert_eq!(retained, 3);
    let negative_transfers = unrelated_negative_transfers(&store);
    assert_eq!(negative_transfers, 0);
    let skill_suite = g3_unseen_skill_compositions();
    let base_interventions = u32::try_from(
        skill_suite
            .independent_skill_traces
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            / skill_suite.independent_skill_traces.len(),
    )
    .expect("small curriculum");
    let skills = skill_suite
        .independent_skill_traces
        .iter()
        .map(|traces| {
            SkillInducer
                .induce(
                    traces,
                    ResourceModel {
                        maximum_steps: 8,
                        maximum_cost: 100,
                    },
                )
                .expect("curriculum skill")
        })
        .collect::<Vec<_>>();
    let composed_plan = SkillPlanner
        .plan(
            &skill_suite.scenarios[0].0,
            &skill_suite.scenarios[0].1,
            &skills,
            100,
        )
        .expect("zero-demonstration skill composition");
    assert_eq!(composed_plan.invocations.len(), 4);
    let composition_interventions = 0;

    let points = vec![
        DevelopmentalPoint {
            available_mechanisms: 3,
            solved_evaluation_tasks: first_solved,
            evaluation_tasks: 3,
            candidate_executions_to_acquire_next: base_candidates / 3,
            cpu_nanos_to_acquire_next: fresh_cpu_ns,
            examples_to_acquire_next: base_examples / 3,
            interventions_to_acquire_next: base_interventions,
            maximum_composition_depth: 1,
            retained_prior_tasks: 3,
            prior_tasks: 3,
            negative_transfers: 0,
        },
        DevelopmentalPoint {
            available_mechanisms: 4,
            solved_evaluation_tasks: second_solved,
            evaluation_tasks: 3,
            candidate_executions_to_acquire_next: depth_two_work,
            cpu_nanos_to_acquire_next: composed_cpu_ns,
            examples_to_acquire_next: depth_two_examples,
            interventions_to_acquire_next: composition_interventions,
            maximum_composition_depth: 2,
            retained_prior_tasks: retained,
            prior_tasks: 3,
            negative_transfers,
        },
        DevelopmentalPoint {
            available_mechanisms: 5,
            solved_evaluation_tasks: final_solved,
            evaluation_tasks: 3,
            candidate_executions_to_acquire_next: depth_three_work,
            cpu_nanos_to_acquire_next: composed_cpu_ns,
            examples_to_acquire_next: depth_three_examples,
            interventions_to_acquire_next: composition_interventions,
            maximum_composition_depth: 4,
            retained_prior_tasks: retained,
            prior_tasks: 3,
            negative_transfers,
        },
    ];
    DevelopmentalGate
        .verify(&points)
        .expect("developmental scaling gate");
    assert!(depth_three_work < base_candidates / 3);
    assert!(depth_three_examples < base_examples / 3);
    eprintln!(
        "G6_METRICS capability={first_solved}/3->{second_solved}/3->{final_solved}/3 mechanisms=3->4->5 base_candidates_per_mechanism={} depth2_candidates={depth_two_work} depth3_candidates={depth_three_work} fresh_cpu_ns={fresh_cpu_ns} composed_cpu_ns={composed_cpu_ns} base_examples_per_mechanism={} depth3_examples={depth_three_examples} composition_depth=1->2->4 retained={retained}/3 negative_transfers={negative_transfers}",
        base_candidates / 3,
        base_examples / 3,
    );
}

fn acquisition_cpu(store: &MechanismStore, law: Law) -> (u64, u64) {
    let tasks = [
        problem("timed_a", law, &[-4, -1, 3]),
        problem("timed_b", law, &[-3, 0, 4]),
    ];
    let mut fresh = Vec::new();
    let mut composed = Vec::new();
    for _ in 0..9 {
        let started = Instant::now();
        for task in &tasks {
            DefinitionInventor
                .invent(task)
                .expect("timed fresh acquisition");
        }
        fresh.push(u64::try_from(started.elapsed().as_nanos()).expect("bounded duration"));

        let started = Instant::now();
        for task in &tasks {
            let outcome = MemoryAssistedInventor
                .invent(task, store)
                .expect("timed composed acquisition");
            assert_eq!(outcome.origin, SolveOrigin::MemoryComposition);
        }
        composed.push(u64::try_from(started.elapsed().as_nanos()).expect("bounded duration"));
    }
    fresh.sort_unstable();
    composed.sort_unstable();
    (fresh[fresh.len() / 2], composed[composed.len() / 2])
}

fn learn_fresh_pair(label: &str, law: Law) -> (cairn_memory::MechanismModule, u64, u32) {
    let first = problem(&format!("{label}_a"), law, &[-5, -3, -1, 0, 2]);
    let second = problem(&format!("{label}_b"), law, &[-4, -2, 0, 1, 4]);
    let first = DefinitionInventor.invent(&first).expect("fresh source a");
    let second = DefinitionInventor.invent(&second).expect("fresh source b");
    let work = first
        .proposal
        .candidates_evaluated
        .saturating_add(second.proposal.candidates_evaluated);
    let examples =
        u32::try_from(first.admission.training_examples + second.admission.training_examples)
            .expect("small curriculum");
    let module = PromotionRunner
        .promote(&[first.proposal, second.proposal], support())
        .expect("base promotion");
    (module, work, examples)
}

fn learn_composed_pair(
    store: &MechanismStore,
    label: &str,
    law: Law,
) -> (cairn_memory::MechanismModule, u64, u32) {
    let first = problem(&format!("{label}_a"), law, &[-4, -1, 3]);
    let second = problem(&format!("{label}_b"), law, &[-3, 0, 4]);
    let first = MemoryAssistedInventor
        .invent(&first, store)
        .expect("composed source a");
    let second = MemoryAssistedInventor
        .invent(&second, store)
        .expect("composed source b");
    assert_eq!(first.origin, SolveOrigin::MemoryComposition);
    assert_eq!(second.origin, SolveOrigin::MemoryComposition);
    let work = first
        .metrics
        .total_search_work
        .saturating_add(second.metrics.total_search_work);
    let examples = u32::try_from(
        first.invention.admission.training_examples + second.invention.admission.training_examples,
    )
    .expect("small curriculum");
    let module = PromotionRunner
        .promote(
            &[first.invention.proposal, second.invention.proposal],
            support(),
        )
        .expect("composed promotion");
    (module, work, examples)
}

fn bounded_capability(store: &MechanismStore, laws: &[Law]) -> u32 {
    laws.iter()
        .enumerate()
        .map(|(index, law)| {
            let mut task = problem(&format!("evaluation_{index}"), *law, &[-3, 0, 4]);
            task.limits.maximum_cost = 1;
            task.limits.maximum_candidates = 1;
            u32::from(MemoryAssistedInventor.invent(&task, store).is_ok())
        })
        .sum()
}

fn retained_base_behaviors(store: &MechanismStore, laws: [Law; 3]) -> u32 {
    laws.into_iter()
        .enumerate()
        .map(|(index, law)| {
            let task = problem(&format!("retention_{index}"), law, &[-6, -2, 1, 5]);
            let learned = MemoryAssistedInventor
                .invent(&task, store)
                .expect("retained base behavior");
            u32::from(
                [-7, 6]
                    .into_iter()
                    .all(|input| predict(&learned.invention, &task, input) == law(input)),
            )
        })
        .sum()
}

fn unrelated_negative_transfers(store: &MechanismStore) -> u32 {
    let cube: Law = |value| value * value * value;
    let task = problem("unrelated_cube", cube, &[-4, -2, -1, 0, 2, 3]);
    let learned = MemoryAssistedInventor
        .invent(&task, store)
        .expect("unrelated fallback");
    u32::from(
        learned.origin != SolveOrigin::FreshSearch
            || predict(&learned.invention, &task, 4) != cube(4),
    )
}

fn predict(
    outcome: &cairn_invent::InventionOutcome,
    problem: &InventionProblem,
    input: i64,
) -> i64 {
    let inputs = environment(&problem.parameters[0].name.0, input);
    match DefinitionInventor
        .predict(&outcome.proposal, &inputs, problem.limits.execution)
        .expect("held-out execution")
    {
        Value::Int(value) => value,
        other => panic!("unexpected output {other:?}"),
    }
}

fn problem(name: &str, law: Law, values: &[i64]) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
        parameters: vec![Binder {
            name: VarName(name.to_owned()),
            ty: Type::Int(IntRange {
                lower: -100,
                upper: 100,
            }),
        }],
        return_type: Type::Int(IntRange {
            lower: -1_000_000,
            upper: 1_000_000,
        }),
        literals: vec![Value::Int(1)],
        demonstrations: values
            .iter()
            .map(|value| BindingExample {
                inputs: environment(name, *value),
                output: Value::Int(law(*value)),
            })
            .collect(),
        limits: InventionLimits {
            maximum_cost: 12,
            maximum_candidates: 2_000_000,
            maximum_expression_nodes: 2_000,
            execution: ExecLimits::default(),
        },
    }
}

fn environment(name: &str, value: i64) -> RuntimeEnvironment {
    let mut environment = RuntimeEnvironment::new();
    environment.insert(VarName(name.to_owned()), Value::Int(value));
    environment
}

fn support() -> SupportGeometry {
    SupportGeometry {
        independent_tasks: 2,
        held_out_rebindings: 1,
        varied_dimensions: BTreeSet::from([
            VariationDimension::ParameterNames,
            VariationDimension::NumericRanges,
        ]),
    }
}
