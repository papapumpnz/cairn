use cairn_core::{
    CanonicalValue, Demonstration, Hash32, LearningProblem, ModalityId, Observation,
    ObservationCertainty, ObservationId, PredictionRequest, SourceReference, TaskId,
};
use cairn_runtime::{CognitionRuntime, ExperienceError};

#[test]
fn admits_a_reference_closed_modality_neutral_experience() {
    let before = observation("before", 1);
    let after = observation("after", 2);
    let problem = LearningProblem {
        id: TaskId(Hash32::digest(b"generic-task")),
        observations: vec![before.clone(), after.clone()],
        demonstrations: vec![Demonstration {
            context: vec![before.id],
            intervention: None,
            outcome: vec![after.id],
        }],
        requested_predictions: vec![PredictionRequest {
            context: vec![before.id],
            intervention: None,
        }],
        attempt_limit: 1,
    };
    let admitted = CognitionRuntime::new()
        .admit_experience(&problem)
        .expect("generic admission");
    assert_eq!(admitted.observations, 2);
    assert_eq!(admitted.demonstrations, 1);
    assert_eq!(admitted.requested_predictions, 1);
}

#[test]
fn rejects_references_not_carried_by_the_experience() {
    let before = observation("before", 1);
    let missing = ObservationId(Hash32::digest(b"missing"));
    let problem = LearningProblem {
        id: TaskId(Hash32::digest(b"invalid-task")),
        observations: vec![before.clone()],
        demonstrations: vec![Demonstration {
            context: vec![before.id],
            intervention: None,
            outcome: vec![missing],
        }],
        requested_predictions: vec![PredictionRequest {
            context: vec![before.id],
            intervention: None,
        }],
        attempt_limit: 1,
    };
    assert_eq!(
        CognitionRuntime::new().admit_experience(&problem),
        Err(ExperienceError::UnknownReference(missing))
    );
}

fn observation(role: &str, value: u64) -> Observation {
    Observation::new(
        ModalityId("urn:test:generic:v1".to_owned()),
        None,
        SourceReference {
            locator: format!("urn:test:{role}"),
            content_hash: Hash32::digest(role.as_bytes()),
        },
        CanonicalValue::Unsigned(value),
        ObservationCertainty::Exact,
    )
    .expect("valid observation")
}
