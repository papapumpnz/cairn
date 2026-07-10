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

/// One frozen measurement taken after a consolidation boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevelopmentalPoint {
    pub available_mechanisms: u32,
    pub solved_evaluation_tasks: u32,
    pub evaluation_tasks: u32,
    pub candidate_executions_to_acquire_next: u64,
    pub cpu_nanos_to_acquire_next: u64,
    pub examples_to_acquire_next: u32,
    pub interventions_to_acquire_next: u32,
    pub maximum_composition_depth: u32,
    pub retained_prior_tasks: u32,
    pub prior_tasks: u32,
    pub negative_transfers: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScalingError {
    InsufficientStages,
    EmptyEvaluation,
    MechanismCountDidNotGrow,
    CapabilityRegressed,
    CapabilityDidNotGrow,
    MarginalSearchDidNotFall,
    MarginalCpuDidNotFall,
    MarginalExamplesDidNotFall,
    MarginalInterventionsDidNotFall,
    CompositionDepthDidNotGrow,
    CatastrophicForgetting,
    NegativeTransfer,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DevelopmentalGate;

impl DevelopmentalGate {
    /// Check the signature of developmental learning on frozen measurements.
    ///
    /// Search, examples, and interventions compare the first independently
    /// acquired mechanisms with the final mechanism acquired by reuse or
    /// composition. Intermediate capability and retention may never regress.
    ///
    /// # Errors
    ///
    /// Returns the first violated scaling invariant.
    pub fn verify(&self, points: &[DevelopmentalPoint]) -> Result<(), ScalingError> {
        if points.len() < 3 {
            return Err(ScalingError::InsufficientStages);
        }
        if points.iter().any(|point| point.evaluation_tasks == 0) {
            return Err(ScalingError::EmptyEvaluation);
        }
        for pair in points.windows(2) {
            if pair[1].available_mechanisms <= pair[0].available_mechanisms {
                return Err(ScalingError::MechanismCountDidNotGrow);
            }
            if u64::from(pair[1].solved_evaluation_tasks) * u64::from(pair[0].evaluation_tasks)
                < u64::from(pair[0].solved_evaluation_tasks) * u64::from(pair[1].evaluation_tasks)
            {
                return Err(ScalingError::CapabilityRegressed);
            }
            if pair[1].retained_prior_tasks < pair[1].prior_tasks {
                return Err(ScalingError::CatastrophicForgetting);
            }
            if pair[1].negative_transfers != 0 {
                return Err(ScalingError::NegativeTransfer);
            }
        }
        let first = &points[0];
        let Some(last) = points.last() else {
            return Err(ScalingError::InsufficientStages);
        };
        if u64::from(last.solved_evaluation_tasks) * u64::from(first.evaluation_tasks)
            <= u64::from(first.solved_evaluation_tasks) * u64::from(last.evaluation_tasks)
        {
            return Err(ScalingError::CapabilityDidNotGrow);
        }
        if last.candidate_executions_to_acquire_next >= first.candidate_executions_to_acquire_next {
            return Err(ScalingError::MarginalSearchDidNotFall);
        }
        if last.cpu_nanos_to_acquire_next >= first.cpu_nanos_to_acquire_next {
            return Err(ScalingError::MarginalCpuDidNotFall);
        }
        if last.examples_to_acquire_next >= first.examples_to_acquire_next {
            return Err(ScalingError::MarginalExamplesDidNotFall);
        }
        if last.interventions_to_acquire_next >= first.interventions_to_acquire_next {
            return Err(ScalingError::MarginalInterventionsDidNotFall);
        }
        if last.maximum_composition_depth <= first.maximum_composition_depth {
            return Err(ScalingError::CompositionDepthDidNotGrow);
        }
        Ok(())
    }
}

fn score(candidate: &ExperienceCandidate) -> i128 {
    i128::from(candidate.expected_information_microunits)
        + i128::from(candidate.expected_transfer_microunits)
        + i128::from(candidate.underused_mechanism_bonus)
        - i128::from(candidate.action_cost)
        - i128::from(candidate.irreversible_risk)
}

#[cfg(test)]
mod tests {
    use super::{DevelopmentalGate, DevelopmentalPoint, ScalingError};

    fn point(mechanisms: u32, solved: u32, search: u64, examples: u32) -> DevelopmentalPoint {
        DevelopmentalPoint {
            available_mechanisms: mechanisms,
            solved_evaluation_tasks: solved,
            evaluation_tasks: 3,
            candidate_executions_to_acquire_next: search,
            cpu_nanos_to_acquire_next: search,
            examples_to_acquire_next: examples,
            interventions_to_acquire_next: examples,
            maximum_composition_depth: mechanisms,
            retained_prior_tasks: mechanisms,
            prior_tasks: mechanisms,
            negative_transfers: 0,
        }
    }

    #[test]
    fn developmental_gate_accepts_growth_with_falling_cost() {
        assert_eq!(
            DevelopmentalGate
                .verify(&[point(1, 1, 100, 4), point(2, 2, 30, 2), point(3, 3, 1, 0),]),
            Ok(())
        );
    }

    #[test]
    fn developmental_gate_rejects_a_solve_rate_only_claim() {
        let result = DevelopmentalGate.verify(&[
            point(1, 1, 100, 4),
            point(2, 2, 80, 3),
            point(3, 3, 120, 2),
        ]);
        assert_eq!(result, Err(ScalingError::MarginalSearchDidNotFall));
    }
}
