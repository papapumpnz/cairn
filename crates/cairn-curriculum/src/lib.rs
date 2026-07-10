//! Deterministic developmental curriculum selection from support uncertainty.

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExperienceId(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperienceCandidate {
    pub id: ExperienceId,
    pub expected_information_microunits: u64,
    pub expected_transfer_microunits: u64,
    pub underused_mechanism_bonus: u64,
    pub action_cost: u64,
    pub irreversible_risk: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurriculumError {
    EmptyCandidates,
    DuplicateCandidate,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Curriculum;

impl Curriculum {
    /// Choose maximum expected learning value with stable ID tie-breaking.
    ///
    /// # Errors
    ///
    /// Rejects an empty or duplicate candidate set.
    pub fn select<'a>(
        &self,
        candidates: &'a [ExperienceCandidate],
    ) -> Result<&'a ExperienceCandidate, CurriculumError> {
        if candidates.is_empty() {
            return Err(CurriculumError::EmptyCandidates);
        }
        let mut ids = candidates
            .iter()
            .map(|candidate| &candidate.id)
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        if ids.len() != candidates.len() {
            return Err(CurriculumError::DuplicateCandidate);
        }
        candidates
            .iter()
            .max_by_key(|candidate| (score(candidate), std::cmp::Reverse(&candidate.id)))
            .ok_or(CurriculumError::EmptyCandidates)
    }
}

fn score(candidate: &ExperienceCandidate) -> i128 {
    i128::from(candidate.expected_information_microunits)
        + i128::from(candidate.expected_transfer_microunits)
        + i128::from(candidate.underused_mechanism_bonus)
        - i128::from(candidate.action_cost)
        - i128::from(candidate.irreversible_risk)
}
