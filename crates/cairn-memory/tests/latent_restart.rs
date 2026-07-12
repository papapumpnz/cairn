use cairn_cas::{Cas, CasLimits};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BudgetRecord, CanonicalValue, ProducerRecord, SchemaRef,
    SemanticsVersion,
};
use cairn_memory::{
    LatentMechanismStore, LatentWorldEvidenceArchive, LatentWorldReplay, ModuleStatus,
    ReuseArtifactMetadata,
};
use cairn_verifier::LatentSchemaVerifier;
use cairn_world::{
    ActionId, BeliefState, LatentEpisode, LatentLimits, LatentMechanismSchema, LatentSchemaBinding,
    LatentSchemaCompiler, LatentSchemaLimits, LatentTransitionTheory, LatentWorldLearner,
    StateSpace, TransitionObservation, WorldState,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct Rule {
    next: u8,
    delta: i64,
}

struct Prepared {
    space: StateSpace,
    actions: [ActionId; 3],
    episodes: Vec<LatentEpisode>,
    theory: LatentTransitionTheory,
    schema: LatentMechanismSchema,
    binding: LatentSchemaBinding,
    compilation: cairn_verifier::VerifiedLatentSchemaCompilation,
    transfer: cairn_verifier::VerifiedLatentSchemaTransfer,
}

#[test]
fn canonical_latent_evidence_replays_and_repromotes_after_restart() {
    let temporary = TemporaryStore::new();
    let cas = Cas::open(&temporary.path, CasLimits::default()).expect("open CAS");
    let archive = LatentWorldEvidenceArchive;
    let first = prepare(0, [ActionId(4), ActionId(9), ActionId(2)], 15);
    let second = prepare(100, [ActionId(31), ActionId(7), ActionId(19)], 130);
    assert_eq!(first.schema, second.schema);
    let stored_first = archive
        .persist(
            &cas,
            &first.schema,
            &first.compilation,
            &first.transfer,
            metadata(parent(&cas, "first"), 1),
        )
        .expect("persist first world");
    let stored_second = archive
        .persist(
            &cas,
            &second.schema,
            &second.compilation,
            &second.transfer,
            metadata(parent(&cas, "second"), 2),
        )
        .expect("persist second world");
    let expected_schema = first.schema.semantic_hash();
    drop(first);
    drop(second);
    drop(cas);

    let reopened = Cas::open(&temporary.path, CasLimits::default()).expect("reopen CAS");
    reopened.recover().expect("recover CAS");
    let artifacts = [stored_first, stored_second]
        .iter()
        .map(|stored| {
            archive
                .load(&reopened, stored.envelope_blob_id)
                .expect("load")
        })
        .collect::<Vec<_>>();
    assert!(artifacts
        .iter()
        .all(|artifact| artifact.schema().semantic_hash() == expected_schema));

    let rebuilt = [
        prepare(0, [ActionId(4), ActionId(9), ActionId(2)], 15),
        prepare(100, [ActionId(31), ActionId(7), ActionId(19)], 130),
    ];
    let mut restarted_store = LatentMechanismStore::new();
    for (artifact, evidence) in artifacts.iter().zip(&rebuilt) {
        let (compilation, transfer) = archive
            .replay(
                artifact,
                LatentWorldReplay {
                    theory: &evidence.theory,
                    space: &evidence.space,
                    actions: &evidence.actions,
                    episodes: &evidence.episodes,
                    binding: &evidence.binding,
                    maximum_latent_permutations: 100,
                },
            )
            .expect("independent restart replay");
        restarted_store
            .record_verified_world(artifact.schema(), compilation, transfer)
            .expect("reconstruct promotion");
    }
    let module = restarted_store.module(expected_schema).expect("module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.independently_verified_worlds(), 2);
    eprintln!("LATENT_RESTART schema={expected_schema} artifacts=2 replayed_worlds=2 repromoted=1");
}

fn prepare(offset: i64, actions: [ActionId; 3], initial: i64) -> Prepared {
    let space = StateSpace::new(vec![offset], vec![offset + 80]).expect("space");
    let rules = truth(actions);
    let episodes = vec![episode(
        &space,
        &rules,
        BeliefState {
            visible: WorldState(vec![initial]),
            latent: 0,
        },
        &[
            actions[0], actions[1], actions[2], actions[0], actions[1], actions[2], actions[0],
        ],
    )];
    let theory = LatentWorldLearner
        .induce(&space, &actions, &episodes, LatentLimits::default())
        .expect("learn")
        .theory;
    let schema = LatentSchemaCompiler
        .compile(&theory, LatentSchemaLimits::default())
        .expect("schema");
    let binding = LatentSchemaCompiler
        .bind(
            &schema,
            &space,
            &actions,
            &episodes,
            LatentSchemaLimits::default(),
        )
        .expect("binding")
        .remove(0);
    let transfer = LatentSchemaVerifier
        .verify_transfer(&schema, &space, &actions, &episodes, &binding)
        .expect("transfer");
    let compilation = LatentSchemaVerifier
        .verify_compilation(&theory, &schema, transfer.experience(), 100)
        .expect("compilation");
    Prepared {
        space,
        actions,
        episodes,
        theory,
        schema,
        binding,
        compilation,
        transfer,
    }
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

fn episode(
    space: &StateSpace,
    rules: &BTreeMap<(u8, ActionId), Rule>,
    initial: BeliefState,
    actions: &[ActionId],
) -> LatentEpisode {
    let mut state = initial;
    let transitions = actions
        .iter()
        .map(|action| {
            let rule = rules.get(&(state.latent, *action)).expect("rule");
            let next = BeliefState {
                visible: WorldState(vec![
                    (state.visible.0[0] + rule.delta).clamp(space.lower()[0], space.upper()[0])
                ]),
                latent: rule.next,
            };
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

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.latent-world-parent", 1).expect("schema"),
        ArtifactKind::new("test.latent-world-parent").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        Vec::new(),
        None,
        producer(),
        budget(),
        0,
        CanonicalValue::Text(name.to_owned()),
    )
    .expect("parent");
    cas.put_envelope(&envelope).expect("put parent").artifact_id
}

fn metadata(parent: cairn_core::ArtifactId, logical: u64) -> ReuseArtifactMetadata {
    ReuseArtifactMetadata {
        parent_ids: vec![parent],
        producer: producer(),
        budget: budget(),
        created_at_logical: logical,
    }
}

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "latent-restart-test".to_owned(),
        version: "0.1.0".to_owned(),
        commit: "test".to_owned(),
        host_run_id: "restart".to_owned(),
        agent_id: Some("memory".to_owned()),
    }
}

const fn budget() -> BudgetRecord {
    BudgetRecord {
        work_units: 64,
        cpu_nanos: 0,
        peak_rss_bytes: 0,
    }
}

struct TemporaryStore {
    path: PathBuf,
}

impl TemporaryStore {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        Self {
            path: std::env::temp_dir().join(format!(
                "cairn-latent-restart-{}-{sequence}",
                std::process::id()
            )),
        }
    }
}

impl Drop for TemporaryStore {
    fn drop(&mut self) {
        let _ignored = std::fs::remove_dir_all(&self.path);
    }
}
