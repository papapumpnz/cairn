//! Proof-gated reusable mechanism memory.
//!
//! Retrieval produces rebound RKL proposals only. Every proposal is checked
//! and replayed against the complete current problem by `cairn-invent` before
//! it can influence a prediction.

use cairn_core::{CanonicalEncode, Hash32};
use cairn_invent::{
    DefinitionInventor, DefinitionProposal, InventionError, InventionOutcome, InventionProblem,
};
use cairn_rkl::{Definition, Expr, Type, VarName};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MEMORY_SEMANTICS_VERSION: &str = "1.0.0";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum VariationDimension {
    ParameterNames,
    NumericRanges,
    CategoricalValues,
    Counts,
    Positions,
    Layouts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportGeometry {
    pub independent_tasks: u32,
    pub held_out_rebindings: u32,
    pub varied_dimensions: BTreeSet<VariationDimension>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleStatus {
    Active,
    Quarantined,
    Deprecated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanismModule {
    id: Hash32,
    definition: Definition,
    support: SupportGeometry,
    lineage: Vec<Hash32>,
    status: ModuleStatus,
    structural_cost: u16,
}

impl MechanismModule {
    #[must_use]
    pub const fn id(&self) -> Hash32 {
        self.id
    }

    #[must_use]
    pub const fn definition(&self) -> &Definition {
        &self.definition
    }

    #[must_use]
    pub const fn support(&self) -> &SupportGeometry {
        &self.support
    }

    #[must_use]
    pub fn lineage(&self) -> &[Hash32] {
        &self.lineage
    }

    #[must_use]
    pub const fn status(&self) -> ModuleStatus {
        self.status
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionError {
    InsufficientTasks { actual: u32, required: u32 },
    MissingHeldOutRebinding,
    MissingVariation,
    EmptyEvidence,
    IncompatibleDefinitions,
    Canonicalization,
}

impl fmt::Display for PromotionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PromotionError {}

#[derive(Clone, Copy, Debug, Default)]
pub struct PromotionRunner;

impl PromotionRunner {
    /// Promote only independently admitted, structurally equivalent proposals
    /// with cross-task and held-out support.
    ///
    /// # Errors
    ///
    /// Returns an exact missing-evidence or incompatibility reason.
    pub fn promote(
        &self,
        proposals: &[DefinitionProposal],
        support: SupportGeometry,
    ) -> Result<MechanismModule, PromotionError> {
        if proposals.is_empty() {
            return Err(PromotionError::EmptyEvidence);
        }
        if support.independent_tasks < 2 {
            return Err(PromotionError::InsufficientTasks {
                actual: support.independent_tasks,
                required: 2,
            });
        }
        if support.held_out_rebindings == 0 {
            return Err(PromotionError::MissingHeldOutRebinding);
        }
        if support.varied_dimensions.is_empty() {
            return Err(PromotionError::MissingVariation);
        }
        let representative = &proposals[0];
        if proposals.iter().skip(1).any(|proposal| {
            !equivalent_definition_shape(&representative.definition, &proposal.definition)
        }) {
            return Err(PromotionError::IncompatibleDefinitions);
        }
        let mut lineage = proposals
            .iter()
            .map(|proposal| proposal.semantic_hash)
            .collect::<Vec<_>>();
        lineage.sort_unstable();
        lineage.dedup();
        let definition_bytes = representative
            .definition
            .encode_canonical()
            .map_err(|_| PromotionError::Canonicalization)?;
        let mut identity = definition_bytes;
        identity.extend_from_slice(MEMORY_SEMANTICS_VERSION.as_bytes());
        for hash in &lineage {
            identity.extend_from_slice(hash.as_bytes());
        }
        identity.extend_from_slice(&support.independent_tasks.to_be_bytes());
        identity.extend_from_slice(&support.held_out_rebindings.to_be_bytes());
        for dimension in &support.varied_dimensions {
            identity.push(variation_code(*dimension));
        }
        Ok(MechanismModule {
            id: Hash32::digest(&identity),
            definition: representative.definition.clone(),
            support,
            lineage,
            status: ModuleStatus::Active,
            structural_cost: representative.structural_cost,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct MechanismStore {
    modules: BTreeMap<Hash32, MechanismModule>,
}

impl MechanismStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, module: MechanismModule) -> Option<MechanismModule> {
        self.modules.insert(module.id, module)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn active_modules(&self) -> impl Iterator<Item = &MechanismModule> {
        self.modules
            .values()
            .filter(|module| module.status == ModuleStatus::Active)
    }

    pub fn set_status(&mut self, id: Hash32, status: ModuleStatus) -> bool {
        let Some(module) = self.modules.get_mut(&id) else {
            return false;
        };
        module.status = status;
        true
    }

    fn proposals(&self, problem: &InventionProblem) -> Vec<ReboundProposal> {
        let mut proposals = Vec::new();
        for module in self.active_modules() {
            if !type_shape_compatible(&module.definition.return_type, &problem.return_type)
                || module.definition.parameters.len() != problem.parameters.len()
            {
                continue;
            }
            let mut used = vec![false; problem.parameters.len()];
            let mut mapping = Vec::with_capacity(problem.parameters.len());
            bindings(module, problem, 0, &mut used, &mut mapping, &mut proposals);
        }
        proposals
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SolveOrigin {
    MemoryProposal,
    FreshSearch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemorySolveMetrics {
    pub memory_proposals_evaluated: u64,
    pub rejected_memory_proposals: u64,
    pub fresh_candidates_evaluated: u64,
    pub total_search_work: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemorySolveOutcome {
    pub invention: InventionOutcome,
    pub origin: SolveOrigin,
    pub metrics: MemorySolveMetrics,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryAssistedInventor;

impl MemoryAssistedInventor {
    /// Try deterministic rebound modules, verifying each against the current
    /// task. If none passes, run unrestricted fresh invention.
    ///
    /// # Errors
    ///
    /// Returns the fresh inventor's exact terminal error after memory proposals
    /// have failed; retrieval never vetoes fallback search.
    pub fn invent(
        &self,
        problem: &InventionProblem,
        store: &MechanismStore,
    ) -> Result<MemorySolveOutcome, InventionError> {
        let inventor = DefinitionInventor;
        let proposals = store.proposals(problem);
        let mut attempted = 0_u64;
        for proposal in proposals {
            attempted += 1;
            if let Ok(invention) = inventor.admit_definition(
                problem,
                proposal.definition,
                proposal.structural_cost,
                attempted,
            ) {
                return Ok(MemorySolveOutcome {
                    invention,
                    origin: SolveOrigin::MemoryProposal,
                    metrics: MemorySolveMetrics {
                        memory_proposals_evaluated: attempted,
                        rejected_memory_proposals: attempted - 1,
                        fresh_candidates_evaluated: 0,
                        total_search_work: attempted,
                    },
                });
            }
        }
        let invention = inventor.invent(problem)?;
        let fresh = invention.proposal.candidates_evaluated;
        Ok(MemorySolveOutcome {
            invention,
            origin: SolveOrigin::FreshSearch,
            metrics: MemorySolveMetrics {
                memory_proposals_evaluated: attempted,
                rejected_memory_proposals: attempted,
                fresh_candidates_evaluated: fresh,
                total_search_work: attempted.saturating_add(fresh),
            },
        })
    }
}

struct ReboundProposal {
    definition: Definition,
    structural_cost: u16,
}

fn bindings(
    module: &MechanismModule,
    problem: &InventionProblem,
    old_index: usize,
    used: &mut [bool],
    mapping: &mut Vec<usize>,
    proposals: &mut Vec<ReboundProposal>,
) {
    if old_index == module.definition.parameters.len() {
        let substitutions = module
            .definition
            .parameters
            .iter()
            .zip(mapping.iter())
            .map(|(old, new)| (old.name.clone(), problem.parameters[*new].name.clone()))
            .collect::<BTreeMap<_, _>>();
        proposals.push(ReboundProposal {
            definition: Definition {
                name: problem.definition_name.clone(),
                parameters: problem.parameters.clone(),
                return_type: problem.return_type.clone(),
                body: substitute(
                    &module.definition.body,
                    &substitutions,
                    &mut BTreeSet::new(),
                ),
                recursion: None,
                proof_annotations: Vec::new(),
            },
            structural_cost: module.structural_cost,
        });
        return;
    }
    let old = &module.definition.parameters[old_index];
    for (new_index, new) in problem.parameters.iter().enumerate() {
        if used[new_index] || !type_shape_compatible(&old.ty, &new.ty) {
            continue;
        }
        used[new_index] = true;
        mapping.push(new_index);
        bindings(module, problem, old_index + 1, used, mapping, proposals);
        mapping.pop();
        used[new_index] = false;
    }
}

fn substitute(
    expression: &Expr,
    substitutions: &BTreeMap<VarName, VarName>,
    bound: &mut BTreeSet<VarName>,
) -> Expr {
    match expression {
        Expr::Bool(value) => Expr::Bool(*value),
        Expr::Int(value) => Expr::Int(*value),
        Expr::Atom(value) => Expr::Atom(*value),
        Expr::Coord2 { row, column } => Expr::Coord2 {
            row: *row,
            column: *column,
        },
        Expr::Var(name) => Expr::Var(if bound.contains(name) {
            name.clone()
        } else {
            substitutions.get(name).unwrap_or(name).clone()
        }),
        Expr::Tuple(values) => Expr::Tuple(
            values
                .iter()
                .map(|value| substitute(value, substitutions, bound))
                .collect(),
        ),
        Expr::Let {
            binder,
            value,
            body,
        } => {
            let value = substitute(value, substitutions, bound);
            bound.insert(binder.clone());
            let body = substitute(body, substitutions, bound);
            bound.remove(binder);
            Expr::Let {
                binder: binder.clone(),
                value: Box::new(value),
                body: Box::new(body),
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => Expr::If {
            condition: Box::new(substitute(condition, substitutions, bound)),
            then_branch: Box::new(substitute(then_branch, substitutions, bound)),
            else_branch: Box::new(substitute(else_branch, substitutions, bound)),
        },
        Expr::Unary { op, argument } => Expr::Unary {
            op: *op,
            argument: Box::new(substitute(argument, substitutions, bound)),
        },
        Expr::Binary { op, left, right } => Expr::Binary {
            op: *op,
            left: Box::new(substitute(left, substitutions, bound)),
            right: Box::new(substitute(right, substitutions, bound)),
        },
        Expr::EmptySet { element, maximum } => Expr::EmptySet {
            element: element.clone(),
            maximum: *maximum,
        },
        Expr::SingletonSet { value, maximum } => Expr::SingletonSet {
            value: Box::new(substitute(value, substitutions, bound)),
            maximum: *maximum,
        },
        Expr::Count(value) => Expr::Count(Box::new(substitute(value, substitutions, bound))),
        Expr::Call {
            function,
            arguments,
        } => Expr::Call {
            function: function.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute(argument, substitutions, bound))
                .collect(),
        },
        Expr::BoundedFixpoint {
            binder,
            initial,
            step,
            maximum_steps,
            progress_measure,
        } => {
            let initial = substitute(initial, substitutions, bound);
            bound.insert(binder.clone());
            let step = substitute(step, substitutions, bound);
            let progress_measure = substitute(progress_measure, substitutions, bound);
            bound.remove(binder);
            Expr::BoundedFixpoint {
                binder: binder.clone(),
                initial: Box::new(initial),
                step: Box::new(step),
                maximum_steps: *maximum_steps,
                progress_measure: Box::new(progress_measure),
            }
        }
    }
}

fn equivalent_definition_shape(left: &Definition, right: &Definition) -> bool {
    if left.parameters.len() != right.parameters.len()
        || !type_shape_compatible(&left.return_type, &right.return_type)
        || left
            .parameters
            .iter()
            .zip(&right.parameters)
            .any(|(left, right)| !type_shape_compatible(&left.ty, &right.ty))
    {
        return false;
    }
    normalized_body(left) == normalized_body(right)
}

fn normalized_body(definition: &Definition) -> Expr {
    let substitutions = definition
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            (
                parameter.name.clone(),
                VarName(format!("parameter_{index}")),
            )
        })
        .collect::<BTreeMap<_, _>>();
    substitute(&definition.body, &substitutions, &mut BTreeSet::new())
}

fn type_shape_compatible(left: &Type, right: &Type) -> bool {
    match (left, right) {
        (Type::Int(_), Type::Int(_))
        | (Type::CellSet { .. }, Type::CellSet { .. })
        | (Type::EntitySet { .. }, Type::EntitySet { .. }) => true,
        (Type::List { element: left, .. }, Type::List { element: right, .. }) => {
            type_shape_compatible(left, right)
        }
        (Type::Tuple(left), Type::Tuple(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| type_shape_compatible(left, right))
        }
        _ => left == right,
    }
}

const fn variation_code(dimension: VariationDimension) -> u8 {
    match dimension {
        VariationDimension::ParameterNames => 0,
        VariationDimension::NumericRanges => 1,
        VariationDimension::CategoricalValues => 2,
        VariationDimension::Counts => 3,
        VariationDimension::Positions => 4,
        VariationDimension::Layouts => 5,
    }
}
