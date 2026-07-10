//! Conditional learned skills and exact effect-chain planning.

use cairn_core::Hash32;
use cairn_rkl::Binder;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fluent(pub u16);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Condition {
    pub required: BTreeSet<Fluent>,
    pub forbidden: BTreeSet<Fluent>,
}

impl Condition {
    #[must_use]
    pub fn holds(&self, state: &BTreeSet<Fluent>) -> bool {
        self.required.is_subset(state) && self.forbidden.is_disjoint(state)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EffectModel {
    pub add: BTreeSet<Fluent>,
    pub remove: BTreeSet<Fluent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlProgram {
    Primitive(Hash32),
    Sequence(Vec<Hash32>),
    Conditional {
        condition: Condition,
        when_true: Box<Self>,
        when_false: Box<Self>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRule {
    pub condition: Condition,
    pub program: ControlProgram,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceModel {
    pub maximum_steps: u32,
    pub maximum_cost: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Skill {
    id: Hash32,
    pub parameters: Vec<Binder>,
    pub initiation_condition: Condition,
    pub goal_condition: Condition,
    pub policy: ControlProgram,
    pub predicted_effects: EffectModel,
    pub invariants: Vec<Condition>,
    pub failure_conditions: Vec<Condition>,
    pub recovery_branches: Vec<RecoveryRule>,
    pub resource_model: ResourceModel,
    pub proof: BTreeSet<Hash32>,
}

impl Skill {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub fn apply(&self, state: &BTreeSet<Fluent>) -> Option<BTreeSet<Fluent>> {
        if !self.initiation_condition.holds(state)
            || self
                .failure_conditions
                .iter()
                .any(|condition| condition.holds(state))
        {
            return None;
        }
        let mut next = state.clone();
        for fluent in &self.predicted_effects.remove {
            next.remove(fluent);
        }
        next.extend(self.predicted_effects.add.iter().copied());
        self.invariants
            .iter()
            .all(|condition| condition.holds(&next))
            .then_some(next)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuccessfulTrace {
    pub episode: Hash32,
    pub before: BTreeSet<Fluent>,
    pub primitive_steps: Vec<Hash32>,
    pub after: BTreeSet<Fluent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillInductionError {
    InsufficientTraces,
    DuplicateEpisode,
    EmptyProgram,
    InconsistentProgram,
    NoStableEffect,
    InvalidResourceBound,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SkillInducer;

impl SkillInducer {
    /// Infer stable initiation facts and effects while dropping varying surface
    /// facts from independently successful traces.
    ///
    /// # Errors
    ///
    /// Requires two independent traces with one shared program and effect.
    pub fn induce(
        &self,
        traces: &[SuccessfulTrace],
        resource_model: ResourceModel,
    ) -> Result<Skill, SkillInductionError> {
        if traces.len() < 2 {
            return Err(SkillInductionError::InsufficientTraces);
        }
        if resource_model.maximum_steps == 0 || resource_model.maximum_cost == 0 {
            return Err(SkillInductionError::InvalidResourceBound);
        }
        let episodes = traces
            .iter()
            .map(|trace| trace.episode)
            .collect::<BTreeSet<_>>();
        if episodes.len() != traces.len() {
            return Err(SkillInductionError::DuplicateEpisode);
        }
        let program = &traces[0].primitive_steps;
        if program.is_empty() {
            return Err(SkillInductionError::EmptyProgram);
        }
        if traces.iter().any(|trace| trace.primitive_steps != *program) {
            return Err(SkillInductionError::InconsistentProgram);
        }
        let initiation = traces
            .iter()
            .skip(1)
            .fold(traces[0].before.clone(), |stable, trace| {
                stable.intersection(&trace.before).copied().collect()
            });
        let additions = traces
            .iter()
            .map(|trace| {
                trace
                    .after
                    .difference(&trace.before)
                    .copied()
                    .collect::<BTreeSet<_>>()
            })
            .collect::<Vec<_>>();
        let removals = traces
            .iter()
            .map(|trace| {
                trace
                    .before
                    .difference(&trace.after)
                    .copied()
                    .collect::<BTreeSet<_>>()
            })
            .collect::<Vec<_>>();
        if additions.iter().any(|effect| *effect != additions[0])
            || removals.iter().any(|effect| *effect != removals[0])
            || additions[0].is_empty() && removals[0].is_empty()
        {
            return Err(SkillInductionError::NoStableEffect);
        }
        let mut identity = Vec::new();
        for step in program {
            identity.extend_from_slice(step.as_bytes());
        }
        for fluent in &initiation {
            identity.extend_from_slice(&fluent.0.to_be_bytes());
        }
        for fluent in &additions[0] {
            identity.extend_from_slice(&fluent.0.to_be_bytes());
        }
        for fluent in &removals[0] {
            identity.extend_from_slice(&fluent.0.to_be_bytes());
        }
        let policy = if program.len() == 1 {
            ControlProgram::Primitive(program[0])
        } else {
            ControlProgram::Sequence(program.clone())
        };
        Ok(Skill {
            id: Hash32::digest(&identity),
            parameters: Vec::new(),
            initiation_condition: Condition {
                required: initiation,
                forbidden: BTreeSet::new(),
            },
            goal_condition: Condition {
                required: additions[0].clone(),
                forbidden: BTreeSet::new(),
            },
            policy,
            predicted_effects: EffectModel {
                add: additions[0].clone(),
                remove: removals[0].clone(),
            },
            invariants: Vec::new(),
            failure_conditions: Vec::new(),
            recovery_branches: Vec::new(),
            resource_model,
            proof: episodes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillInvocation {
    pub skill: Hash32,
    pub state_before: BTreeSet<Fluent>,
    pub state_after: BTreeSet<Fluent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillPlan {
    pub invocations: Vec<SkillInvocation>,
    pub final_state: BTreeSet<Fluent>,
    pub expanded_states: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillPlanError {
    GoalAlreadySatisfied,
    NoSkills,
    DuplicateSkill,
    NoPlan,
    ExpansionLimit,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SkillPlanner;

impl SkillPlanner {
    /// Find a shortest exact effect-chain over learned conditional skills.
    ///
    /// # Errors
    ///
    /// Rejects malformed libraries, already achieved goals, unreachable goals,
    /// and explicit expansion caps.
    pub fn plan(
        &self,
        initial: &BTreeSet<Fluent>,
        goal: &Condition,
        skills: &[Skill],
        maximum_expansions: u64,
    ) -> Result<SkillPlan, SkillPlanError> {
        if goal.holds(initial) {
            return Err(SkillPlanError::GoalAlreadySatisfied);
        }
        if skills.is_empty() {
            return Err(SkillPlanError::NoSkills);
        }
        let by_id = skills
            .iter()
            .map(|skill| (skill.id(), skill))
            .collect::<BTreeMap<_, _>>();
        if by_id.len() != skills.len() {
            return Err(SkillPlanError::DuplicateSkill);
        }
        let mut queue = VecDeque::from([(initial.clone(), Vec::new())]);
        let mut visited = BTreeSet::from([initial.clone()]);
        let mut expanded = 0_u64;
        while let Some((state, path)) = queue.pop_front() {
            expanded = expanded.saturating_add(1);
            if expanded > maximum_expansions {
                return Err(SkillPlanError::ExpansionLimit);
            }
            for skill in by_id.values() {
                let Some(next) = skill.apply(&state) else {
                    continue;
                };
                if next == state || !visited.insert(next.clone()) {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(SkillInvocation {
                    skill: skill.id(),
                    state_before: state.clone(),
                    state_after: next.clone(),
                });
                if goal.holds(&next) {
                    return Ok(SkillPlan {
                        invocations: next_path,
                        final_state: next,
                        expanded_states: expanded,
                    });
                }
                queue.push_back((next, next_path));
            }
        }
        Err(SkillPlanError::NoPlan)
    }
}
