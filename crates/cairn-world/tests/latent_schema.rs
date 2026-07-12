use cairn_world::{
    ActionId, BeliefState, LatentEpisode, LatentLimits, LatentSchemaCompiler, LatentSchemaError,
    LatentSchemaLimits, LatentTransitionTheory, LatentWorldLearner, StateSpace,
    TransitionObservation, WorldState,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Rule {
    next: u8,
    delta: i64,
}

#[test]
fn canonical_schema_ignores_action_ids_bounds_and_source_support() {
    let first = learned_world(0, 30, [ActionId(4), ActionId(9), ActionId(2)], 15);
    let second = learned_world(100, 160, [ActionId(31), ActionId(7), ActionId(19)], 130);
    let first_schema = LatentSchemaCompiler
        .compile(&first, LatentSchemaLimits::default())
        .expect("first schema");
    let second_schema = LatentSchemaCompiler
        .compile(&second, LatentSchemaLimits::default())
        .expect("second schema");
    assert_eq!(first_schema, second_schema);
    assert_eq!(first_schema.latent_states(), 2);
    assert_eq!(first_schema.roles().len(), 3);
    assert_eq!(first_schema.dimensions(), 1);
}

#[test]
fn schema_rebinds_changed_actions_and_layout_by_exact_episode_replay() {
    let source = learned_world(0, 30, [ActionId(4), ActionId(9), ActionId(2)], 15);
    let schema = LatentSchemaCompiler
        .compile(&source, LatentSchemaLimits::default())
        .expect("schema");
    let space = StateSpace::new(vec![200], vec![260]).expect("changed space");
    let actions = [ActionId(41), ActionId(13), ActionId(29)];
    let truth = truth(actions);
    let episodes = ambiguous_episodes(&space, &truth, 230, 218, actions);
    let bindings = LatentSchemaCompiler
        .bind(
            &schema,
            &space,
            &actions,
            &episodes,
            LatentSchemaLimits::default(),
        )
        .expect("current-world replay bindings");
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].replayed_transitions, 9);
    assert_predictions(&space, &truth, &bindings[0].theory, actions);

    assert_eq!(
        LatentSchemaCompiler.bind(
            &schema,
            &space,
            &actions,
            &episodes,
            LatentSchemaLimits {
                maximum_latent_permutations: 10,
                maximum_action_mappings: 1,
            },
        ),
        Err(LatentSchemaError::Capacity)
    );
    let incompatible = StateSpace::new(vec![0, 0], vec![10, 10]).expect("two dimensions");
    assert_eq!(
        LatentSchemaCompiler.bind(
            &schema,
            &incompatible,
            &actions,
            &episodes,
            LatentSchemaLimits::default(),
        ),
        Err(LatentSchemaError::DimensionMismatch)
    );
}

fn learned_world(
    lower: i64,
    upper: i64,
    actions: [ActionId; 3],
    initial: i64,
) -> LatentTransitionTheory {
    let space = StateSpace::new(vec![lower], vec![upper]).expect("space");
    let laws = truth(actions);
    let episode = episode(
        &space,
        &laws,
        BeliefState {
            visible: WorldState(vec![initial]),
            latent: 0,
        },
        &[
            actions[0], actions[1], actions[2], actions[0], actions[1], actions[2], actions[0],
        ],
    );
    LatentWorldLearner
        .induce(&space, &actions, &[episode], LatentLimits::default())
        .expect("complete learned world")
        .theory
}

fn truth(actions: [ActionId; 3]) -> BTreeMap<(u8, ActionId), Rule> {
    let [one, two, flip] = actions;
    BTreeMap::from([
        ((0, one), Rule { next: 0, delta: 1 }),
        ((0, two), Rule { next: 0, delta: 2 }),
        ((0, flip), Rule { next: 1, delta: 0 }),
        ((1, one), Rule { next: 1, delta: -1 }),
        ((1, two), Rule { next: 1, delta: -2 }),
        ((1, flip), Rule { next: 0, delta: 0 }),
    ])
}

fn ambiguous_episodes(
    space: &StateSpace,
    laws: &BTreeMap<(u8, ActionId), Rule>,
    first: i64,
    second: i64,
    actions: [ActionId; 3],
) -> Vec<LatentEpisode> {
    let [one, two, flip] = actions;
    vec![
        episode(
            space,
            laws,
            BeliefState {
                visible: WorldState(vec![first]),
                latent: 0,
            },
            &[one, two, flip, one],
        ),
        episode(
            space,
            laws,
            BeliefState {
                visible: WorldState(vec![second]),
                latent: 0,
            },
            &[flip, two, flip, one, two],
        ),
    ]
}

fn episode(
    space: &StateSpace,
    laws: &BTreeMap<(u8, ActionId), Rule>,
    initial: BeliefState,
    actions: &[ActionId],
) -> LatentEpisode {
    let mut state = initial;
    let transitions = actions
        .iter()
        .map(|action| {
            let next = step(space, laws, &state, *action);
            let observed = TransitionObservation {
                before: state.visible.clone(),
                action: *action,
                after: next.visible.clone(),
            };
            state = next;
            observed
        })
        .collect();
    LatentEpisode { transitions }
}

fn step(
    space: &StateSpace,
    laws: &BTreeMap<(u8, ActionId), Rule>,
    state: &BeliefState,
    action: ActionId,
) -> BeliefState {
    let rule = laws.get(&(state.latent, action)).expect("truth");
    BeliefState {
        visible: WorldState(vec![
            (state.visible.0[0] + rule.delta).clamp(space.lower()[0], space.upper()[0])
        ]),
        latent: rule.next,
    }
}

fn assert_predictions(
    space: &StateSpace,
    truth: &BTreeMap<(u8, ActionId), Rule>,
    theory: &LatentTransitionTheory,
    actions: [ActionId; 3],
) {
    for latent in 0_u8..2 {
        for visible in 205_i64..=255 {
            for action in actions {
                let state = BeliefState {
                    visible: WorldState(vec![visible]),
                    latent,
                };
                assert_eq!(
                    theory.predict(&state, action).expect("prediction"),
                    step(space, truth, &state, action)
                );
            }
        }
    }
}
