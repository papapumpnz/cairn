use cairn_curriculum::{
    Curriculum, ExperienceId, FrontierCurriculumGenerator, FrontierGap, LearningAxis,
};
use cairn_memory::{LatentMechanismStore, ModuleStatus};
use cairn_runtime::CognitionRuntime;
use cairn_verifier::LatentSchemaVerifier;
use cairn_world::{
    ActionId, BeliefState, LatentEpisode, LatentLimits, LatentMechanismSchema,
    LatentSchemaCompiler, LatentSchemaLimits, LatentTransitionTheory, LatentWorldLearner,
    StateSpace, TransitionObservation, WorldState,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
struct Structure {
    latent_states: u8,
    actions: u16,
    dimensions: usize,
}

#[derive(Clone, Debug)]
struct Rule {
    next: u8,
    delta: Vec<i64>,
}

#[derive(Clone)]
struct World {
    space: StateSpace,
    actions: Vec<ActionId>,
    rules: BTreeMap<(u8, ActionId), Rule>,
    episode: LatentEpisode,
}

const STRUCTURES: [Structure; 5] = [
    Structure {
        latent_states: 1,
        actions: 2,
        dimensions: 1,
    },
    Structure {
        latent_states: 2,
        actions: 3,
        dimensions: 1,
    },
    Structure {
        latent_states: 3,
        actions: 3,
        dimensions: 1,
    },
    Structure {
        latent_states: 2,
        actions: 4,
        dimensions: 2,
    },
    Structure {
        latent_states: 4,
        actions: 3,
        dimensions: 2,
    },
];

#[test]
#[allow(clippy::too_many_lines)] // One frozen end-to-end developmental learning curve.
fn five_structurally_distinct_world_mechanisms_promote_and_rebind() {
    let mut store = LatentMechanismStore::new();
    let mut schemas = BTreeMap::new();
    let mut fresh_search_nodes = 0_u64;
    let mut curriculum_order = Vec::new();
    let mut remaining = (0..STRUCTURES.len()).collect::<Vec<_>>();

    while !remaining.is_empty() {
        let gaps = remaining
            .iter()
            .map(|index| {
                let structure = STRUCTURES[*index];
                FrontierGap {
                    id: ExperienceId(index.to_string()),
                    axis: LearningAxis::CausalLaw,
                    competing_hypotheses: u64::from(structure.latent_states)
                        .saturating_mul(u64::from(structure.actions)),
                    expected_transfer_microunits: u64::try_from(structure.dimensions)
                        .expect("small")
                        .saturating_mul(1_000_000),
                    prior_exercises: 0,
                    action_cost: u64::from(structure.actions),
                    irreversible_risk: 0,
                }
            })
            .collect::<Vec<_>>();
        let candidates = FrontierCurriculumGenerator.generate(&gaps);
        let selected = Curriculum
            .select(&candidates)
            .expect("curriculum selection");
        let index = selected.id.0.parse::<usize>().expect("generated index");
        let position = remaining
            .iter()
            .position(|candidate| *candidate == index)
            .expect("remaining objective");
        remaining.remove(position);
        curriculum_order.push(index);
        let structure = STRUCTURES[index];
        let first = world(structure, u16::try_from(20 * index).expect("small"), 0);
        let second = world(
            structure,
            u16::try_from(100 + 20 * index).expect("small"),
            10_000,
        );
        let first_schema = learn_verify_record(&mut store, &first);
        let second_schema = learn_verify_record(&mut store, &second);
        assert_eq!(first_schema, second_schema);
        let module = store.module(first_schema.semantic_hash()).expect("module");
        assert_eq!(module.status(), ModuleStatus::Persistent);
        assert_eq!(
            module.support_geometry(),
            cairn_memory::LatentSupportGeometry {
                visible_dimensions: u16::try_from(structure.dimensions).expect("small"),
                latent_states: structure.latent_states,
                action_roles: usize::from(structure.actions),
                independently_verified_worlds: 2,
            }
        );
        schemas.insert(index, first_schema);
    }
    assert_eq!(store.persistent_schemas().count(), 5);
    assert_eq!(curriculum_order.len(), 5);

    let runtime = CognitionRuntime::with_latent_mechanism_memory(store);
    let mut full_transitions = 0_u64;
    let mut memory_transitions = 0_u64;
    for (index, structure) in STRUCTURES.into_iter().enumerate() {
        let held_out = world(
            structure,
            u16::try_from(300 + 20 * index).expect("small"),
            -10_000,
        );
        let fresh = LatentWorldLearner
            .induce(
                &held_out.space,
                &held_out.actions,
                std::slice::from_ref(&held_out.episode),
                latent_limits(),
            )
            .expect("fresh structural induction");
        fresh_search_nodes = fresh_search_nodes
            .checked_add(fresh.search_nodes)
            .expect("bounded sum");
        let schema = &schemas[&index];
        let mut selected = None;
        for length in 1..=held_out.episode.transitions.len() {
            let partial = LatentEpisode {
                transitions: held_out.episode.transitions[..length].to_vec(),
            };
            if let Ok(outcome) = runtime.propose_latent_world_from_memory(
                &held_out.space,
                &held_out.actions,
                std::slice::from_ref(&partial),
                LatentSchemaLimits::default(),
            ) {
                let matching = outcome
                    .proposals
                    .into_iter()
                    .filter(|proposal| proposal.schema == schema.semantic_hash())
                    .collect::<Vec<_>>();
                if matching.len() == 1 {
                    selected = Some((partial, matching.into_iter().next().expect("one")));
                    break;
                }
            }
        }
        let (partial, proposal) = selected.expect("minimal persistent binding");
        assert!(partial.transitions.len() < held_out.episode.transitions.len());
        full_transitions += u64::try_from(held_out.episode.transitions.len()).expect("small");
        memory_transitions += u64::try_from(partial.transitions.len()).expect("small");
        let outcome = runtime
            .propose_latent_world_from_memory(
                &held_out.space,
                &held_out.actions,
                std::slice::from_ref(&partial),
                LatentSchemaLimits::default(),
            )
            .expect("persistent structural proposal");
        assert_eq!(outcome.persistent_schemas_attempted, 5);
        LatentSchemaVerifier
            .verify_transfer(
                schema,
                &held_out.space,
                &held_out.actions,
                std::slice::from_ref(&partial),
                &proposal.binding,
            )
            .expect("held-out independent replay");
        assert_predictions(&held_out, &fresh.theory);
        assert_predictions(&held_out, &proposal.binding.theory);
    }
    assert!(memory_transitions < full_transitions);

    eprintln!(
        "G2_STRUCTURAL_LATENT mechanisms=5 source_worlds=10 heldout_worlds=5 curriculum={curriculum_order:?} fresh_search_nodes={fresh_search_nodes} transitions={full_transitions}->{memory_transitions} memory_exact_bindings=5 negative_transfer=0"
    );
}

fn learn_verify_record(store: &mut LatentMechanismStore, world: &World) -> LatentMechanismSchema {
    let report = LatentWorldLearner
        .induce(
            &world.space,
            &world.actions,
            std::slice::from_ref(&world.episode),
            latent_limits(),
        )
        .expect("learn source structure");
    let schema = LatentSchemaCompiler
        .compile(&report.theory, LatentSchemaLimits::default())
        .expect("compile source-free schema");
    let bindings = LatentSchemaCompiler
        .bind(
            &schema,
            &world.space,
            &world.actions,
            std::slice::from_ref(&world.episode),
            LatentSchemaLimits::default(),
        )
        .expect("source bindings");
    let binding = bindings
        .iter()
        .find(|binding| binding.theory == report.theory)
        .unwrap_or(&bindings[0]);
    let transfer = LatentSchemaVerifier
        .verify_transfer(
            &schema,
            &world.space,
            &world.actions,
            std::slice::from_ref(&world.episode),
            binding,
        )
        .expect("source transfer");
    let compilation = LatentSchemaVerifier
        .verify_compilation(
            &report.theory,
            &schema,
            transfer.experience(),
            LatentSchemaLimits::default().maximum_latent_permutations,
        )
        .expect("source compilation");
    store
        .record_verified_world(&schema, compilation, transfer)
        .expect("record source world");
    schema
}

fn latent_limits() -> LatentLimits {
    LatentLimits {
        maximum_latent_states: 5,
        maximum_search_nodes: 10_000_000,
    }
}

fn world(structure: Structure, action_offset: u16, coordinate_offset: i64) -> World {
    let dimensions = structure.dimensions;
    let lower = vec![coordinate_offset - 100_000; dimensions];
    let upper = vec![coordinate_offset + 100_000; dimensions];
    let space = StateSpace::new(lower, upper).expect("space");
    let actions = (0..structure.actions)
        .map(|index| ActionId(action_offset + index + 1))
        .collect::<Vec<_>>();
    let rules = (0..structure.latent_states)
        .flat_map(|latent| {
            actions.iter().enumerate().map(move |(role, action)| {
                let role = u16::try_from(role).expect("small");
                let advance = role == 0;
                let next = if advance {
                    (latent + 1) % structure.latent_states
                } else {
                    latent
                };
                let delta = (0..dimensions)
                    .map(|dimension| {
                        if advance {
                            0
                        } else {
                            i64::from(latent + 1)
                                * i64::from(role)
                                * i64::try_from(dimension + 1).expect("small")
                        }
                    })
                    .collect();
                ((*action, latent), Rule { next, delta })
            })
        })
        .map(|((action, latent), rule)| ((latent, action), rule))
        .collect::<BTreeMap<_, _>>();
    let initial = BeliefState {
        visible: WorldState(vec![coordinate_offset; dimensions]),
        latent: 0,
    };
    let mut sequence = (0..structure.latent_states)
        .flat_map(|_| {
            actions
                .iter()
                .skip(1)
                .chain(actions.iter().take(1))
                .copied()
        })
        .collect::<Vec<_>>();
    sequence.push(actions[1]);
    let episode = execute_episode(&space, &rules, initial, &sequence);
    World {
        space,
        actions,
        rules,
        episode,
    }
}

fn execute_episode(
    space: &StateSpace,
    rules: &BTreeMap<(u8, ActionId), Rule>,
    initial: BeliefState,
    actions: &[ActionId],
) -> LatentEpisode {
    let mut state = initial;
    let transitions = actions
        .iter()
        .map(|action| {
            let next = step(space, rules, &state, *action);
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
    rules: &BTreeMap<(u8, ActionId), Rule>,
    state: &BeliefState,
    action: ActionId,
) -> BeliefState {
    let rule = rules.get(&(state.latent, action)).expect("truth rule");
    let visible = state
        .visible
        .0
        .iter()
        .zip(&rule.delta)
        .zip(space.lower().iter().zip(space.upper()))
        .map(|((value, delta), (lower, upper))| (value + delta).clamp(*lower, *upper))
        .collect();
    BeliefState {
        visible: WorldState(visible),
        latent: rule.next,
    }
}

fn assert_predictions(world: &World, theory: &LatentTransitionTheory) {
    for latent in 0..theory.latent_states() {
        for action in &world.actions {
            let state = BeliefState {
                visible: WorldState(vec![0; world.space.dimensions()]),
                latent,
            };
            assert_eq!(
                theory.predict(&state, *action).expect("prediction"),
                step(&world.space, &world.rules, &state, *action)
            );
        }
    }
}
