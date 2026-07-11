use cairn_core::Hash32;
use cairn_curriculum::{
    Curriculum, ExperienceId, FrontierCurriculumGenerator, FrontierGap, LearningAxis,
};
use cairn_runtime::{TheoryHypothesis, VersionSpace};
use cairn_world::{ActionId, StateSpace, TransitionObservation, WorldLearner, WorldState};
use std::collections::{BTreeMap, BTreeSet};

type Mapping = [usize; 4];

#[test]
fn active_version_spaces_learn_and_relearn_unknown_action_laws() {
    let mappings = permutations();
    assert_eq!(mappings.len(), 24);
    let effects = [[-1_i64, 0_i64], [1, 0], [0, -1], [0, 1]];
    let mut random = 0x51f1_5eed_1234_abcd_u64;
    let mut active_actions = 0_u64;
    let mut random_actions = 0_u64;
    let mut exact_predictions = 0_u64;
    let mut layout_transfers = 0_u64;
    let mut changed_law_transfers = 0_u64;

    for world in 0_u16..100 {
        let actions = [
            ActionId(world * 4 + 1),
            ActionId(world * 4 + 2),
            ActionId(world * 4 + 3),
            ActionId(world * 4 + 4),
        ];
        let truth = mappings[usize::from(world) % mappings.len()];
        let (learned, interventions) = identify_mapping(actions, truth, &mappings);
        assert_eq!(learned, truth);
        active_actions += interventions;
        random_actions += random_coverage_actions(&mut random);

        let space = StateSpace::new(vec![0, 0], vec![8, 8]).expect("space");
        let theory = induce(&space, actions, learned, &effects);
        for row in 0_i64..=8 {
            for column in 0_i64..=8 {
                let state = WorldState(vec![row, column]);
                for (action_index, action) in actions.iter().enumerate() {
                    assert_eq!(
                        theory.predict(&state, *action).expect("prediction"),
                        apply_effect(&space, &state, effects[truth[action_index]])
                    );
                    exact_predictions += 1;
                }
            }
        }

        let shifted = StateSpace::new(vec![20, 20], vec![28, 28]).expect("shifted space");
        let shifted_theory = theory
            .rebind_space(shifted.clone())
            .expect("layout transfer");
        let shifted_state = WorldState(vec![24, 24]);
        assert_eq!(
            shifted_theory
                .predict(&shifted_state, actions[0])
                .expect("shifted prediction"),
            apply_effect(&shifted, &shifted_state, effects[truth[0]])
        );
        layout_transfers += 1;

        let changed_truth = mappings[(usize::from(world) + 7) % mappings.len()];
        assert_ne!(changed_truth, truth);
        let (changed_learned, _) = identify_mapping(actions, changed_truth, &mappings);
        let changed_theory = induce(&space, actions, changed_learned, &effects);
        let probe = WorldState(vec![4, 4]);
        for (action_index, action) in actions.iter().enumerate() {
            assert_eq!(
                changed_theory
                    .predict(&probe, *action)
                    .expect("changed-law prediction"),
                apply_effect(&space, &probe, effects[changed_truth[action_index]])
            );
        }
        changed_law_transfers += 1;
    }

    assert_eq!(exact_predictions, 32_400);
    assert_eq!(layout_transfers, 100);
    assert_eq!(changed_law_transfers, 100);
    assert!(active_actions.saturating_mul(2) <= random_actions);
    eprintln!(
        "G4_ACTIVE worlds=100 exact_predictions={exact_predictions} active_actions={active_actions} random_actions={random_actions} layout_transfers={layout_transfers} changed_law_transfers={changed_law_transfers}"
    );
}

fn identify_mapping(
    actions: [ActionId; 4],
    truth: Mapping,
    mappings: &[Mapping],
) -> (Mapping, u64) {
    let action_hashes = actions.map(action_hash);
    let mut lookup = BTreeMap::new();
    let hypotheses = mappings
        .iter()
        .map(|mapping| {
            let id = mapping_hash(*mapping);
            lookup.insert(id, *mapping);
            TheoryHypothesis {
                id,
                predicted_outcomes: action_hashes
                    .iter()
                    .enumerate()
                    .map(|(index, action)| (*action, effect_hash(mapping[index])))
                    .collect(),
            }
        })
        .collect();
    let mut space = VersionSpace::new(hypotheses).expect("version space");
    let mut interventions = 0_u64;
    while space.len() > 1 {
        let tests = space.discriminating_tests();
        let gaps = tests
            .iter()
            .filter(|test| test.outcome_partitions > 1)
            .map(|test| FrontierGap {
                id: ExperienceId(
                    action_hashes
                        .iter()
                        .position(|hash| *hash == test.intervention)
                        .expect("modeled action")
                        .to_string(),
                ),
                axis: LearningAxis::CausalLaw,
                competing_hypotheses: u64::try_from(test.outcome_partitions)
                    .expect("small partitions"),
                expected_transfer_microunits: 0,
                prior_exercises: interventions,
                action_cost: 1,
                irreversible_risk: 0,
            })
            .collect::<Vec<_>>();
        let candidates = FrontierCurriculumGenerator.generate(&gaps);
        let selected = Curriculum.select(&candidates).expect("curriculum choice");
        let index = selected.id.0.parse::<usize>().expect("generated index");
        space
            .observe(action_hashes[index], effect_hash(truth[index]))
            .expect("causal observation");
        interventions += 1;
    }
    let id = space.sole_hypothesis_id().expect("resolved mapping");
    (lookup[&id], interventions)
}

fn induce(
    space: &StateSpace,
    actions: [ActionId; 4],
    mapping: Mapping,
    effects: &[[i64; 2]; 4],
) -> cairn_world::TransitionTheory {
    let state = WorldState(vec![4, 4]);
    let observations = actions
        .iter()
        .enumerate()
        .map(|(index, action)| TransitionObservation {
            before: state.clone(),
            action: *action,
            after: apply_effect(space, &state, effects[mapping[index]]),
        })
        .collect::<Vec<_>>();
    WorldLearner
        .induce_transitions(space, &actions, &observations)
        .expect("induced mapping")
}

fn apply_effect(space: &StateSpace, state: &WorldState, effect: [i64; 2]) -> WorldState {
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

fn action_hash(action: ActionId) -> Hash32 {
    Hash32::digest(&action.0.to_be_bytes())
}

fn effect_hash(effect: usize) -> Hash32 {
    Hash32::digest(&[u8::try_from(effect).expect("small effect")])
}

fn mapping_hash(mapping: Mapping) -> Hash32 {
    Hash32::digest(
        &mapping
            .into_iter()
            .map(|value| u8::try_from(value).expect("small mapping"))
            .collect::<Vec<_>>(),
    )
}

fn permutations() -> Vec<Mapping> {
    let mut values = [0, 1, 2, 3];
    let mut output = vec![values];
    while next_permutation(&mut values) {
        output.push(values);
    }
    output
}

fn next_permutation(values: &mut Mapping) -> bool {
    let Some(pivot) = (0..values.len() - 1)
        .rev()
        .find(|index| values[*index] < values[*index + 1])
    else {
        return false;
    };
    let successor = (pivot + 1..values.len())
        .rev()
        .find(|index| values[*index] > values[pivot])
        .expect("permutation successor");
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

fn random_coverage_actions(random: &mut u64) -> u64 {
    let mut seen = BTreeSet::new();
    let mut actions = 0_u64;
    while seen.len() < 4 {
        let sample = u32::try_from(next(random) >> 32).expect("upper word");
        seen.insert(usize::try_from(sample % 4).expect("small action"));
        actions += 1;
    }
    actions
}

fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}
