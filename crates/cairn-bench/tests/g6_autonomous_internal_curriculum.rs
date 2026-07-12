use cairn_cas::{Cas, CasLimits};
use cairn_consolidator::{
    CompiledInternalMechanism, Consolidator, ConstructedInternalComposition,
    InternalConstructionLimits, InternalGraphCompressionProposal, InternalMiningLimits,
    LoweredObservationFrontier, ObservationFrontierLimits, ProofDatum, ProofFrontier, ProofGraph,
    ProofGraphArchive, ProofGraphArtifactMetadata, ProofNode, ProofValueType,
};
use cairn_core::{
    ArtifactEnvelope, ArtifactKind, BlobId, BudgetRecord, CanonicalValue, EvidenceElement,
    EvidenceElementId, EvidenceRelation, EvidenceValueType, Hash32, ModalityId,
    ObservationCertainty, ProducerRecord, RelationalObservation, RelationalObservationLimits,
    SchemaRef, SemanticsVersion, SourceReference,
};
use cairn_curriculum::{
    AutonomousCurriculum, CurriculumCheckpointArchive, CurriculumCheckpointMetadata,
    DevelopmentalNeed, DevelopmentalObjective, DevelopmentalOutcome, ExperienceId,
    FailureCoreExtractor, FailureDrivenObjectiveGenerator, LearnedMechanismStatus, LearningAxis,
    MechanismTarget, SustainedLearningGate, SustainedLearningPoint,
};
use cairn_invent::{BindingExample, InventionError, InventionLimits, InventionProblem};
use cairn_memory::{
    InternalMechanismStore, InternalPromotionProgress, InternalReuseArtifactArchive,
    InternalReuseReplay, ModuleStatus, ReuseArtifactMetadata,
};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_runtime::{CognitionError, CognitionRuntime, InternalCompositionRuntimeError};
use cairn_verifier::{
    InternalCompositionEvidence, InternalCompressionVerifyError, InternalMechanismVerifier,
    RelationalConstructionEvidence, RelationalPolicyInductionEvidence,
    VerifiedInternalConstruction, VerifiedInternalMechanism, VerifiedRelationalConstructionPolicy,
};
use cairn_world::{
    ActionId, GoalObservation, PlanLimits, StateSpace, TransitionObservation, WorldState,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct FamilySpec {
    objective: &'static str,
    target: &'static str,
    operations: [&'static str; 4],
    base_episode: u8,
    first_host: (usize, usize),
    second_host: (usize, usize),
}

#[derive(Clone)]
struct PersistedFamily {
    graph_blobs: Vec<BlobId>,
    receipt_blobs: Vec<BlobId>,
    mechanism: Hash32,
    proposal: Hash32,
}

#[derive(Clone)]
struct LearnedFamily {
    sources: [ProofGraph; 4],
    proposal: InternalGraphCompressionProposal,
    mechanism: CompiledInternalMechanism,
    compilation: VerifiedInternalMechanism,
}

#[derive(Debug)]
struct MaterializedConstructionExercise {
    observation: RelationalObservation,
    lowering: LoweredObservationFrontier,
    expected: ProofGraph,
}

#[derive(Debug)]
struct MaterializedCurriculum {
    exercises: [MaterializedConstructionExercise; 2],
    learned_world_rules: usize,
    world_prediction_cases: usize,
    world_plan_length: usize,
}

struct LearnedWorldExercise {
    values: [(String, String); 2],
    rules: usize,
    prediction_cases: usize,
    plan_length: usize,
}

fn verify_materialized_candidate(
    exercise: &MaterializedConstructionExercise,
    candidate: &ConstructedInternalComposition,
    learned: &[LearnedFamily],
    required: [Hash32; 2],
) -> Result<VerifiedInternalConstruction, InternalCompressionVerifyError> {
    InternalMechanismVerifier.verify_observation_frontier(
        &exercise.observation,
        &exercise.lowering,
        observation_frontier_limits(),
    )?;
    let evidence = materialized_evidence(candidate, learned, required)?;
    InternalMechanismVerifier.verify_constructed_composition(
        &exercise.lowering.frontier,
        &exercise.expected,
        candidate,
        &evidence,
        64,
    )
}

fn verify_relational_materialized_candidate(
    exercise: &MaterializedConstructionExercise,
    candidate: &ConstructedInternalComposition,
    learned: &[LearnedFamily],
    required: [Hash32; 2],
    policy: &cairn_consolidator::RelationalConstructionPolicy,
    policy_verification: &VerifiedRelationalConstructionPolicy,
) -> Result<VerifiedInternalConstruction, InternalCompressionVerifyError> {
    let evidence = materialized_evidence(candidate, learned, required)?;
    InternalMechanismVerifier.verify_relational_constructed_composition(
        RelationalConstructionEvidence {
            observation: &exercise.observation,
            lowering: &exercise.lowering,
            policy,
            expected: &exercise.expected,
            candidate,
            mechanisms: &evidence,
            policy_verification,
            limits: observation_frontier_limits(),
        },
    )
}

fn materialized_evidence<'a>(
    candidate: &'a ConstructedInternalComposition,
    learned: &'a [LearnedFamily],
    required: [Hash32; 2],
) -> Result<[InternalCompositionEvidence<'a>; 2], InternalCompressionVerifyError> {
    let first = learned
        .iter()
        .find(|family| family.mechanism.semantic_hash() == required[0])
        .ok_or(InternalCompressionVerifyError::CompiledLineageMismatch)?;
    let second = learned
        .iter()
        .find(|family| family.mechanism.semantic_hash() == required[1])
        .ok_or(InternalCompressionVerifyError::CompiledLineageMismatch)?;
    let first_binding = candidate
        .invocations
        .iter()
        .find(|invocation| invocation.mechanism == required[0])
        .map(|invocation| &invocation.binding)
        .ok_or(InternalCompressionVerifyError::CompiledLineageMismatch)?;
    let second_binding = candidate
        .invocations
        .iter()
        .find(|invocation| invocation.mechanism == required[1])
        .map(|invocation| &invocation.binding)
        .ok_or(InternalCompressionVerifyError::CompiledLineageMismatch)?;
    Ok([
        InternalCompositionEvidence {
            proposal: &first.proposal,
            mechanism: &first.mechanism,
            compilation: &first.compilation,
            sources: &first.sources,
            binding: first_binding,
        },
        InternalCompositionEvidence {
            proposal: &second.proposal,
            mechanism: &second.mechanism,
            compilation: &second.compilation,
            sources: &second.sources,
            binding: second_binding,
        },
    ])
}

#[test]
#[allow(clippy::too_many_lines)] // Frozen multi-phase autonomous/restart audit.
fn autonomous_curriculum_grows_and_retains_a_restartable_mechanism_library() {
    let families = families();
    let fixture = TemporaryStore::new();
    let mut cas = Cas::open(&fixture.path, CasLimits::default()).expect("open CAS");
    let mut curriculum =
        AutonomousCurriculum::new(objectives(), 12).expect("autonomous curriculum");
    let mut store = InternalMechanismStore::new();
    let mut persisted = Vec::<PersistedFamily>::new();
    let mut retention_hosts = Vec::<ProofGraph>::new();
    let mut learned = Vec::<LearnedFamily>::new();
    let mut points = Vec::<SustainedLearningPoint>::new();
    let mut completed_restarts = 0_u32;
    let mut curriculum_restores = 0_u32;
    let mut curriculum_checkpoints = Vec::new();
    let mut selected = Vec::new();

    for stage in 0..families.len() {
        let decision = curriculum.select_next().expect("autonomous base decision");
        selected.push(decision.objective.0.clone());
        let spec = families
            .iter()
            .find(|spec| spec.objective == decision.objective.0)
            .expect("generator owns selected objective");
        let sources = source_cohort(spec);
        let hosts = held_out_hosts(spec);
        let proposal = Consolidator
            .mine_internal_proof_subgraphs(&sources, limits())
            .expect("mine selected family");
        assert_eq!(
            proposal.pattern()[0].operation,
            spec.operations[0],
            "learner must discover the generator's hidden internal family"
        );
        let mechanism = Consolidator
            .compile_internal_proof_mechanism(&proposal)
            .expect("compile selected family");
        let compilation = InternalMechanismVerifier
            .verify_compilation(&proposal, &mechanism, &sources)
            .expect("verify selected family");
        let id = store
            .insert_provisional(mechanism.clone(), compilation.clone())
            .expect("new semantic mechanism");
        let receipts = hosts
            .iter()
            .map(|host| {
                let binding = Consolidator
                    .match_compiled_internal_mechanism(&mechanism, host, 64)
                    .expect("bind held-out host")
                    .remove(0);
                InternalMechanismVerifier
                    .verify_reuse(
                        &proposal,
                        &mechanism,
                        &compilation,
                        &sources,
                        host,
                        &binding,
                        64,
                    )
                    .expect("verify held-out reuse")
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            store
                .record_runtime_reuse(id, receipts[0].clone())
                .expect("first reuse"),
            InternalPromotionProgress::Pending { verified_reuses: 1 }
        ));
        assert!(matches!(
            store
                .record_runtime_reuse(id, receipts[1].clone())
                .expect("second reuse"),
            InternalPromotionProgress::Promoted(_)
        ));
        assert_eq!(
            store.get(id).expect("learned module").status(),
            ModuleStatus::Persistent
        );

        let retained = retention_hosts
            .iter()
            .enumerate()
            .filter(|(index, host)| {
                store
                    .get(persisted[*index].mechanism)
                    .is_some_and(|module| {
                        Consolidator
                            .match_compiled_internal_mechanism(module.mechanism(), host, 64)
                            .is_ok()
                    })
            })
            .count();
        assert_eq!(retained, retention_hosts.len());
        let negative_host = negative_host(spec.base_episode.saturating_add(9));
        let mut transfer_attempts = 1_u32;
        let mut negative_transfers = u32::from(
            Consolidator
                .match_compiled_internal_mechanism(&mechanism, &negative_host, 64)
                .is_ok(),
        );
        for prior in &retention_hosts {
            transfer_attempts = transfer_attempts.saturating_add(1);
            negative_transfers = negative_transfers.saturating_add(u32::from(
                Consolidator
                    .match_compiled_internal_mechanism(&mechanism, prior, 64)
                    .is_ok(),
            ));
        }
        assert_eq!(negative_transfers, 0);

        let stored = persist_family(&cas, &sources, &hosts, &receipts);
        persisted.push(PersistedFamily {
            graph_blobs: stored.0,
            receipt_blobs: stored.1,
            mechanism: mechanism.semantic_hash(),
            proposal: proposal.semantic_hash(),
        });
        learned.push(LearnedFamily {
            sources: sources.clone(),
            proposal: proposal.clone(),
            mechanism: mechanism.clone(),
            compilation: compilation.clone(),
        });
        retention_hosts.push(hosts[0].clone());

        let metrics = receipts[0].metrics();
        curriculum
            .record(DevelopmentalOutcome {
                decision_sequence: decision.sequence,
                objective: decision.objective,
                mechanism_evidence: mechanism.semantic_hash().to_hex(),
                independently_verified: true,
                held_out_transfer_cases: 2,
                candidate_work_without: metrics.candidate_windows_without,
                candidate_work_with: metrics.candidate_windows_with,
                examples_without: metrics.examples_without,
                examples_with: metrics.examples_with,
                retained_prior_tasks: u32::try_from(retained).expect("small"),
                prior_tasks: u32::try_from(stage).expect("small"),
                negative_transfers,
            })
            .expect("curriculum accepts measured learning");
        points.push(SustainedLearningPoint {
            learned_mechanisms: u32::try_from(store.len()).expect("small"),
            solved_capabilities: u32::try_from(store.len()).expect("small"),
            evaluated_capabilities: u32::try_from(families.len()).expect("small"),
            candidate_work_without: metrics.candidate_windows_without,
            candidate_work_with: metrics.candidate_windows_with,
            examples_without: metrics.examples_without,
            examples_with: metrics.examples_with,
            proof_units_without: metrics.proof_units_without,
            proof_units_with: metrics.proof_units_with,
            retained_prior_tasks: u32::try_from(retained).expect("small"),
            prior_tasks: u32::try_from(stage).expect("small"),
            negative_transfers,
            transfer_attempts,
            completed_restarts,
        });

        if stage == 1 || stage + 1 == families.len() {
            if stage + 1 == families.len() {
                let target = MechanismTarget("target-d".to_owned());
                assert_eq!(
                    curriculum
                        .record_counterexample(
                            &target,
                            "held-out-overlap-failure".to_owned(),
                            true,
                        )
                        .expect("quarantine failed mechanism"),
                    LearnedMechanismStatus::Quarantined
                );
                assert_eq!(
                    curriculum
                        .record_counterexample(
                            &target,
                            "held-out-nesting-failure".to_owned(),
                            true,
                        )
                        .expect("deprecate failed mechanism"),
                    LearnedMechanismStatus::Deprecated
                );
            }
            let curriculum_parent = parent(&cas, &format!("curriculum-stage-{stage}"));
            let checkpoint = CurriculumCheckpointArchive
                .persist(
                    &cas,
                    &curriculum.checkpoint(),
                    CurriculumCheckpointMetadata {
                        parent_ids: vec![curriculum_parent],
                        producer: producer(),
                        budget: budget(),
                        created_at_logical: 1_000 + u64::try_from(stage).expect("small"),
                    },
                )
                .expect("persist curriculum checkpoint");
            curriculum_checkpoints.push(checkpoint.artifact_id);
            drop(cas);
            cas = Cas::open(&fixture.path, CasLimits::default()).expect("restart CAS");
            cas.recover().expect("recover CAS");
            curriculum = CurriculumCheckpointArchive
                .load(&cas, checkpoint.envelope_blob_id)
                .expect("restore curriculum by replay");
            curriculum_restores = curriculum_restores.saturating_add(1);
            store = reconstruct_store(&cas, &persisted);
            completed_restarts = completed_restarts.saturating_add(1);
            points.last_mut().expect("point").completed_restarts = completed_restarts;
            assert_eq!(store.len(), persisted.len());
            assert!(persisted.iter().all(|family| {
                store
                    .get(family.mechanism)
                    .is_some_and(|module| module.status() == ModuleStatus::Persistent)
            }));
        }
    }

    let active_support = BTreeSet::from([
        MechanismTarget(families[0].target.to_owned()),
        MechanismTarget(families[1].target.to_owned()),
    ]);
    let generated = FailureDrivenObjectiveGenerator
        .generate(&unresolved_composition_core(), &active_support)
        .expect("generate objective from failure core");
    assert_eq!(generated.need, DevelopmentalNeed::MechanismComposition);
    curriculum
        .add_objective(generated.objective.clone())
        .expect("admit generated objective");
    let generated_checkpoint = CurriculumCheckpointArchive
        .persist(
            &cas,
            &curriculum.checkpoint(),
            CurriculumCheckpointMetadata {
                parent_ids: vec![parent(&cas, "generated-objective")],
                producer: producer(),
                budget: budget(),
                created_at_logical: 2_000,
            },
        )
        .expect("persist generated objective event");
    curriculum_checkpoints.push(generated_checkpoint.artifact_id);
    curriculum = CurriculumCheckpointArchive
        .load(&cas, generated_checkpoint.envelope_blob_id)
        .expect("restore generated objective boundary");
    curriculum_restores = curriculum_restores.saturating_add(1);
    let composition = curriculum
        .select_next()
        .expect("generated composition objective");
    assert_eq!(composition.axis, LearningAxis::Composition);
    assert_eq!(composition.objective, generated.objective.id);
    let mut runtime = CognitionRuntime::new();
    *runtime.internal_mechanism_memory_mut() = store.clone();
    let required = [
        learned[0].mechanism.semantic_hash(),
        learned[1].mechanism.semantic_hash(),
    ];
    let materialized = materialize_composition_exercises(
        &generated.exercise_request(),
        &families[0],
        &families[1],
        required,
        &runtime,
    );
    let exercises = &materialized.exercises;
    assert_eq!(materialized.learned_world_rules, 2);
    assert_eq!(materialized.world_prediction_cases, 2);
    assert_eq!(materialized.world_plan_length, 1);
    assert!(matches!(
        CognitionRuntime::new().construct_internal_compositions(
            &required,
            &exercises[0].lowering.frontier,
            InternalConstructionLimits::default(),
        ),
        Err(CognitionError::InternalComposition(
            InternalCompositionRuntimeError::UnavailableMechanism(_)
        ))
    ));
    let training_candidates = runtime
        .construct_internal_compositions(
            &required,
            &exercises[0].lowering.frontier,
            InternalConstructionLimits {
                maximum_bindings_per_mechanism: 8,
                maximum_candidates: 32,
            },
        )
        .expect("training construction candidates");
    let (training_candidate, training_verified) = training_candidates
        .iter()
        .find_map(|candidate| {
            verify_materialized_candidate(&exercises[0], candidate, &learned, required)
                .ok()
                .map(|verified| (candidate, verified))
        })
        .expect("verified training construction");
    let policy = Consolidator
        .induce_relational_construction_policy(
            &[learned[0].mechanism.clone(), learned[1].mechanism.clone()],
            &exercises[0].observation,
            &exercises[0].lowering,
            training_candidate,
            observation_frontier_limits(),
        )
        .expect("relational correspondence policy from verified construction");
    let training_evidence = materialized_evidence(training_candidate, &learned, required)
        .expect("training mechanism evidence");
    let policy_verification = InternalMechanismVerifier
        .verify_relational_policy_induction(RelationalPolicyInductionEvidence {
            observation: &exercises[0].observation,
            lowering: &exercises[0].lowering,
            policy: &policy,
            candidate: training_candidate,
            verified_construction: &training_verified,
            mechanisms: &training_evidence,
            limits: observation_frontier_limits(),
        })
        .expect("independently bind policy to verified training construction");
    let ordinal_policy = Consolidator
        .induce_internal_construction_policy(
            &[learned[0].mechanism.clone(), learned[1].mechanism.clone()],
            &exercises[0].lowering.frontier,
            training_candidate,
        )
        .expect("ordinal ablation policy");
    if let Ok(ordinal_candidate) = runtime.apply_internal_construction_policy(
        &required,
        &exercises[1].lowering.frontier,
        &ordinal_policy,
    ) {
        assert_ne!(
            ordinal_candidate.graph, exercises[1].expected,
            "changed ordering/distractor must defeat ordinal correspondence"
        );
    }
    let held_out_candidate = runtime
        .apply_relational_construction_policy(
            &required,
            &exercises[1].observation,
            &exercises[1].lowering,
            &policy,
            observation_frontier_limits(),
        )
        .expect("one cross-domain held-out construction without expected-proof input");
    let held_out_verified = verify_relational_materialized_candidate(
        &exercises[1],
        &held_out_candidate,
        &learned,
        required,
        &policy,
        &policy_verification,
    )
    .expect("independently verified held-out relational prediction");
    let mut tampered_policy = policy.clone();
    tampered_policy.value_selectors[0][0].fingerprint = Hash32::digest(b"tampered-policy");
    assert_eq!(
        verify_relational_materialized_candidate(
            &exercises[1],
            &held_out_candidate,
            &learned,
            required,
            &tampered_policy,
            &policy_verification,
        ),
        Err(InternalCompressionVerifyError::RelationalPolicyMismatch)
    );
    let verified_constructions = [training_verified, held_out_verified];
    assert!(verified_constructions.iter().all(|verified| verified
        .composition()
        .invocations()
        .len()
        == 2));
    let construction_candidates_generated =
        u64::try_from(training_candidates.len()).expect("small") + 1;
    let construction_candidate_rank = training_candidate.candidates_considered + 1;
    let composition_candidates_without = verified_constructions
        .iter()
        .map(|verified| verified.composition().metrics().candidate_windows_without)
        .sum();
    let composition_candidates_with = verified_constructions
        .iter()
        .map(|verified| verified.composition().metrics().candidate_windows_with)
        .sum();
    let composition_proof_without = verified_constructions
        .iter()
        .map(|verified| verified.composition().metrics().proof_units_without)
        .sum::<u64>();
    let composition_proof_with = verified_constructions
        .iter()
        .map(|verified| verified.composition().metrics().proof_units_with)
        .sum::<u64>();
    let mut certificate_bytes = verified_constructions
        .iter()
        .map(cairn_verifier::VerifiedInternalConstruction::certificate)
        .collect::<Vec<_>>();
    certificate_bytes.extend(exercises.iter().map(|exercise| {
        InternalMechanismVerifier
            .verify_observation_frontier(
                &exercise.observation,
                &exercise.lowering,
                observation_frontier_limits(),
            )
            .expect("independently verified observation lowering")
            .certificate()
    }));
    certificate_bytes.sort_unstable();
    let composition_evidence = Hash32::digest(
        &certificate_bytes
            .iter()
            .flat_map(|hash| hash.as_bytes().iter().copied())
            .collect::<Vec<_>>(),
    );
    curriculum
        .record(DevelopmentalOutcome {
            decision_sequence: composition.sequence,
            objective: composition.objective,
            mechanism_evidence: composition_evidence.to_hex(),
            independently_verified: true,
            held_out_transfer_cases: 1,
            candidate_work_without: composition_candidates_without,
            candidate_work_with: composition_candidates_with,
            examples_without: 16,
            examples_with: 1,
            retained_prior_tasks: 4,
            prior_tasks: 4,
            negative_transfers: 0,
        })
        .expect("record generated composition capability");
    let completed_checkpoint = CurriculumCheckpointArchive
        .persist(
            &cas,
            &curriculum.checkpoint(),
            CurriculumCheckpointMetadata {
                parent_ids: vec![parent(&cas, "completed-generated-objective")],
                producer: producer(),
                budget: budget(),
                created_at_logical: 3_000,
            },
        )
        .expect("persist completed generated objective");
    curriculum_checkpoints.push(completed_checkpoint.artifact_id);
    curriculum = CurriculumCheckpointArchive
        .load(&cas, completed_checkpoint.envelope_blob_id)
        .expect("restore completed generated objective");
    curriculum_restores = curriculum_restores.saturating_add(1);
    assert_eq!(curriculum.completed_objectives(), 5);
    assert_eq!(
        curriculum.select_next(),
        Err(cairn_curriculum::AutonomousCurriculumError::Complete)
    );
    assert_eq!(selected, ["family-a", "family-b", "family-c", "family-d"]);
    assert_eq!(completed_restarts, 2);
    assert_eq!(curriculum_restores, 4);
    assert_eq!(curriculum_checkpoints.len(), 4);
    assert_eq!(
        curriculum
            .mechanisms()
            .get(&MechanismTarget("target-d".to_owned()))
            .expect("restored deprecated mechanism")
            .status,
        LearnedMechanismStatus::Deprecated
    );
    SustainedLearningGate
        .verify(&points)
        .expect("sustained autonomous learning gate");
    assert_eq!(points[0].candidate_work_with, 9);
    assert!(points.iter().all(|point| point.candidate_work_with == 9));
    assert!(points.iter().all(|point| point.examples_without == 4));
    assert!(points.iter().all(|point| point.examples_with == 1));
    assert!(points.iter().all(|point| point.proof_units_with == 4));
    assert_eq!(store.len(), 4);
    eprintln!(
        "G6_AUTONOMOUS_INTERNAL selected=family-a,family-b,family-c,family-d generated=composition observation_lowering=2 relational_transfer=vector-to-unknown-world ordinal_ablation=fail held_out_without_expected=1 world_rules=2 world_predictions=2 world_plan=1 completed=1 generated_core={} generated_objective={} composition_evidence={} mechanisms=1->2->3->4 mechanism_hashes={},{},{},{} construction_generated={} construction_exact_rank_sum={} composition_invocations=2x2 composition_candidates={}->{} composition_examples=16->1 composition_proof={}->{} curriculum_checkpoints={},{},{},{} restarts=2 curriculum_restores=4 deprecated=target-d retained=6/6 negative_transfer=0/10",
        generated.failure_core,
        generated.objective.id.0,
        composition_evidence,
        persisted[0].mechanism,
        persisted[1].mechanism,
        persisted[2].mechanism,
        persisted[3].mechanism,
        construction_candidates_generated,
        construction_candidate_rank,
        composition_candidates_without,
        composition_candidates_with,
        composition_proof_without,
        composition_proof_with,
        curriculum_checkpoints[0],
        curriculum_checkpoints[1],
        curriculum_checkpoints[2],
        curriculum_checkpoints[3],
    );
}

fn families() -> [FamilySpec; 4] {
    [
        FamilySpec {
            objective: "family-a",
            target: "target-a",
            operations: [
                "select-role",
                "filter-relation",
                "map-effect",
                "render-effect",
            ],
            base_episode: 1,
            first_host: (5, 3),
            second_host: (2, 2),
        },
        FamilySpec {
            objective: "family-b",
            target: "target-b",
            operations: [
                "collect-members",
                "order-members",
                "fold-effect",
                "emit-fold",
            ],
            base_episode: 21,
            first_host: (5, 3),
            second_host: (2, 2),
        },
        FamilySpec {
            objective: "family-c",
            target: "target-c",
            operations: ["trace-links", "guard-terminal", "project-path", "emit-path"],
            base_episode: 41,
            first_host: (5, 3),
            second_host: (2, 2),
        },
        FamilySpec {
            objective: "family-d",
            target: "target-d",
            operations: [
                "group-equivalent",
                "select-canonical",
                "map-symbol",
                "emit-symbol",
            ],
            base_episode: 61,
            first_host: (5, 3),
            second_host: (2, 2),
        },
    ]
}

fn objectives() -> Vec<DevelopmentalObjective> {
    let specs = families();
    let hypotheses = [41, 31, 21, 11];
    let objectives = specs
        .iter()
        .zip(hypotheses)
        .enumerate()
        .map(
            |(index, (spec, competing_hypotheses))| DevelopmentalObjective {
                id: ExperienceId(spec.objective.to_owned()),
                axis: [
                    LearningAxis::CausalLaw,
                    LearningAxis::Cardinality,
                    LearningAxis::Value,
                    LearningAxis::Layout,
                ][index],
                produces: MechanismTarget(spec.target.to_owned()),
                prerequisites: BTreeSet::new(),
                competing_hypotheses,
                expected_transfer_microunits: 100,
                action_cost: 1,
                irreversible_risk: 0,
            },
        )
        .collect::<Vec<_>>();
    objectives
}

fn unresolved_composition_core() -> cairn_curriculum::LearningFailureCore {
    let integer = Type::Int(IntRange { lower: 0, upper: 8 });
    let left = VarName("anonymous-left".to_owned());
    let right = VarName("anonymous-right".to_owned());
    let examples = [(0, 0, 0), (0, 2, 2), (3, 0, 3), (2, 4, 6)]
        .into_iter()
        .map(|(a, b, output)| {
            let mut inputs = RuntimeEnvironment::new();
            inputs.insert(left.clone(), Value::Int(a));
            inputs.insert(right.clone(), Value::Int(b));
            BindingExample {
                inputs,
                output: Value::Int(output),
            }
        })
        .collect();
    let problem = InventionProblem {
        definition_name: "anonymous-unresolved-composition".to_owned(),
        parameters: vec![
            Binder {
                name: left,
                ty: integer.clone(),
            },
            Binder {
                name: right,
                ty: integer.clone(),
            },
        ],
        return_type: integer,
        literals: vec![Value::Int(0)],
        demonstrations: examples,
        limits: InventionLimits::default(),
    };
    FailureCoreExtractor
        .extract(
            &problem,
            &InventionError::CandidateCapacity { maximum: 1_000 },
        )
        .expect("redacted failure core")
}

fn source_cohort(spec: &FamilySpec) -> [ProofGraph; 4] {
    [
        host_graph(spec, spec.base_episode, 1, 1, "triangle", "red"),
        host_graph(spec, spec.base_episode + 1, 2, 2, "circle", "blue"),
        host_graph(spec, spec.base_episode + 2, 3, 1, "segment", "green"),
        host_graph(spec, spec.base_episode + 3, 1, 3, "polygon", "yellow"),
    ]
}

fn held_out_hosts(spec: &FamilySpec) -> [ProofGraph; 2] {
    [
        host_graph(
            spec,
            spec.base_episode + 10,
            spec.first_host.0,
            spec.first_host.1,
            "spiral",
            "violet",
        ),
        host_graph(
            spec,
            spec.base_episode + 11,
            spec.second_host.0,
            spec.second_host.1,
            "wave",
            "orange",
        ),
    ]
}

fn host_graph(
    spec: &FamilySpec,
    episode: u8,
    prefix_nodes: usize,
    suffix_nodes: usize,
    role: &str,
    effect: &str,
) -> ProofGraph {
    let mut nodes = vec![ProofNode {
        operation: "host-observation".to_owned(),
        output_type: ProofValueType::EntitySet,
        inputs: vec![],
        arguments: vec![datum(ProofValueType::Atom, &format!("world-{episode}"))],
    }];
    for index in 1..prefix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-prefix-{episode}-{index}"),
            output_type: ProofValueType::Integer,
            inputs: vec![],
            arguments: vec![datum(ProofValueType::Integer, &index.to_string())],
        });
    }
    let start = nodes.len();
    nodes.extend([
        ProofNode {
            operation: spec.operations[0].to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![0],
            arguments: vec![datum(ProofValueType::Entity, role)],
        },
        ProofNode {
            operation: spec.operations[1].to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![u32::try_from(start).expect("small")],
            arguments: vec![datum(ProofValueType::Relation, "inside")],
        },
        ProofNode {
            operation: spec.operations[2].to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 1).expect("small")],
            arguments: vec![datum(ProofValueType::Effect, effect)],
        },
        ProofNode {
            operation: spec.operations[3].to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 2).expect("small")],
            arguments: vec![],
        },
    ]);
    let mut previous = start + 3;
    for index in 0..suffix_nodes {
        nodes.push(ProofNode {
            operation: format!("host-suffix-{episode}-{index}"),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(previous).expect("small")],
            arguments: vec![],
        });
        previous += 1;
    }
    ProofGraph {
        episode: Hash32::digest(&[episode]),
        output: u32::try_from(nodes.len() - 1).expect("small"),
        nodes,
    }
}

fn negative_host(episode: u8) -> ProofGraph {
    let spec = FamilySpec {
        objective: "negative",
        target: "negative",
        operations: ["unrelated-a", "unrelated-b", "unrelated-c", "unrelated-d"],
        base_episode: episode,
        first_host: (2, 1),
        second_host: (2, 1),
    };
    host_graph(&spec, episode, 2, 1, "noise", "none")
}

fn materialize_composition_exercises(
    request: &cairn_curriculum::DevelopmentalExerciseRequest,
    first: &FamilySpec,
    second: &FamilySpec,
    required: [Hash32; 2],
    runtime: &CognitionRuntime,
) -> MaterializedCurriculum {
    assert_eq!(request.need, DevelopmentalNeed::MechanismComposition);
    assert_eq!(request.minimum_independent_variations, 2);
    assert_eq!(
        request.prerequisites,
        BTreeSet::from([
            MechanismTarget(first.target.to_owned()),
            MechanismTarget(second.target.to_owned()),
        ])
    );
    let training_values = [
        (
            "static-vector-entity-a".to_owned(),
            "static-vector-effect-a".to_owned(),
        ),
        (
            "static-vector-entity-b".to_owned(),
            "static-vector-effect-b".to_owned(),
        ),
    ];
    let training_observation = relational_exercise_observation(
        91,
        "urn:cairn:vector-relational:v1",
        &training_values,
        false,
        false,
    );
    let training_lowering = runtime
        .lower_relational_observation(&training_observation, observation_frontier_limits())
        .expect("runtime training observation lowering");
    let training_expected = expected_composition(
        &training_lowering.frontier,
        first,
        second,
        required,
        &training_values,
    );

    let world = learn_unknown_world_exercise(runtime);
    let held_out_observation = relational_exercise_observation(
        92,
        "urn:cairn:interactive-world-relational:v1",
        &world.values,
        true,
        true,
    );
    let held_out_lowering = runtime
        .lower_relational_observation(&held_out_observation, observation_frontier_limits())
        .expect("runtime held-out observation lowering");
    let held_out_expected = expected_composition(
        &held_out_lowering.frontier,
        first,
        second,
        required,
        &world.values,
    );
    MaterializedCurriculum {
        exercises: [
            MaterializedConstructionExercise {
                observation: training_observation,
                lowering: training_lowering,
                expected: training_expected,
            },
            MaterializedConstructionExercise {
                observation: held_out_observation,
                lowering: held_out_lowering,
                expected: held_out_expected,
            },
        ],
        learned_world_rules: world.rules,
        world_prediction_cases: world.prediction_cases,
        world_plan_length: world.plan_length,
    }
}

fn learn_unknown_world_exercise(runtime: &CognitionRuntime) -> LearnedWorldExercise {
    let space = StateSpace::new(vec![-5, -5], vec![5, 5]).expect("world bounds");
    let actions = [ActionId(7), ActionId(11)];
    let transitions = [
        TransitionObservation {
            before: WorldState(vec![0, 0]),
            action: actions[0],
            after: WorldState(vec![1, 0]),
        },
        TransitionObservation {
            before: WorldState(vec![-2, 3]),
            action: actions[0],
            after: WorldState(vec![-1, 3]),
        },
        TransitionObservation {
            before: WorldState(vec![0, 0]),
            action: actions[1],
            after: WorldState(vec![0, 1]),
        },
        TransitionObservation {
            before: WorldState(vec![2, -2]),
            action: actions[1],
            after: WorldState(vec![2, -1]),
        },
    ];
    let world = runtime
        .learn_and_plan_world(
            &space,
            &actions,
            &transitions,
            &[
                GoalObservation {
                    state: WorldState(vec![1, 0]),
                    achieved: true,
                },
                GoalObservation {
                    state: WorldState(vec![0, 0]),
                    achieved: false,
                },
            ],
            &WorldState(vec![0, 0]),
            PlanLimits {
                maximum_visited_states: 32,
                maximum_plan_length: 4,
            },
        )
        .expect("learn unknown action laws and plan");
    let values = world
        .transition_theory
        .rules()
        .map(|rule| {
            (
                format!("learned-world-action-{}", rule.action.0),
                format!(
                    "learned-world-effect-{}-{}",
                    rule.action.0,
                    rule.delta
                        .iter()
                        .map(i64::to_string)
                        .collect::<Vec<_>>()
                        .join(":"),
                ),
            )
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("two independently learned action laws");
    let predictions = [
        (WorldState(vec![3, 2]), actions[0], WorldState(vec![4, 2])),
        (WorldState(vec![3, 2]), actions[1], WorldState(vec![3, 3])),
    ];
    assert!(predictions.iter().all(|(before, action, expected)| {
        world.transition_theory.predict(before, *action).as_ref() == Ok(expected)
    }));
    LearnedWorldExercise {
        values,
        rules: world.transition_theory.rules().count(),
        prediction_cases: predictions.len(),
        plan_length: world.plan.actions.len(),
    }
}

fn relational_exercise_observation(
    episode: u8,
    modality: &str,
    values: &[(String, String); 2],
    reverse_input: bool,
    include_distractor: bool,
) -> RelationalObservation {
    let mut elements = Vec::new();
    let mut relations = Vec::new();
    for (index, ((entity, effect), marker_type)) in values
        .iter()
        .zip([EvidenceValueType::Boolean, EvidenceValueType::Integer])
        .enumerate()
    {
        append_relational_role(
            &mut elements,
            &mut relations,
            episode,
            index,
            entity,
            effect,
            marker_type,
        );
    }
    if include_distractor {
        append_relational_role(
            &mut elements,
            &mut relations,
            episode,
            2,
            "unrelated-world-entity",
            "unrelated-world-effect",
            EvidenceValueType::Action,
        );
    }
    if reverse_input {
        elements.reverse();
        relations.reverse();
    }
    RelationalObservation::new(
        ModalityId(modality.to_owned()),
        Some(u64::from(episode)),
        SourceReference {
            locator: format!("generated:g6-relational/{episode}"),
            content_hash: Hash32::digest(&[episode, u8::from(include_distractor)]),
        },
        ObservationCertainty::Exact,
        elements,
        relations,
        RelationalObservationLimits::default(),
    )
    .expect("generic relational exercise observation")
}

fn append_relational_role(
    elements: &mut Vec<EvidenceElement>,
    relations: &mut Vec<EvidenceRelation>,
    episode: u8,
    index: usize,
    entity: &str,
    effect: &str,
    marker_type: EvidenceValueType,
) {
    let entity_element = EvidenceElement {
        id: EvidenceElementId(Hash32::digest(
            format!("entity-node-{episode}-{index}").as_bytes(),
        )),
        value_type: EvidenceValueType::Entity,
        value: Hash32::digest(entity.as_bytes()),
    };
    let effect_element = EvidenceElement {
        id: EvidenceElementId(Hash32::digest(
            format!("effect-node-{episode}-{index}").as_bytes(),
        )),
        value_type: EvidenceValueType::Effect,
        value: Hash32::digest(effect.as_bytes()),
    };
    let marker = EvidenceElement {
        id: EvidenceElementId(Hash32::digest(
            format!("marker-node-{episode}-{index}").as_bytes(),
        )),
        value_type: marker_type,
        value: Hash32::digest(format!("opaque-marker-{episode}-{index}").as_bytes()),
    };
    let predicate = Hash32::digest(b"opaque-causal-link");
    relations.extend([
        EvidenceRelation {
            predicate,
            participants: vec![entity_element.id, effect_element.id],
        },
        EvidenceRelation {
            predicate,
            participants: vec![entity_element.id, marker.id],
        },
    ]);
    elements.extend([entity_element, effect_element, marker]);
}

fn observation_frontier_limits() -> ObservationFrontierLimits {
    ObservationFrontierLimits {
        observation: RelationalObservationLimits::default(),
        maximum_frontier_nodes: 64,
    }
}

fn expected_composition(
    frontier: &ProofFrontier,
    first: &FamilySpec,
    second: &FamilySpec,
    required: [Hash32; 2],
    values: &[(String, String); 2],
) -> ProofGraph {
    let mut nodes = frontier.nodes.clone();
    let mut families = [
        (
            required[0],
            first,
            values[0].0.as_str(),
            values[0].1.as_str(),
        ),
        (
            required[1],
            second,
            values[1].0.as_str(),
            values[1].1.as_str(),
        ),
    ];
    families.sort_unstable_by_key(|family| family.0);
    let outputs = families
        .into_iter()
        .map(|(_, family, role, effect)| append_family_occurrence(&mut nodes, family, role, effect))
        .collect::<Vec<_>>();
    nodes.push(ProofNode {
        operation: "join-learned-results".to_owned(),
        output_type: ProofValueType::Effect,
        inputs: outputs,
        arguments: vec![],
    });
    ProofGraph {
        episode: frontier.episode,
        output: u32::try_from(nodes.len() - 1).expect("small"),
        nodes,
    }
}

fn append_family_occurrence(
    nodes: &mut Vec<ProofNode>,
    spec: &FamilySpec,
    role: &str,
    effect: &str,
) -> u32 {
    let start = nodes.len();
    nodes.extend([
        ProofNode {
            operation: spec.operations[0].to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![0],
            arguments: vec![datum(ProofValueType::Entity, role)],
        },
        ProofNode {
            operation: spec.operations[1].to_owned(),
            output_type: ProofValueType::EntitySet,
            inputs: vec![u32::try_from(start).expect("small")],
            arguments: vec![datum(ProofValueType::Relation, "inside")],
        },
        ProofNode {
            operation: spec.operations[2].to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 1).expect("small")],
            arguments: vec![datum(ProofValueType::Effect, effect)],
        },
        ProofNode {
            operation: spec.operations[3].to_owned(),
            output_type: ProofValueType::Effect,
            inputs: vec![u32::try_from(start + 2).expect("small")],
            arguments: vec![],
        },
    ]);
    u32::try_from(start + 3).expect("small")
}

const fn limits() -> InternalMiningLimits {
    InternalMiningLimits {
        minimum_nodes: 4,
        maximum_nodes: 4,
        minimum_independent_episodes: 4,
        maximum_windows_per_episode: 64,
    }
}

fn persist_family(
    cas: &Cas,
    sources: &[ProofGraph],
    hosts: &[ProofGraph],
    receipts: &[cairn_verifier::VerifiedInternalReuse],
) -> (Vec<BlobId>, Vec<BlobId>) {
    let graph_archive = ProofGraphArchive;
    let stored = sources
        .iter()
        .chain(hosts)
        .enumerate()
        .map(|(index, graph)| {
            graph_archive
                .persist(
                    cas,
                    graph,
                    ProofGraphArtifactMetadata {
                        parent_ids: vec![parent(cas, &format!("graph-{index}-{}", graph.episode))],
                        producer: producer(),
                        budget: budget(),
                        created_at_logical: u64::try_from(index).expect("small"),
                    },
                )
                .expect("persist curriculum proof")
        })
        .collect::<Vec<_>>();
    let parents = stored
        .iter()
        .map(|item| item.artifact_id)
        .collect::<Vec<_>>();
    let receipt_archive = InternalReuseArtifactArchive;
    let receipts = receipts
        .iter()
        .enumerate()
        .map(|(index, receipt)| {
            receipt_archive
                .persist(
                    cas,
                    receipt,
                    ReuseArtifactMetadata {
                        parent_ids: parents.clone(),
                        producer: producer(),
                        budget: budget(),
                        created_at_logical: u64::try_from(index).expect("small") + 100,
                    },
                )
                .expect("persist curriculum reuse")
                .envelope_blob_id
        })
        .collect();
    (
        stored.iter().map(|item| item.envelope_blob_id).collect(),
        receipts,
    )
}

fn reconstruct_store(cas: &Cas, families: &[PersistedFamily]) -> InternalMechanismStore {
    let graph_archive = ProofGraphArchive;
    let receipt_archive = InternalReuseArtifactArchive;
    let mut store = InternalMechanismStore::new();
    for family in families {
        let graphs = family
            .graph_blobs
            .iter()
            .map(|blob| graph_archive.load(cas, *blob).expect("reload graph"))
            .collect::<Vec<_>>();
        let sources = &graphs[..4];
        let hosts = &graphs[4..];
        let proposal = Consolidator
            .mine_internal_proof_subgraphs(sources, limits())
            .expect("remine family");
        let mechanism = Consolidator
            .compile_internal_proof_mechanism(&proposal)
            .expect("recompile family");
        assert_eq!(proposal.semantic_hash(), family.proposal);
        assert_eq!(mechanism.semantic_hash(), family.mechanism);
        let compilation = InternalMechanismVerifier
            .verify_compilation(&proposal, &mechanism, sources)
            .expect("reverify family");
        let id = store
            .insert_provisional(mechanism.clone(), compilation.clone())
            .expect("restore provisional family");
        for (blob, host) in family.receipt_blobs.iter().zip(hosts) {
            let artifact = receipt_archive.load(cas, *blob).expect("reload receipt");
            let binding = Consolidator
                .match_compiled_internal_mechanism(&mechanism, host, 64)
                .expect("rebind family")
                .remove(0);
            let receipt = receipt_archive
                .replay(
                    artifact,
                    InternalReuseReplay {
                        proposal: &proposal,
                        mechanism: &mechanism,
                        compilation: &compilation,
                        sources,
                        held_out: host,
                        binding: &binding,
                        maximum_match_windows: 64,
                    },
                )
                .expect("replay family receipt");
            store
                .record_runtime_reuse(id, receipt)
                .expect("restore family ledger");
        }
        assert_eq!(
            store.get(id).expect("restored family").status(),
            ModuleStatus::Persistent
        );
    }
    store
}

fn datum(ty: ProofValueType, value: &str) -> ProofDatum {
    ProofDatum::digest(ty, value.as_bytes())
}

fn parent(cas: &Cas, name: &str) -> cairn_core::ArtifactId {
    let envelope = ArtifactEnvelope::new(
        SchemaRef::new("test.autonomous-curriculum", 1).expect("schema"),
        ArtifactKind::new("test.autonomous-curriculum").expect("kind"),
        SemanticsVersion::new(1, 0, 0),
        Vec::new(),
        None,
        producer(),
        budget(),
        0,
        CanonicalValue::Text(name.to_owned()),
    )
    .expect("parent");
    cas.put_envelope(&envelope)
        .expect("persist parent")
        .artifact_id
}

fn producer() -> ProducerRecord {
    ProducerRecord {
        component: "g6-autonomous-internal-curriculum".to_owned(),
        version: "0.1.0".to_owned(),
        commit: "test".to_owned(),
        host_run_id: "frozen".to_owned(),
        agent_id: Some("curriculum".to_owned()),
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
                "cairn-g6-autonomous-{}-{sequence}",
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
