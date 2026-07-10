use cairn_world::{
    ActionId, BeliefState, ExactPlanner, GoalObservation, LatentEpisode, LatentError, LatentLimits,
    LatentWorldLearner, PlanLimits, StateSpace, TransitionObservation, WorldLearner, WorldState,
};
use std::collections::BTreeMap;

#[derive(Clone)]
struct Rule {
    next: u8,
    delta: Vec<i64>,
}

#[test]
fn experiment_1_recovers_minimum_three_state_cycle() {
    let space = StateSpace::new(vec![0], vec![30]).expect("space");
    let move_action = ActionId(91);
    let cycle_action = ActionId(17);
    let truth = cycle_truth(move_action, cycle_action, &[1, 2, -1]);
    let training = episode(
        &space,
        &truth,
        BeliefState {
            visible: WorldState(vec![15]),
            latent: 0,
        },
        &[
            move_action,
            cycle_action,
            move_action,
            cycle_action,
            move_action,
            cycle_action,
            move_action,
        ],
    );
    let report = LatentWorldLearner
        .induce(
            &space,
            &[move_action, cycle_action],
            &[training],
            LatentLimits {
                maximum_latent_states: 4,
                maximum_search_nodes: 100_000,
            },
        )
        .expect("three-state theory");
    assert_eq!(report.theory.latent_states(), 3);
    assert_eq!(report.rejected_state_counts, 2);
    assert_eq!(report.theory.rules().count(), 6);
    assert!(report.successor_ambiguities.is_empty());
    assert_history_matches(
        &space,
        &truth,
        &report.theory,
        BeliefState {
            visible: WorldState(vec![12]),
            latent: 0,
        },
        &[
            cycle_action,
            move_action,
            move_action,
            cycle_action,
            move_action,
            cycle_action,
            move_action,
        ],
    );
    println!(
        "EXPERIMENT_1 latent_states=3 rejected_smaller=2 rules=6 search_nodes={}",
        report.search_nodes
    );
}

#[test]
fn experiment_2_recovers_minimum_four_state_cycle() {
    let space = StateSpace::new(vec![0], vec![50]).expect("space");
    let move_action = ActionId(203);
    let cycle_action = ActionId(11);
    let truth = cycle_truth(move_action, cycle_action, &[1, 2, -1, -2]);
    let training_actions = [
        move_action,
        cycle_action,
        move_action,
        cycle_action,
        move_action,
        cycle_action,
        move_action,
        cycle_action,
        move_action,
    ];
    let report = LatentWorldLearner
        .induce(
            &space,
            &[move_action, cycle_action],
            &[episode(
                &space,
                &truth,
                BeliefState {
                    visible: WorldState(vec![25]),
                    latent: 0,
                },
                &training_actions,
            )],
            LatentLimits {
                maximum_latent_states: 4,
                maximum_search_nodes: 1_000_000,
            },
        )
        .expect("four-state theory");
    assert_eq!(report.theory.latent_states(), 4);
    assert_eq!(report.rejected_state_counts, 3);
    assert_eq!(report.theory.rules().count(), 8);
    assert!(report.successor_ambiguities.is_empty());
    assert_history_matches(
        &space,
        &truth,
        &report.theory,
        BeliefState {
            visible: WorldState(vec![31]),
            latent: 0,
        },
        &[
            cycle_action,
            cycle_action,
            move_action,
            cycle_action,
            move_action,
            move_action,
            cycle_action,
            move_action,
        ],
    );
    println!(
        "EXPERIMENT_2 latent_states=4 rejected_smaller=3 rules=8 search_nodes={}",
        report.search_nodes
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn experiment_3_combines_reset_episodes_and_generalizes_history() {
    let space = StateSpace::new(vec![0], vec![30]).expect("space");
    let one = ActionId(71);
    let two = ActionId(5);
    let flip = ActionId(149);
    let truth = BTreeMap::from([
        (
            (0, one),
            Rule {
                next: 0,
                delta: vec![1],
            },
        ),
        (
            (0, two),
            Rule {
                next: 0,
                delta: vec![2],
            },
        ),
        (
            (0, flip),
            Rule {
                next: 1,
                delta: vec![0],
            },
        ),
        (
            (1, one),
            Rule {
                next: 1,
                delta: vec![-1],
            },
        ),
        (
            (1, two),
            Rule {
                next: 1,
                delta: vec![-2],
            },
        ),
        (
            (1, flip),
            Rule {
                next: 0,
                delta: vec![0],
            },
        ),
    ]);
    let first_actions = vec![one, two, flip, one];
    let second_actions = vec![flip, two, flip, one, two];
    let first = episode(
        &space,
        &truth,
        BeliefState {
            visible: WorldState(vec![15]),
            latent: 0,
        },
        &first_actions,
    );
    let second = episode(
        &space,
        &truth,
        BeliefState {
            visible: WorldState(vec![9]),
            latent: 0,
        },
        &second_actions,
    );
    let ambiguous = LatentWorldLearner
        .induce(
            &space,
            &[one, two, flip],
            &[first, second.clone()],
            LatentLimits::default(),
        )
        .expect("detect terminal ambiguity");
    assert_eq!(ambiguous.successor_ambiguities.len(), 1);
    assert_eq!(ambiguous.theory.unresolved_successors(), 1);
    let request = &ambiguous.successor_ambiguities[0];
    assert_eq!(request.terminal_episode, 0);
    let distinguishing_action = request
        .distinguishing_action
        .expect("one-step distinguishing continuation");
    let mut extended_first_actions = first_actions;
    extended_first_actions.push(distinguishing_action);
    let resolved_first = episode(
        &space,
        &truth,
        BeliefState {
            visible: WorldState(vec![15]),
            latent: 0,
        },
        &extended_first_actions,
    );
    let report = LatentWorldLearner
        .induce(
            &space,
            &[one, two, flip],
            &[resolved_first, second],
            LatentLimits::default(),
        )
        .expect("multi-episode theory after continuation");
    assert_eq!(report.theory.latent_states(), 2);
    assert_eq!(report.theory.rules().count(), 6);
    assert!(report.successor_ambiguities.is_empty());
    assert_eq!(report.theory.unresolved_successors(), 0);
    assert_history_matches(
        &space,
        &truth,
        &report.theory,
        BeliefState {
            visible: WorldState(vec![22]),
            latent: 0,
        },
        &[two, flip, one, two, flip, one, one],
    );
    println!(
        "EXPERIMENT_3 episodes=2 ambiguity_detected=1 continuation_action={} latent_states=2 rules=6 held_out_history_steps=7 search_nodes={}",
        distinguishing_action.0,
        report.search_nodes,
    );
}

#[test]
fn experiment_4_infers_partial_goal_and_ignores_irrelevant_axis() {
    let space = StateSpace::new(vec![0, 0], vec![5, 5]).expect("space");
    let actions = [ActionId(9), ActionId(3), ActionId(27), ActionId(14)];
    let deltas = [[-1_i64, 0_i64], [1, 0], [0, -1], [0, 1]];
    let center = WorldState(vec![2, 2]);
    let observations = actions
        .iter()
        .zip(deltas)
        .map(|(action, delta)| TransitionObservation {
            before: center.clone(),
            action: *action,
            after: WorldState(vec![center.0[0] + delta[0], center.0[1] + delta[1]]),
        })
        .collect::<Vec<_>>();
    let theory = WorldLearner
        .induce_transitions(&space, &actions, &observations)
        .expect("dynamics");
    let goal = WorldLearner
        .induce_goal(
            &space,
            &[
                GoalObservation {
                    state: WorldState(vec![4, 0]),
                    achieved: true,
                },
                GoalObservation {
                    state: WorldState(vec![4, 5]),
                    achieved: true,
                },
                GoalObservation {
                    state: WorldState(vec![3, 0]),
                    achieved: false,
                },
                GoalObservation {
                    state: WorldState(vec![5, 5]),
                    achieved: false,
                },
            ],
        )
        .expect("partial goal");
    assert_eq!(goal.required_values(), &[Some(4), None]);
    let initial = WorldState(vec![0, 3]);
    let plan = ExactPlanner
        .plan(&theory, &goal, &initial, PlanLimits::default())
        .expect("partial-goal plan");
    let terminal = plan.actions.iter().fold(initial, |state, action| {
        theory.predict(&state, *action).expect("plan step")
    });
    assert!(goal.achieved(&terminal));
    assert_eq!(terminal.0[0], 4);
    assert_eq!(plan.actions.len(), 4);
    println!(
        "EXPERIMENT_4 required_goal_dimensions=1 ignored_dimensions=1 shortest_plan_length=4 terminal={terminal:?}"
    );
}

#[test]
fn experiment_5_rejects_irreconcilable_histories() {
    let space = StateSpace::new(vec![0], vec![10]).expect("space");
    let action = ActionId(33);
    let positive = LatentEpisode {
        transitions: vec![TransitionObservation {
            before: WorldState(vec![5]),
            action,
            after: WorldState(vec![6]),
        }],
    };
    let negative = LatentEpisode {
        transitions: vec![TransitionObservation {
            before: WorldState(vec![5]),
            action,
            after: WorldState(vec![4]),
        }],
    };
    let result = LatentWorldLearner.induce(
        &space,
        &[action],
        &[positive, negative],
        LatentLimits {
            maximum_latent_states: 8,
            maximum_search_nodes: 100_000,
        },
    );
    assert_eq!(
        result,
        Err(LatentError::NoTheory {
            maximum_latent_states: 8
        })
    );
    println!("EXPERIMENT_5 contradictory_initial_histories=rejected max_latent_states=8");
}

fn cycle_truth(
    move_action: ActionId,
    cycle_action: ActionId,
    deltas: &[i64],
) -> BTreeMap<(u8, ActionId), Rule> {
    let states = u8::try_from(deltas.len()).expect("small state count");
    let mut truth = BTreeMap::new();
    for (latent, delta) in deltas.iter().enumerate() {
        let latent = u8::try_from(latent).expect("latent");
        truth.insert(
            (latent, move_action),
            Rule {
                next: latent,
                delta: vec![*delta],
            },
        );
        truth.insert(
            (latent, cycle_action),
            Rule {
                next: (latent + 1) % states,
                delta: vec![0],
            },
        );
    }
    truth
}

fn episode(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), Rule>,
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

fn assert_history_matches(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), Rule>,
    theory: &cairn_world::LatentTransitionTheory,
    initial: BeliefState,
    actions: &[ActionId],
) {
    let mut expected = initial.clone();
    let mut actual = initial;
    for action in actions {
        expected = truth_step(space, truth, &expected, *action);
        actual = theory.predict(&actual, *action).expect("learned step");
        assert_eq!(actual, expected);
    }
}

fn truth_step(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), Rule>,
    state: &BeliefState,
    action: ActionId,
) -> BeliefState {
    let rule = truth.get(&(state.latent, action)).expect("truth rule");
    let visible = WorldState(
        state
            .visible
            .0
            .iter()
            .zip(&rule.delta)
            .zip(space.lower().iter().zip(space.upper()))
            .map(|((value, delta), (lower, upper))| (value + delta).clamp(*lower, *upper))
            .collect(),
    );
    BeliefState {
        visible,
        latent: rule.next,
    }
}
