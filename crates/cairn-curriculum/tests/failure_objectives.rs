use cairn_curriculum::{
    AutonomousCurriculum, AutonomousCurriculumError, DeterminismStatus, DevelopmentalNeed,
    DevelopmentalObjective, DevelopmentalOutcome, ExperienceId, FailureDrivenObjectiveGenerator,
    LearningAxis, LearningFailureCore, MechanismTarget, ObjectiveGenerationError, RedactionStatus,
    SearchFailure, ValueClass, FAILURE_CORE_SEMANTICS_VERSION,
};
use std::collections::BTreeSet;

#[test]
fn redacted_failure_generates_a_deterministic_composition_objective() {
    let support = BTreeSet::from([
        MechanismTarget("filter-role".to_owned()),
        MechanismTarget("map-effect".to_owned()),
    ]);
    let generated = FailureDrivenObjectiveGenerator
        .generate(&composition_core(), &support)
        .expect("generated objective");
    assert_eq!(generated.need, DevelopmentalNeed::MechanismComposition);
    assert_eq!(generated.objective.axis, LearningAxis::Composition);
    assert_eq!(generated.objective.prerequisites, support);
    assert!(generated.objective.id.0.starts_with("generated-"));
    assert!(generated.objective.produces.0.starts_with("mechanism-"));
    let request = generated.exercise_request();
    assert_eq!(request.objective, generated.objective.id);
    assert_eq!(request.prerequisites, support);
    assert_eq!(request.minimum_independent_variations, 2);

    let reversed = BTreeSet::from([
        MechanismTarget("map-effect".to_owned()),
        MechanismTarget("filter-role".to_owned()),
    ]);
    assert_eq!(
        generated,
        FailureDrivenObjectiveGenerator
            .generate(&composition_core(), &reversed)
            .expect("same structural objective")
    );
}

#[test]
fn objective_generation_classifies_latent_and_collection_gaps_and_rejects_content() {
    let mut latent = composition_core();
    latent.determinism = DeterminismStatus::ConflictingCompleteInputs;
    assert_eq!(
        FailureDrivenObjectiveGenerator
            .generate(&latent, &BTreeSet::new())
            .expect("latent objective")
            .need,
        DevelopmentalNeed::LatentContext
    );
    let mut collection = composition_core();
    collection.parameter_types = vec![ValueClass::Collection];
    collection.return_type = ValueClass::Collection;
    assert_eq!(
        FailureDrivenObjectiveGenerator
            .generate(&collection, &BTreeSet::new())
            .expect("collection objective")
            .need,
        DevelopmentalNeed::VariableCardinality
    );
    collection.redaction = RedactionStatus::ContainsRestrictedContent;
    assert_eq!(
        FailureDrivenObjectiveGenerator.generate(&collection, &BTreeSet::new()),
        Err(ObjectiveGenerationError::RestrictedContent)
    );
}

#[test]
fn generated_objective_enters_a_live_curriculum_and_replays_at_its_event_boundary() {
    let support = BTreeSet::from([
        MechanismTarget("filter-role".to_owned()),
        MechanismTarget("map-effect".to_owned()),
    ]);
    let initial = support
        .iter()
        .enumerate()
        .map(|(index, target)| DevelopmentalObjective {
            id: ExperienceId(format!("seed-{index}")),
            axis: LearningAxis::CausalLaw,
            produces: target.clone(),
            prerequisites: BTreeSet::new(),
            competing_hypotheses: 10 - u64::try_from(index).expect("small"),
            expected_transfer_microunits: 10,
            action_cost: 1,
            irreversible_risk: 0,
        })
        .collect();
    let mut curriculum = AutonomousCurriculum::new(initial, 8).expect("curriculum");
    for prior_tasks in 0..2 {
        let decision = curriculum.select_next().expect("seed decision");
        curriculum
            .record(success(&decision, prior_tasks))
            .expect("seed learning");
    }
    let generated = FailureDrivenObjectiveGenerator
        .generate(&composition_core(), &support)
        .expect("new objective");
    curriculum
        .add_objective(generated.objective.clone())
        .expect("admit generated objective");

    let checkpoint = curriculum.checkpoint();
    assert_eq!(checkpoint.objectives().len(), 2, "only initial objectives");
    let mut restored = AutonomousCurriculum::restore(&checkpoint).expect("event replay");
    let selected = restored
        .select_next()
        .expect("generated objective selected");
    assert_eq!(selected.objective, generated.objective.id);
    assert_eq!(selected.axis, LearningAxis::Composition);
    assert_eq!(selected.produces, generated.objective.produces);
    assert_eq!(restored.mechanisms().len(), 2);

    assert_eq!(
        restored.add_objective(generated.objective),
        Err(AutonomousCurriculumError::ObjectiveAdmissionWhileOutstanding)
    );
}

fn composition_core() -> LearningFailureCore {
    LearningFailureCore {
        semantics_version: FAILURE_CORE_SEMANTICS_VERSION.to_owned(),
        failure: SearchFailure::CandidateCapacity { maximum: 1_000 },
        parameter_types: vec![ValueClass::Entity, ValueClass::Relation],
        return_type: ValueClass::Entity,
        demonstrations: 6,
        varying_parameters: 2,
        distinct_output_classes: 3,
        constant_explanation_possible: false,
        direct_projection_candidates: 0,
        single_parameter_explanations: 0,
        exact_two_parameter_families: BTreeSet::new(),
        exact_three_parameter_families: BTreeSet::new(),
        determinism: DeterminismStatus::Deterministic,
        minimum_dependency_arity_lower_bound: 2,
        redaction: RedactionStatus::ContentFree,
    }
}

fn success(
    decision: &cairn_curriculum::CurriculumDecision,
    prior_tasks: u32,
) -> DevelopmentalOutcome {
    DevelopmentalOutcome {
        decision_sequence: decision.sequence,
        objective: decision.objective.clone(),
        mechanism_evidence: format!("verified-{}", decision.sequence),
        independently_verified: true,
        held_out_transfer_cases: 2,
        candidate_work_without: 20,
        candidate_work_with: 4,
        examples_without: 4,
        examples_with: 1,
        retained_prior_tasks: prior_tasks,
        prior_tasks,
        negative_transfers: 0,
    }
}
