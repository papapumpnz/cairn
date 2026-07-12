use cairn_world::{
    ActionId, ActiveLatentLimits, ActiveLatentWorldLearner, BeliefState, LatentEpisode,
    LatentError, LatentExperiment, LatentLimits, StateSpace, TransitionObservation, WorldState,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct TruthRule {
    next_latent: u8,
    delta: i64,
}

struct HiddenEnvironment {
    space: StateSpace,
    truth: BTreeMap<(u8, ActionId), TruthRule>,
    initial: Vec<BeliefState>,
    corrupt: bool,
}

impl LatentExperiment for HiddenEnvironment {
    fn observe_continuation(
        &mut self,
        episode_index: usize,
        history: &[TransitionObservation],
        action: ActionId,
    ) -> Result<TransitionObservation, LatentError> {
        let mut state = self
            .initial
            .get(episode_index)
            .cloned()
            .ok_or(LatentError::InvalidInterventionObservation)?;
        for observed in history {
            if observed.before != state.visible {
                return Err(LatentError::InvalidInterventionObservation);
            }
            let next = step(&self.space, &self.truth, &state, observed.action);
            if observed.after != next.visible {
                return Err(LatentError::InvalidInterventionObservation);
            }
            state = next;
        }
        let next = step(&self.space, &self.truth, &state, action);
        Ok(TransitionObservation {
            before: if self.corrupt {
                WorldState(vec![state.visible.0[0] + 1])
            } else {
                state.visible
            },
            action,
            after: next.visible,
        })
    }
}

#[test]
fn actively_selects_the_distinguishing_continuation_and_recovers_latent_law() {
    let (space, actions, truth, initial, episodes) = fixture();
    let mut environment = HiddenEnvironment {
        space: space.clone(),
        truth: truth.clone(),
        initial,
        corrupt: false,
    };
    let report = ActiveLatentWorldLearner
        .induce(
            &space,
            &actions,
            &episodes,
            ActiveLatentLimits {
                induction: LatentLimits::default(),
                maximum_interventions: 1,
            },
            &mut environment,
        )
        .expect("active latent induction");
    assert_eq!(report.interventions.len(), 1);
    assert_eq!(report.induction.theory.latent_states(), 2);
    assert_eq!(report.induction.theory.rules().count(), 6);
    assert_eq!(report.induction.theory.unresolved_successors(), 0);
    assert!(report.induction.successor_ambiguities.is_empty());
    assert_eq!(report.episodes[0].transitions.len(), 5);

    let mut prediction_cases = 0_u64;
    for latent in 0_u8..2 {
        for visible in 3_i64..=27 {
            for action in actions {
                let state = BeliefState {
                    visible: WorldState(vec![visible]),
                    latent,
                };
                assert_eq!(
                    report
                        .induction
                        .theory
                        .predict(&state, action)
                        .expect("learned prediction"),
                    step(&space, &truth, &state, action)
                );
                prediction_cases += 1;
            }
        }
    }
    let uninformed_coverage_actions = u64::try_from(actions.len()).expect("small");
    assert!(
        u64::try_from(report.interventions.len()).expect("small") * 2
            <= uninformed_coverage_actions
    );
    eprintln!(
        "ACTIVE_LATENT latent_states=2 rules=6 interventions=1 uninformed_coverage=3 predictions={prediction_cases}"
    );
}

#[test]
fn intervention_caps_and_corrupt_visible_outcomes_fail_closed() {
    let (space, actions, truth, initial, episodes) = fixture();
    let mut capped = HiddenEnvironment {
        space: space.clone(),
        truth: truth.clone(),
        initial: initial.clone(),
        corrupt: false,
    };
    assert_eq!(
        ActiveLatentWorldLearner.induce(
            &space,
            &actions,
            &episodes,
            ActiveLatentLimits {
                induction: LatentLimits::default(),
                maximum_interventions: 0,
            },
            &mut capped,
        ),
        Err(LatentError::InterventionCapacity { maximum: 0 })
    );
    let mut corrupt = HiddenEnvironment {
        space: space.clone(),
        truth,
        initial,
        corrupt: true,
    };
    assert_eq!(
        ActiveLatentWorldLearner.induce(
            &space,
            &actions,
            &episodes,
            ActiveLatentLimits {
                induction: LatentLimits::default(),
                maximum_interventions: 1,
            },
            &mut corrupt,
        ),
        Err(LatentError::InvalidInterventionObservation)
    );
}

type Fixture = (
    StateSpace,
    [ActionId; 3],
    BTreeMap<(u8, ActionId), TruthRule>,
    Vec<BeliefState>,
    Vec<LatentEpisode>,
);

fn fixture() -> Fixture {
    let space = StateSpace::new(vec![0], vec![30]).expect("space");
    let one = ActionId(4);
    let two = ActionId(9);
    let flip = ActionId(2);
    let actions = [one, two, flip];
    let truth = BTreeMap::from([
        (
            (0, one),
            TruthRule {
                next_latent: 0,
                delta: 1,
            },
        ),
        (
            (0, two),
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
            (1, one),
            TruthRule {
                next_latent: 1,
                delta: -1,
            },
        ),
        (
            (1, two),
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
    let initial = vec![
        BeliefState {
            visible: WorldState(vec![15]),
            latent: 0,
        },
        BeliefState {
            visible: WorldState(vec![9]),
            latent: 0,
        },
    ];
    let episodes = vec![
        episode(&space, &truth, initial[0].clone(), &[one, two, flip, one]),
        episode(
            &space,
            &truth,
            initial[1].clone(),
            &[flip, two, flip, one, two],
        ),
    ];
    (space, actions, truth, initial, episodes)
}

fn episode(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), TruthRule>,
    initial: BeliefState,
    actions: &[ActionId],
) -> LatentEpisode {
    let mut state = initial;
    let transitions = actions
        .iter()
        .map(|action| {
            let next = step(space, truth, &state, *action);
            let transition = TransitionObservation {
                before: state.visible.clone(),
                action: *action,
                after: next.visible.clone(),
            };
            state = next;
            transition
        })
        .collect();
    LatentEpisode { transitions }
}

fn step(
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
