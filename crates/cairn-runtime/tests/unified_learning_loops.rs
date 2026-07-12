use cairn_consolidator::{ProofGraph, ProofNode, ProofValueType};
use cairn_core::Hash32;
use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_runtime::{CognitionError, CognitionRuntime, InternalCompositionRuntimeError};
use cairn_skill::{Condition, Fluent, ResourceModel, SkillInducer, SuccessfulTrace};
use cairn_world::{
    ActionId, GoalObservation, PlanLimits, StateSpace, TransitionObservation, WorldState,
};
use std::collections::BTreeSet;

#[test]
fn one_runtime_serves_task_world_and_skill_learning_loops() {
    let runtime = CognitionRuntime::new();
    let task = scalar_task();
    let learned = runtime.learn(&task).expect("task-local learning");
    assert_eq!(learned.invention.admission.training_examples, 3);

    let space = StateSpace::new(vec![0], vec![5]).expect("space");
    let increase = ActionId(1);
    let decrease = ActionId(2);
    let world = runtime
        .learn_and_plan_world(
            &space,
            &[increase, decrease],
            &[
                transition(1, increase, 2),
                transition(3, increase, 4),
                transition(3, decrease, 2),
                transition(5, decrease, 4),
            ],
            &[
                GoalObservation {
                    state: WorldState(vec![4]),
                    achieved: true,
                },
                GoalObservation {
                    state: WorldState(vec![1]),
                    achieved: false,
                },
            ],
            &WorldState(vec![1]),
            PlanLimits::default(),
        )
        .expect("interactive learning and planning");
    assert_eq!(world.plan.actions, vec![increase, increase, increase]);

    let skills = [
        skill("first", Fluent(1), Fluent(2)),
        skill("second", Fluent(2), Fluent(3)),
    ];
    let skill_plan = runtime
        .plan_with_skills(
            &BTreeSet::from([Fluent(1)]),
            &Condition {
                required: BTreeSet::from([Fluent(3)]),
                forbidden: BTreeSet::new(),
            },
            &skills,
            10,
        )
        .expect("skill composition");
    assert_eq!(skill_plan.invocations.len(), 2);

    let unavailable = Hash32::digest(b"not-in-persistent-memory");
    assert_eq!(
        runtime.plan_internal_composition(
            &[unavailable, Hash32::digest(b"other")],
            &proof_host(),
            8,
            8
        ),
        Err(CognitionError::InternalComposition(
            InternalCompositionRuntimeError::UnavailableMechanism(unavailable)
        ))
    );
}

fn proof_host() -> ProofGraph {
    ProofGraph {
        episode: Hash32::digest(b"runtime-proof-host"),
        nodes: vec![ProofNode {
            operation: "observe".to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![],
            arguments: vec![],
        }],
        output: 0,
    }
}

fn scalar_task() -> InventionProblem {
    let name = VarName("value".to_owned());
    InventionProblem {
        definition_name: "task_local".to_owned(),
        parameters: vec![Binder {
            name: name.clone(),
            ty: Type::Int(IntRange {
                lower: -10,
                upper: 10,
            }),
        }],
        return_type: Type::Int(IntRange {
            lower: -20,
            upper: 20,
        }),
        literals: vec![Value::Int(1)],
        demonstrations: [-3, 0, 4]
            .into_iter()
            .map(|value| {
                let mut inputs = RuntimeEnvironment::new();
                inputs.insert(name.clone(), Value::Int(value));
                BindingExample {
                    inputs,
                    output: Value::Int(value + 1),
                }
            })
            .collect(),
        limits: InventionLimits::default(),
    }
}

fn transition(before: i64, action: ActionId, after: i64) -> TransitionObservation {
    TransitionObservation {
        before: WorldState(vec![before]),
        action,
        after: WorldState(vec![after]),
    }
}

fn skill(label: &str, before: Fluent, after: Fluent) -> cairn_skill::Skill {
    let primitive = Hash32::digest(label.as_bytes());
    SkillInducer
        .induce(
            &[
                trace(&format!("{label}-a"), before, after, Fluent(100), primitive),
                trace(&format!("{label}-b"), before, after, Fluent(101), primitive),
            ],
            ResourceModel {
                maximum_steps: 2,
                maximum_cost: 10,
            },
        )
        .expect("skill induction")
}

fn trace(
    episode: &str,
    before: Fluent,
    after: Fluent,
    noise: Fluent,
    primitive: Hash32,
) -> SuccessfulTrace {
    SuccessfulTrace {
        episode: Hash32::digest(episode.as_bytes()),
        before: BTreeSet::from([before, noise]),
        primitive_steps: vec![primitive],
        after: BTreeSet::from([before, after, noise]),
    }
}
