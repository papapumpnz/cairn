use cairn_memory::{LatentMechanismStore, LatentPromotionProgress, LatentStoreError, ModuleStatus};
use cairn_runtime::{CognitionError, CognitionRuntime, LatentMemoryRuntimeError};
use cairn_verifier::LatentSchemaVerifier;
use cairn_world::{
    ActionId, ActiveLatentLimits, ActiveLatentWorldLearner, BeliefState, LatentEpisode,
    LatentError, LatentExperiment, LatentSchemaCompiler, LatentSchemaLimits,
    LatentTransitionTheory, StateSpace, TransitionObservation, WorldState,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct TruthRule {
    next_latent: u8,
    delta: i64,
}

struct HiddenEnvironment {
    fixture: Fixture,
}

impl LatentExperiment for HiddenEnvironment {
    fn observe_continuation(
        &mut self,
        episode_index: usize,
        history: &[TransitionObservation],
        action: ActionId,
    ) -> Result<TransitionObservation, LatentError> {
        let mut state = self
            .fixture
            .initial
            .get(episode_index)
            .cloned()
            .ok_or(LatentError::InvalidInterventionObservation)?;
        for observed in history {
            if observed.before != state.visible {
                return Err(LatentError::InvalidInterventionObservation);
            }
            let next = step(&self.fixture, &state, observed.action);
            if observed.after != next.visible {
                return Err(LatentError::InvalidInterventionObservation);
            }
            state = next;
        }
        let next = step(&self.fixture, &state, action);
        Ok(TransitionObservation {
            before: state.visible,
            action,
            after: next.visible,
        })
    }
}

#[derive(Clone)]
struct Fixture {
    space: StateSpace,
    actions: [ActionId; 3],
    truth: BTreeMap<(u8, ActionId), TruthRule>,
    initial: Vec<BeliefState>,
    episodes: Vec<LatentEpisode>,
}

#[test]
fn two_verified_worlds_promote_and_remove_third_world_intervention() {
    let mut store = LatentMechanismStore::new();
    assert_eq!(store.persistent_schemas().count(), 0);

    let first = fixture(0, 30, [4, 9, 2], [15, 9]);
    let first_schema = learn_verify_and_record(&mut store, &first);
    assert_eq!(
        store
            .module(first_schema.semantic_hash())
            .map(cairn_memory::LatentMechanismModule::status),
        Some(ModuleStatus::Provisional)
    );
    assert_eq!(store.persistent_schemas().count(), 0);

    let second = fixture(100, 170, [31, 7, 19], [135, 122]);
    let second_schema = learn_verify_and_record(&mut store, &second);
    assert_eq!(first_schema, second_schema);
    let module = store
        .module(first_schema.semantic_hash())
        .expect("promoted module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.independently_verified_worlds(), 2);

    let third = fixture(-80, 40, [71, 43, 89], [-11, 23]);
    let mut fresh_environment = HiddenEnvironment {
        fixture: third.clone(),
    };
    let fresh = ActiveLatentWorldLearner
        .induce(
            &third.space,
            &third.actions,
            &third.episodes,
            ActiveLatentLimits::default(),
            &mut fresh_environment,
        )
        .expect("fresh active induction");
    assert_eq!(fresh.interventions.len(), 1);

    let persistent = store
        .persistent_schemas()
        .next()
        .expect("persistent proposal")
        .clone();
    let runtime = CognitionRuntime::with_latent_mechanism_memory(store);
    let memory_outcome = runtime
        .propose_latent_world_from_memory(
            &third.space,
            &third.actions,
            &third.episodes,
            LatentSchemaLimits::default(),
        )
        .expect("runtime bounded current-world rebinding");
    assert_eq!(memory_outcome.persistent_schemas_attempted, 1);
    let bindings = memory_outcome.bindings;
    assert_eq!(bindings.len(), 1);
    let mut corrupt_binding = bindings[0].clone();
    let duplicate_role = corrupt_binding.action_roles[&third.actions[1]];
    corrupt_binding
        .action_roles
        .insert(third.actions[0], duplicate_role);
    assert_eq!(
        LatentSchemaVerifier.verify_transfer(
            &persistent,
            &third.space,
            &third.actions,
            &third.episodes,
            &corrupt_binding,
        ),
        Err(cairn_verifier::LatentSchemaVerifyError::InvalidBinding)
    );
    let transfer = LatentSchemaVerifier
        .verify_transfer(
            &persistent,
            &third.space,
            &third.actions,
            &third.episodes,
            &bindings[0],
        )
        .expect("independent current replay");
    assert_eq!(transfer.replayed_transitions(), 9);
    let predictions = assert_predictions(&third, &bindings[0].theory);
    assert_eq!(predictions, 510);
    assert_eq!(
        assert_predictions(&third, &fresh.induction.theory),
        predictions
    );

    eprintln!(
        "G2_LATENT_MEMORY verified_worlds=2 fresh_interventions=1 memory_interventions=0 predictions={predictions} schema={:?}",
        persistent.semantic_hash()
    );
}

fn learn_verify_and_record(
    store: &mut LatentMechanismStore,
    fixture: &Fixture,
) -> cairn_world::LatentMechanismSchema {
    let mut environment = HiddenEnvironment {
        fixture: fixture.clone(),
    };
    let report = ActiveLatentWorldLearner
        .induce(
            &fixture.space,
            &fixture.actions,
            &fixture.episodes,
            ActiveLatentLimits::default(),
            &mut environment,
        )
        .expect("source active induction");
    assert_eq!(report.interventions.len(), 1);
    let schema = LatentSchemaCompiler
        .compile(&report.induction.theory, LatentSchemaLimits::default())
        .expect("source-independent schema");
    let binding = LatentSchemaCompiler
        .bind(
            &schema,
            &fixture.space,
            &fixture.actions,
            &report.episodes,
            LatentSchemaLimits::default(),
        )
        .expect("source replay binding")
        .into_iter()
        .next()
        .expect("source binding");
    let transfer = LatentSchemaVerifier
        .verify_transfer(
            &schema,
            &fixture.space,
            &fixture.actions,
            &report.episodes,
            &binding,
        )
        .expect("source transfer receipt");
    let compilation = LatentSchemaVerifier
        .verify_compilation(
            &report.induction.theory,
            &schema,
            transfer.experience(),
            LatentSchemaLimits::default().maximum_latent_permutations,
        )
        .expect("independent schema compilation");
    let progress = store
        .record_verified_world(&schema, compilation, transfer)
        .expect("proof-gated evidence");
    match store
        .module(schema.semantic_hash())
        .expect("registered module")
        .independently_verified_worlds()
    {
        1 => assert_eq!(
            progress,
            LatentPromotionProgress::Provisional {
                independently_verified_worlds: 1
            }
        ),
        2 => assert_eq!(
            progress,
            LatentPromotionProgress::Persistent {
                independently_verified_worlds: 2
            }
        ),
        count => panic!("unexpected source count {count}"),
    }
    schema
}

fn fixture(lower: i64, upper: i64, raw_actions: [u16; 3], visible: [i64; 2]) -> Fixture {
    let actions = raw_actions.map(ActionId);
    let [one, two, flip] = actions;
    let space = StateSpace::new(vec![lower], vec![upper]).expect("space");
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
    let initial = visible
        .into_iter()
        .map(|value| BeliefState {
            visible: WorldState(vec![value]),
            latent: 0,
        })
        .collect::<Vec<_>>();
    let mut result = Fixture {
        space,
        actions,
        truth,
        initial,
        episodes: Vec::new(),
    };
    result.episodes = vec![
        episode(&result, result.initial[0].clone(), &[one, two, flip, one]),
        episode(
            &result,
            result.initial[1].clone(),
            &[flip, two, flip, one, two],
        ),
    ];
    result
}

fn episode(fixture: &Fixture, initial: BeliefState, actions: &[ActionId]) -> LatentEpisode {
    let mut state = initial;
    let transitions = actions
        .iter()
        .map(|action| {
            let next = step(fixture, &state, *action);
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

fn step(fixture: &Fixture, state: &BeliefState, action: ActionId) -> BeliefState {
    let rule = fixture
        .truth
        .get(&(state.latent, action))
        .expect("truth rule");
    BeliefState {
        visible: WorldState(vec![(state.visible.0[0] + rule.delta)
            .clamp(fixture.space.lower()[0], fixture.space.upper()[0])]),
        latent: rule.next_latent,
    }
}

fn assert_predictions(fixture: &Fixture, theory: &LatentTransitionTheory) -> u64 {
    let mut predictions = 0_u64;
    for latent in 0_u8..2 {
        for visible in -70_i64..=14 {
            for action in fixture.actions {
                let state = BeliefState {
                    visible: WorldState(vec![visible]),
                    latent,
                };
                assert_eq!(
                    theory.predict(&state, action).expect("prediction"),
                    step(fixture, &state, action)
                );
                predictions += 1;
            }
        }
    }
    predictions
}

#[test]
fn duplicate_world_evidence_cannot_fake_promotion() {
    let fixture = fixture(0, 30, [4, 9, 2], [15, 9]);
    let mut store = LatentMechanismStore::new();
    let schema = learn_verify_and_record(&mut store, &fixture);

    let mut environment = HiddenEnvironment {
        fixture: fixture.clone(),
    };
    let report = ActiveLatentWorldLearner
        .induce(
            &fixture.space,
            &fixture.actions,
            &fixture.episodes,
            ActiveLatentLimits::default(),
            &mut environment,
        )
        .expect("repeat source");
    let binding = LatentSchemaCompiler
        .bind(
            &schema,
            &fixture.space,
            &fixture.actions,
            &report.episodes,
            LatentSchemaLimits::default(),
        )
        .expect("repeat binding")
        .remove(0);
    let transfer = LatentSchemaVerifier
        .verify_transfer(
            &schema,
            &fixture.space,
            &fixture.actions,
            &report.episodes,
            &binding,
        )
        .expect("repeat transfer");
    let compilation = LatentSchemaVerifier
        .verify_compilation(
            &report.induction.theory,
            &schema,
            transfer.experience(),
            100,
        )
        .expect("repeat compilation");
    assert_eq!(
        store.record_verified_world(&schema, compilation, transfer),
        Err(LatentStoreError::DuplicateExperience)
    );
    assert_eq!(store.persistent_schemas().count(), 0);
}

#[test]
fn blank_runtime_cannot_claim_latent_memory_learning() {
    let current = fixture(-80, 40, [71, 43, 89], [-11, 23]);
    assert_eq!(
        CognitionRuntime::new().propose_latent_world_from_memory(
            &current.space,
            &current.actions,
            &current.episodes,
            LatentSchemaLimits::default(),
        ),
        Err(CognitionError::LatentMemory(
            LatentMemoryRuntimeError::NoVerifiedProposal
        ))
    );
}
