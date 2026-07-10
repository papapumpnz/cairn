use cairn_world::{
    ActionId, ExactPlanner, GoalObservation, OptionSkill, PlanLimits, StateSpace,
    TransitionObservation, TransitionTheory, WorldError, WorldLearner, WorldState,
};
use std::collections::BTreeMap;

#[test]
#[allow(clippy::too_many_lines)]
fn frozen_gll4_learns_unknown_worlds_plans_improves_and_transfers() {
    let mut random = 0x9e37_79b9_7f4a_7c15_u64;
    let mut exact_transition_predictions = 0_u64;
    let mut exact_goal_classifications = 0_u64;
    let mut exact_plans = 0_u64;
    let mut exact_option_transfers = 0_u64;
    let mut first_episode_actions = 0_u64;
    let mut repeated_episode_actions = 0_u64;

    for world_index in 0_u16..100 {
        let space = StateSpace::new(vec![0, 0], vec![8, 8]).expect("space");
        let mut actions = [
            ActionId(world_index * 4 + 1),
            ActionId(world_index * 4 + 2),
            ActionId(world_index * 4 + 3),
            ActionId(world_index * 4 + 4),
        ];
        shuffle(&mut actions, &mut random);
        let effects = [[-1_i64, 0_i64], [1, 0], [0, -1], [0, 1]];
        let truth = actions
            .iter()
            .copied()
            .zip(effects)
            .collect::<BTreeMap<_, _>>();

        // The learner chooses one safe intervention per unknown action. Action
        // identifiers have freshly permuted meanings in every world.
        let experiment_state = WorldState(vec![4, 4]);
        let interventions = WorldLearner
            .propose_interventions(&actions, &[])
            .expect("select discriminating interventions");
        let observations = interventions
            .iter()
            .map(|action| TransitionObservation {
                before: experiment_state.clone(),
                action: *action,
                after: truth_transition(&space, &truth, &experiment_state, *action),
            })
            .collect::<Vec<_>>();
        assert!(WorldLearner
            .propose_interventions(&actions, &observations)
            .expect("causal evidence complete")
            .is_empty());
        let theory = WorldLearner
            .induce_transitions(&space, &actions, &observations)
            .expect("induce unknown transition laws");

        for row in 0_i64..=8 {
            for column in 0_i64..=8 {
                let state = WorldState(vec![row, column]);
                for action in actions {
                    assert_eq!(
                        theory.predict(&state, action).expect("prediction"),
                        truth_transition(&space, &truth, &state, action)
                    );
                    exact_transition_predictions += 1;
                }
            }
        }

        let start = generated_state(&mut random);
        let mut target = generated_state(&mut random);
        if target == start {
            target.0[0] = (target.0[0] + 1) % 9;
        }
        let negative = if experiment_state == target {
            WorldState(vec![0, 0])
        } else {
            experiment_state.clone()
        };
        let goal = WorldLearner
            .induce_goal(
                &space,
                &[
                    GoalObservation {
                        state: target.clone(),
                        achieved: true,
                    },
                    GoalObservation {
                        state: start.clone(),
                        achieved: false,
                    },
                    GoalObservation {
                        state: negative,
                        achieved: false,
                    },
                ],
            )
            .expect("infer goal predicate");
        for row in 0_i64..=8 {
            for column in 0_i64..=8 {
                let state = WorldState(vec![row, column]);
                assert_eq!(goal.achieved(&state), state == target);
                exact_goal_classifications += 1;
            }
        }

        let plan = ExactPlanner
            .plan(&theory, &goal, &start, PlanLimits::default())
            .expect("construct plan through learned theory");
        let terminal = execute(&theory, &start, &plan.actions);
        assert_eq!(terminal, target);
        assert_eq!(
            plan.actions.len(),
            manhattan_distance(&start, &target),
            "plan must be shortest"
        );
        exact_plans += 1;

        let exploration_actions = u64::try_from(interventions.len()).expect("action count");
        let plan_actions = u64::try_from(plan.actions.len()).expect("plan count");
        first_episode_actions += exploration_actions + plan_actions;
        repeated_episode_actions += plan_actions;
        assert!(exploration_actions + plan_actions > plan_actions);

        let option = OptionSkill::induce(&start, &plan, &theory).expect("induce option");
        let shifted_space = StateSpace::new(vec![20, 20], vec![28, 28]).expect("shifted layout");
        let shifted_theory = theory
            .rebind_space(shifted_space)
            .expect("rebind world model");
        let shifted_start = shift(&start, 20);
        let shifted_target = shift(&target, 20);
        assert_eq!(
            option
                .transfer(&shifted_start, &shifted_theory)
                .expect("transfer learned option"),
            shifted_target
        );
        exact_option_transfers += 1;
    }

    assert_eq!(exact_transition_predictions, 32_400);
    assert_eq!(exact_goal_classifications, 8_100);
    assert_eq!(exact_plans, 100);
    assert_eq!(exact_option_transfers, 100);
    assert!(repeated_episode_actions < first_episode_actions);
    let saved_actions = first_episode_actions - repeated_episode_actions;
    println!(
        "GLL4 worlds=100 transition_predictions={exact_transition_predictions} goal_classifications={exact_goal_classifications} plans={exact_plans} option_transfers={exact_option_transfers} first_episode_actions={first_episode_actions} repeated_episode_actions={repeated_episode_actions} saved_actions={saved_actions}"
    );
}

#[test]
fn inconsistent_worlds_and_resource_caps_fail_closed() {
    let space = StateSpace::new(vec![0], vec![10]).expect("space");
    let action = ActionId(7);
    let inconsistent = WorldLearner.induce_transitions(
        &space,
        &[action],
        &[
            TransitionObservation {
                before: WorldState(vec![2]),
                action,
                after: WorldState(vec![3]),
            },
            TransitionObservation {
                before: WorldState(vec![5]),
                action,
                after: WorldState(vec![7]),
            },
        ],
    );
    assert_eq!(
        inconsistent,
        Err(WorldError::InconsistentActionEffect(action))
    );

    let theory = WorldLearner
        .induce_transitions(
            &space,
            &[action],
            &[TransitionObservation {
                before: WorldState(vec![2]),
                action,
                after: WorldState(vec![3]),
            }],
        )
        .expect("theory");
    let goal = WorldLearner
        .induce_goal(
            &space,
            &[
                GoalObservation {
                    state: WorldState(vec![10]),
                    achieved: true,
                },
                GoalObservation {
                    state: WorldState(vec![0]),
                    achieved: false,
                },
            ],
        )
        .expect("goal");
    assert_eq!(
        ExactPlanner.plan(
            &theory,
            &goal,
            &WorldState(vec![0]),
            PlanLimits {
                maximum_visited_states: 3,
                maximum_plan_length: 20,
            },
        ),
        Err(WorldError::PlanningCapacity { maximum: 3 })
    );
}

fn truth_transition(
    space: &StateSpace,
    truth: &BTreeMap<ActionId, [i64; 2]>,
    state: &WorldState,
    action: ActionId,
) -> WorldState {
    let effect = truth.get(&action).expect("known truth action");
    WorldState(
        state
            .0
            .iter()
            .zip(effect)
            .zip(space.lower().iter().zip(space.upper()))
            .map(|((value, delta), (lower, upper))| (value + delta).clamp(*lower, *upper))
            .collect(),
    )
}

fn execute(theory: &TransitionTheory, initial: &WorldState, actions: &[ActionId]) -> WorldState {
    actions.iter().fold(initial.clone(), |state, action| {
        theory.predict(&state, *action).expect("execute plan")
    })
}

fn generated_state(random: &mut u64) -> WorldState {
    WorldState(vec![
        i64::from(next(random).to_le_bytes()[0] % 9),
        i64::from(next(random).to_le_bytes()[0] % 9),
    ])
}

fn manhattan_distance(left: &WorldState, right: &WorldState) -> usize {
    left.0
        .iter()
        .zip(&right.0)
        .map(|(left, right)| usize::try_from(left.abs_diff(*right)).expect("bounded distance"))
        .sum()
}

fn shift(state: &WorldState, amount: i64) -> WorldState {
    WorldState(state.0.iter().map(|value| value + amount).collect())
}

fn shuffle(actions: &mut [ActionId; 4], random: &mut u64) {
    for upper in (1..actions.len()).rev() {
        let index = usize::from(next(random).to_le_bytes()[0]) % (upper + 1);
        actions.swap(upper, index);
    }
}

fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}
