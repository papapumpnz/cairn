//! Vector-scene observations lowered to the generic learning contract.

use cairn_core::{CollectionDemonstration, CollectionProcedureProblem};
use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_theory::{
    EntityCorrespondenceError, EntityCorrespondenceExample, EntityCorrespondenceLearner,
    EntityCorrespondenceLimits, LayerLearningExample, LayerLearningLimits, LayerMechanism,
    LayerMechanismError, LayerMechanismLearner, LearnedEntityCorrespondence,
};
use cairn_world::{ActionId, LatentEpisode, TransitionObservation, WorldState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorTranslation {
    pub point: (i64, i64),
    pub displacement: (i64, i64),
}

/// A batch of scalar attributes attached to otherwise frontend-owned vector
/// entities. Cognition sees only the generic collection contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorAttributeBatch {
    pub input_attributes: Vec<i64>,
    pub output_attributes: Vec<i64>,
}

/// One exact action-conditioned point transition. Geometry is lowered to the
/// same two-coordinate visible state used by cognition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorPointTransition {
    pub before: (i64, i64),
    pub action: u16,
    pub after: (i64, i64),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VectorAdapter;

impl VectorAdapter {
    /// Learn a collection-level coordinate correspondence with exact
    /// entity/point construction provenance.
    ///
    /// # Errors
    ///
    /// Returns validation, ambiguity, capacity, or no-mechanism failures.
    pub fn learn_entity_correspondence(
        &self,
        examples: &[EntityCorrespondenceExample],
        limits: EntityCorrespondenceLimits,
    ) -> Result<LearnedEntityCorrespondence, EntityCorrespondenceError> {
        EntityCorrespondenceLearner.learn(examples, limits)
    }

    /// Learn a modality-neutral coordinate-layer construction and renderer from
    /// exact vector-scene examples. The adapter supplies no expression label.
    ///
    /// # Errors
    ///
    /// Returns exact scene, capacity, ambiguity, or unrealizability failures.
    pub fn learn_layer_mechanism(
        &self,
        examples: &[LayerLearningExample],
        limits: LayerLearningLimits,
    ) -> Result<LayerMechanism, LayerMechanismError> {
        LayerMechanismLearner.learn(examples, limits)
    }

    /// Lower a vector point trace into the common visible transition protocol.
    #[must_use]
    pub fn lower_transition_episode(&self, transitions: &[VectorPointTransition]) -> LatentEpisode {
        LatentEpisode {
            transitions: transitions
                .iter()
                .map(|transition| TransitionObservation {
                    before: WorldState(vec![transition.before.0, transition.before.1]),
                    action: ActionId(transition.action),
                    after: WorldState(vec![transition.after.0, transition.after.1]),
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn lower_attribute_collection(
        &self,
        maximum_cardinality: u32,
        examples: &[VectorAttributeBatch],
        prediction_attributes: Vec<i64>,
    ) -> CollectionProcedureProblem {
        CollectionProcedureProblem {
            maximum_cardinality,
            demonstrations: examples
                .iter()
                .map(|example| CollectionDemonstration {
                    input: example.input_attributes.clone(),
                    output: example.output_attributes.iter().copied().collect(),
                })
                .collect(),
            prediction_input: prediction_attributes,
            maximum_predictions: 16,
        }
    }

    #[must_use]
    pub fn lower(
        &self,
        bound: i64,
        examples: &[VectorTranslation],
        limits: InventionLimits,
    ) -> InventionProblem {
        problem(
            ["point_y", "point_x", "translation_y", "translation_x"],
            bound,
            examples
                .iter()
                .map(|example| {
                    (
                        example.point.0,
                        example.point.1,
                        example.displacement.0,
                        example.displacement.1,
                    )
                })
                .collect(),
            limits,
        )
    }
}

fn problem(
    names: [&str; 4],
    bound: i64,
    cases: Vec<(i64, i64, i64, i64)>,
    limits: InventionLimits,
) -> InventionProblem {
    InventionProblem {
        definition_name: "adapter_neutral_mechanism".to_owned(),
        parameters: binders(names, bound),
        return_type: output_type(bound),
        literals: Vec::new(),
        demonstrations: cases
            .into_iter()
            .map(|(first, second, first_delta, second_delta)| {
                binding(names, first, second, first_delta, second_delta)
            })
            .collect(),
        limits,
    }
}

fn binders(names: [&str; 4], bound: i64) -> Vec<Binder> {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| Binder {
            name: VarName((*name).to_owned()),
            ty: Type::Int(IntRange {
                lower: if index < 2 { -bound } else { -10 },
                upper: if index < 2 { bound } else { 10 },
            }),
        })
        .collect()
}

fn output_type(bound: i64) -> Type {
    Type::Tuple(vec![
        Type::Int(IntRange {
            lower: -bound - 10,
            upper: bound + 10,
        }),
        Type::Int(IntRange {
            lower: -bound - 10,
            upper: bound + 10,
        }),
    ])
}

fn binding(
    names: [&str; 4],
    first: i64,
    second: i64,
    first_delta: i64,
    second_delta: i64,
) -> BindingExample {
    let mut inputs = RuntimeEnvironment::new();
    for (name, value) in names.iter().zip([first, second, first_delta, second_delta]) {
        inputs.insert(VarName((*name).to_owned()), Value::Int(value));
    }
    BindingExample {
        inputs,
        output: Value::Tuple(vec![
            Value::Int(first + first_delta),
            Value::Int(second + second_delta),
        ]),
    }
}
