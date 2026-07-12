use cairn_memory::LatentMechanismStore;
use cairn_runtime::{CognitionRuntime, LatentMemoryRuntimeError};
use cairn_sequence::{SequenceAdapter, SequenceTransition};
use cairn_vector::{VectorAdapter, VectorPointTransition};
use cairn_verifier::LatentSchemaVerifier;
use cairn_world::{
    ActionId, LatentEpisode, LatentLimits, LatentMechanismSchema, LatentSchemaCompiler,
    LatentSchemaLimits, LatentWorldLearner, StateSpace, TransitionObservation, WorldState,
};

#[test]
fn interactive_world_mechanism_improves_sequence_and_vector_prediction() {
    let mut store = LatentMechanismStore::new();
    let first = source_world([ActionId(4), ActionId(9)], [10, 20]);
    let second = source_world([ActionId(31), ActionId(7)], [100, -50]);
    let schema = learn_and_record(&mut store, &first.0, first.1, &first.2);
    assert_eq!(
        schema,
        learn_and_record(&mut store, &second.0, second.1, &second.2)
    );
    let runtime = CognitionRuntime::with_latent_mechanism_memory(store);

    let sequence_actions = [ActionId(41), ActionId(13)];
    let sequence_episode = SequenceAdapter.lower_transition_episode(&[
        SequenceTransition {
            before: vec![5, 8],
            action: 41,
            after: vec![6, 10],
        },
        SequenceTransition {
            before: vec![6, 10],
            action: 13,
            after: vec![5, 11],
        },
    ]);
    let sequence_space = StateSpace::new(vec![-100, -100], vec![100, 100]).expect("space");
    let sequence = runtime
        .propose_latent_world_from_memory(
            &sequence_space,
            &sequence_actions,
            std::slice::from_ref(&sequence_episode),
            LatentSchemaLimits::default(),
        )
        .expect("sequence transfer");
    assert_eq!(sequence.proposals.len(), 1);
    LatentSchemaVerifier
        .verify_transfer(
            &schema,
            &sequence_space,
            &sequence_actions,
            std::slice::from_ref(&sequence_episode),
            &sequence.proposals[0].binding,
        )
        .expect("sequence replay");

    let vector_actions = [ActionId(71), ActionId(89)];
    let vector_episode = VectorAdapter.lower_transition_episode(&[
        VectorPointTransition {
            before: (30, 40),
            action: 71,
            after: (31, 42),
        },
        VectorPointTransition {
            before: (31, 42),
            action: 89,
            after: (30, 43),
        },
    ]);
    let vector_space = StateSpace::new(vec![-100, -100], vec![100, 100]).expect("space");
    let vector = runtime
        .propose_latent_world_from_memory(
            &vector_space,
            &vector_actions,
            std::slice::from_ref(&vector_episode),
            LatentSchemaLimits::default(),
        )
        .expect("vector transfer");
    assert_eq!(vector.proposals.len(), 1);
    LatentSchemaVerifier
        .verify_transfer(
            &schema,
            &vector_space,
            &vector_actions,
            std::slice::from_ref(&vector_episode),
            &vector.proposals[0].binding,
        )
        .expect("vector replay");

    let blank = CognitionRuntime::new().propose_latent_world_from_memory(
        &vector_space,
        &vector_actions,
        &[vector_episode],
        LatentSchemaLimits::default(),
    );
    assert!(matches!(
        blank,
        Err(cairn_runtime::CognitionError::LatentMemory(
            LatentMemoryRuntimeError::NoVerifiedProposal
        ))
    ));
    eprintln!(
        "G5_PERSISTENT_WORLD source=interactive targets=sequence,vector memory_predictions=2 blank_predictions=0 exact_replays=2"
    );
}

fn source_world(
    actions: [ActionId; 2],
    initial: [i64; 2],
) -> (StateSpace, [ActionId; 2], LatentEpisode) {
    let space = StateSpace::new(vec![-1_000, -1_000], vec![1_000, 1_000]).expect("space");
    let mut state = initial;
    let sequence = [actions[0], actions[1], actions[0]];
    let transitions = sequence
        .into_iter()
        .map(|action| {
            let delta = if action == actions[0] {
                [1, 2]
            } else {
                [-1, 1]
            };
            let after = [state[0] + delta[0], state[1] + delta[1]];
            let observed = TransitionObservation {
                before: WorldState(state.to_vec()),
                action,
                after: WorldState(after.to_vec()),
            };
            state = after;
            observed
        })
        .collect();
    (space, actions, LatentEpisode { transitions })
}

fn learn_and_record(
    store: &mut LatentMechanismStore,
    space: &StateSpace,
    actions: [ActionId; 2],
    episode: &LatentEpisode,
) -> LatentMechanismSchema {
    let episodes = std::slice::from_ref(episode);
    let theory = LatentWorldLearner
        .induce(space, &actions, episodes, LatentLimits::default())
        .expect("learn source")
        .theory;
    let schema = LatentSchemaCompiler
        .compile(&theory, LatentSchemaLimits::default())
        .expect("schema");
    let binding = LatentSchemaCompiler
        .bind(
            &schema,
            space,
            &actions,
            episodes,
            LatentSchemaLimits::default(),
        )
        .expect("binding")
        .remove(0);
    let transfer = LatentSchemaVerifier
        .verify_transfer(&schema, space, &actions, episodes, &binding)
        .expect("transfer");
    let compilation = LatentSchemaVerifier
        .verify_compilation(&theory, &schema, transfer.experience(), 100)
        .expect("compilation");
    store
        .record_verified_world(&schema, compilation, transfer)
        .expect("record");
    schema
}
