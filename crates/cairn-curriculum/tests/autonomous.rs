use cairn_curriculum::{
    AutonomousCurriculum, AutonomousCurriculumError, DevelopmentalObjective, DevelopmentalOutcome,
    ExperienceId, LearnedMechanismStatus, LearningAxis, MechanismTarget,
};
use std::collections::BTreeSet;

#[test]
fn prerequisites_unlock_novel_composition_without_scripted_stage_order() {
    let mut curriculum = AutonomousCurriculum::new(objectives(), 8).expect("curriculum");
    let first = curriculum.select_next().expect("first decision");
    assert_eq!(first.objective, ExperienceId("base-a".to_owned()));
    curriculum
        .record(success(&first, 0))
        .expect("first verified result");

    let second = curriculum.select_next().expect("second decision");
    assert_eq!(second.objective, ExperienceId("base-b".to_owned()));
    curriculum
        .record(success(&second, 1))
        .expect("second verified result");

    let third = curriculum.select_next().expect("composition now eligible");
    assert_eq!(third.axis, LearningAxis::Composition);
    assert_eq!(third.objective, ExperienceId("compose-ab".to_owned()));
    assert!(third.underuse_bonus > 0);
    curriculum
        .record(success(&third, 2))
        .expect("composed result");
    assert_eq!(curriculum.completed_objectives(), 3);
    assert_eq!(curriculum.mechanisms().len(), 3);
    assert_eq!(curriculum.history().len(), 3);
}

#[test]
fn deterministic_selection_capacity_and_prerequisite_deadlock_are_explicit() {
    let mut reversed = objectives();
    reversed.reverse();
    let mut left = AutonomousCurriculum::new(objectives(), 8).expect("left");
    let mut right = AutonomousCurriculum::new(reversed, 8).expect("right");
    assert_eq!(
        left.select_next().expect("left choice"),
        right.select_next().expect("right choice")
    );
    assert_eq!(
        left.select_next(),
        Err(AutonomousCurriculumError::OutstandingDecision)
    );

    let blocked = DevelopmentalObjective {
        id: ExperienceId("blocked".to_owned()),
        axis: LearningAxis::Composition,
        produces: MechanismTarget("blocked-result".to_owned()),
        prerequisites: BTreeSet::from([MechanismTarget("missing".to_owned())]),
        competing_hypotheses: 10,
        expected_transfer_microunits: 1,
        action_cost: 1,
        irreversible_risk: 0,
    };
    assert_eq!(
        AutonomousCurriculum::new(vec![blocked], 1)
            .expect("blocked curriculum")
            .select_next(),
        Err(AutonomousCurriculumError::BlockedByPrerequisites)
    );

    let mut capped = AutonomousCurriculum::new(objectives(), 1).expect("capped");
    let decision = capped.select_next().expect("only decision");
    capped.record(success(&decision, 0)).expect("record only");
    assert_eq!(
        capped.select_next(),
        Err(AutonomousCurriculumError::Capacity)
    );
}

#[test]
fn unverified_results_and_counterexamples_fail_closed() {
    let mut curriculum = AutonomousCurriculum::new(objectives(), 8).expect("curriculum");
    let decision = curriculum.select_next().expect("decision");
    let mut unverified = success(&decision, 0);
    unverified.independently_verified = false;
    assert_eq!(
        curriculum.record(unverified),
        Err(AutonomousCurriculumError::UnverifiedOutcome)
    );
    curriculum
        .record(success(&decision, 0))
        .expect("verified retry");
    let target = &decision.produces;
    assert_eq!(
        curriculum
            .record_counterexample(target, "counterexample-a".to_owned(), true)
            .expect("first counterexample"),
        LearnedMechanismStatus::Quarantined
    );
    assert_eq!(
        curriculum.record_counterexample(target, "counterexample-a".to_owned(), true),
        Err(AutonomousCurriculumError::DuplicateCounterexample)
    );
    assert_eq!(
        curriculum
            .record_counterexample(target, "counterexample-b".to_owned(), true)
            .expect("second counterexample"),
        LearnedMechanismStatus::Deprecated
    );
    assert_eq!(
        curriculum.mechanisms().get(target).expect("record").status,
        LearnedMechanismStatus::Deprecated
    );
}

fn objectives() -> Vec<DevelopmentalObjective> {
    let a = MechanismTarget("mechanism-a".to_owned());
    let b = MechanismTarget("mechanism-b".to_owned());
    vec![
        DevelopmentalObjective {
            id: ExperienceId("base-a".to_owned()),
            axis: LearningAxis::CausalLaw,
            produces: a.clone(),
            prerequisites: BTreeSet::new(),
            competing_hypotheses: 10,
            expected_transfer_microunits: 10,
            action_cost: 1,
            irreversible_risk: 0,
        },
        DevelopmentalObjective {
            id: ExperienceId("base-b".to_owned()),
            axis: LearningAxis::Cardinality,
            produces: b.clone(),
            prerequisites: BTreeSet::new(),
            competing_hypotheses: 8,
            expected_transfer_microunits: 10,
            action_cost: 1,
            irreversible_risk: 0,
        },
        DevelopmentalObjective {
            id: ExperienceId("later-independent".to_owned()),
            axis: LearningAxis::Layout,
            produces: MechanismTarget("mechanism-c".to_owned()),
            prerequisites: BTreeSet::new(),
            competing_hypotheses: 2,
            expected_transfer_microunits: 1,
            action_cost: 1,
            irreversible_risk: 0,
        },
        DevelopmentalObjective {
            id: ExperienceId("compose-ab".to_owned()),
            axis: LearningAxis::Composition,
            produces: MechanismTarget("mechanism-ab".to_owned()),
            prerequisites: BTreeSet::from([a, b]),
            competing_hypotheses: 100,
            expected_transfer_microunits: 1_000_000,
            action_cost: 2,
            irreversible_risk: 0,
        },
    ]
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
        held_out_transfer_cases: 3,
        candidate_work_without: 100,
        candidate_work_with: 10,
        examples_without: 4,
        examples_with: 1,
        retained_prior_tasks: prior_tasks,
        prior_tasks,
        negative_transfers: 0,
    }
}
