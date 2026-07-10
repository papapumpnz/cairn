use cairn_world::{
    ActionId, BeliefState, GoalObservation, LatentEpisode, LatentError, LatentLimits,
    LatentPlanner, LatentWorldLearner, PlanLimits, StateSpace, TransitionObservation, WorldLearner,
    WorldState,
};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy)]
struct TruthRule {
    next_latent: u8,
    delta: i64,
}

#[test]
#[allow(clippy::too_many_lines)]
fn learns_minimal_hidden_modes_and_plans_through_them() {
    let mut random = 0xd1b5_4a32_d192_ed03_u64;
    let mut exact_predictions = 0_u64;
    let mut exact_plans = 0_u64;
    let mut rejected_one_state_models = 0_u64;
    let mut first_episode_search_nodes = 0_u64;

    for world_index in 0_u16..100 {
        let space = StateSpace::new(vec![0], vec![20]).expect("space");
        let mut actions = [
            ActionId(world_index * 3 + 1),
            ActionId(world_index * 3 + 2),
            ActionId(world_index * 3 + 3),
        ];
        shuffle(&mut actions, &mut random);
        let move_one = actions[0];
        let move_two = actions[1];
        let flip = actions[2];
        let truth = BTreeMap::from([
            (
                (0, move_one),
                TruthRule {
                    next_latent: 0,
                    delta: 1,
                },
            ),
            (
                (0, move_two),
                TruthRule {
                    next_latent: 0,
                    delta: 2,
                },
            ),
            (
                (0, flip),
                TruthRule {
                    next_latent: 1,
                    delta: 0,
                },
            ),
            (
                (1, move_one),
                TruthRule {
                    next_latent: 1,
                    delta: -1,
                },
            ),
            (
                (1, move_two),
                TruthRule {
                    next_latent: 1,
                    delta: -2,
                },
            ),
            (
                (1, flip),
                TruthRule {
                    next_latent: 0,
                    delta: 0,
                },
            ),
        ]);
        let training_actions = [move_one, move_two, flip, move_one, move_two, flip];
        let episode = observed_episode(
            &space,
            &truth,
            BeliefState {
                visible: WorldState(vec![10]),
                latent: 0,
            },
            &training_actions,
        );
        let report = LatentWorldLearner
            .induce(
                &space,
                &actions,
                &[episode],
                LatentLimits {
                    maximum_latent_states: 3,
                    maximum_search_nodes: 100_000,
                },
            )
            .expect("infer hidden-state machine");
        assert_eq!(report.theory.latent_states(), 2);
        assert_eq!(report.rejected_state_counts, 1);
        assert_eq!(report.theory.rules().count(), 6);
        rejected_one_state_models += 1;
        first_episode_search_nodes += report.search_nodes;

        for latent in 0_u8..=1 {
            for visible in 0_i64..=20 {
                for action in actions {
                    let state = BeliefState {
                        visible: WorldState(vec![visible]),
                        latent,
                    };
                    assert_eq!(
                        report
                            .theory
                            .predict(&state, action)
                            .expect("latent prediction"),
                        truth_step(&space, &truth, &state, action)
                    );
                    exact_predictions += 1;
                }
            }
        }

        let initial_value = i64::from(next(&mut random).to_le_bytes()[0] % 21);
        let mut target_value = i64::from(next(&mut random).to_le_bytes()[0] % 21);
        if target_value == initial_value {
            target_value = (target_value + 7) % 21;
        }
        let goal = WorldLearner
            .induce_goal(
                &space,
                &[
                    GoalObservation {
                        state: WorldState(vec![target_value]),
                        achieved: true,
                    },
                    GoalObservation {
                        state: WorldState(vec![initial_value]),
                        achieved: false,
                    },
                ],
            )
            .expect("goal");
        let initial = BeliefState {
            visible: WorldState(vec![initial_value]),
            latent: 0,
        };
        let plan = LatentPlanner
            .plan(&report.theory, &goal, &initial, PlanLimits::default())
            .expect("plan over inferred latent state");
        let terminal = plan
            .actions
            .iter()
            .try_fold(initial.clone(), |state, action| {
                report.theory.predict(&state, *action)
            });
        assert_eq!(
            terminal.expect("execute learned plan").visible,
            WorldState(vec![target_value])
        );
        assert_eq!(
            plan.actions.len(),
            reference_shortest_length(&space, &truth, &initial, target_value)
        );
        exact_plans += 1;
    }

    assert_eq!(rejected_one_state_models, 100);
    assert_eq!(exact_predictions, 12_600);
    assert_eq!(exact_plans, 100);
    assert!(first_episode_search_nodes > 0);
    println!(
        "HIDDEN worlds=100 rejected_one_state=100 inferred_two_state=100 transition_predictions={exact_predictions} shortest_plans={exact_plans} first_episode_search_nodes={first_episode_search_nodes} repeated_episode_search_nodes=0"
    );
}

#[test]
fn one_state_ablation_and_search_caps_fail_closed() {
    let space = StateSpace::new(vec![0], vec![20]).expect("space");
    let forward = ActionId(1);
    let flip = ActionId(2);
    let truth = BTreeMap::from([
        (
            (0, forward),
            TruthRule {
                next_latent: 0,
                delta: 1,
            },
        ),
        (
            (0, flip),
            TruthRule {
                next_latent: 1,
                delta: 0,
            },
        ),
        (
            (1, forward),
            TruthRule {
                next_latent: 1,
                delta: -1,
            },
        ),
        (
            (1, flip),
            TruthRule {
                next_latent: 0,
                delta: 0,
            },
        ),
    ]);
    let episode = observed_episode(
        &space,
        &truth,
        BeliefState {
            visible: WorldState(vec![10]),
            latent: 0,
        },
        &[forward, flip, forward, flip],
    );
    assert_eq!(
        LatentWorldLearner.induce(
            &space,
            &[forward, flip],
            &[episode.clone()],
            LatentLimits {
                maximum_latent_states: 1,
                maximum_search_nodes: 1_000,
            },
        ),
        Err(LatentError::NoTheory {
            maximum_latent_states: 1
        })
    );
    assert_eq!(
        LatentWorldLearner.induce(
            &space,
            &[forward, flip],
            &[episode],
            LatentLimits {
                maximum_latent_states: 2,
                maximum_search_nodes: 1,
            },
        ),
        Err(LatentError::SearchCapacity { maximum: 1 })
    );
}

fn observed_episode(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), TruthRule>,
    initial: BeliefState,
    actions: &[ActionId],
) -> LatentEpisode {
    let mut state = initial;
    let mut transitions = Vec::new();
    for action in actions {
        let next = truth_step(space, truth, &state, *action);
        transitions.push(TransitionObservation {
            before: state.visible,
            action: *action,
            after: next.visible.clone(),
        });
        state = next;
    }
    LatentEpisode { transitions }
}

fn truth_step(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), TruthRule>,
    state: &BeliefState,
    action: ActionId,
) -> BeliefState {
    let rule = truth.get(&(state.latent, action)).expect("truth rule");
    BeliefState {
        visible: WorldState(vec![
            (state.visible.0[0] + rule.delta).clamp(space.lower()[0], space.upper()[0])
        ]),
        latent: rule.next_latent,
    }
}

fn reference_shortest_length(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), TruthRule>,
    initial: &BeliefState,
    target: i64,
) -> usize {
    let actions = truth
        .keys()
        .map(|(_, action)| *action)
        .collect::<std::collections::BTreeSet<_>>();
    let mut frontier = VecDeque::from([(initial.clone(), 0_usize)]);
    let mut visited = std::collections::BTreeSet::from([initial.clone()]);
    while let Some((state, depth)) = frontier.pop_front() {
        if state.visible.0[0] == target {
            return depth;
        }
        for action in &actions {
            let next = truth_step(space, truth, &state, *action);
            if visited.insert(next.clone()) {
                frontier.push_back((next, depth + 1));
            }
        }
    }
    panic!("reachable target")
}

fn shuffle(actions: &mut [ActionId; 3], random: &mut u64) {
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
