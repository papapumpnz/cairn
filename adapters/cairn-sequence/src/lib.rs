//! Sequence segment relations lowered to generic typed examples.

use cairn_core::{CollectionDemonstration, CollectionProcedureProblem, EntityId, RelationRecord};
use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_theory::{
    PrimitivePairEntity, RawAttributedObservationPrimitive, RawEntityObservationEvidence,
    RawObservationPrimitive, RawRelationObservationTrace, ScalarExpressionExample,
    ScalarExpressionTrace,
};
use cairn_world::{ActionId, LatentEpisode, TransitionObservation, WorldState};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SegmentShift {
    pub endpoints: (i64, i64),
    pub shifts: (i64, i64),
}

/// Frontend sequence segment with orientation left uninterpreted for cognition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceSpanObservation {
    pub id: cairn_core::EntityId,
    pub endpoints: (i64, i64),
    pub value: i64,
}

/// One uninterpreted scalar sequence primitive. Candidate membership relations
/// are lowered separately and remain competing cognition hypotheses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequencePrimitiveObservation {
    pub id: EntityId,
    pub observation: i64,
}

/// One sequence primitive with anonymous scalar attributes and no supplied
/// relation or entity membership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceAttributedPrimitiveObservation {
    pub id: EntityId,
    pub observation: i64,
    pub attributes: Vec<i64>,
}

/// One exact scalar result over anonymous sequence-derived channels. The
/// adapter assigns no algebraic role to any channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceScalarExample {
    pub channels: Vec<i64>,
    pub result: i64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SequenceAdapter;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionSequenceExample {
    pub input: Vec<i64>,
    pub output: Vec<i64>,
}

/// One action-conditioned exact sequence observation. The adapter assigns no
/// semantic meaning to positions or actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceTransition {
    pub before: Vec<i64>,
    pub action: u16,
    pub after: Vec<i64>,
}

impl SequenceAdapter {
    /// Lower exact anonymous scalar evidence to the shared expression learner.
    #[must_use]
    pub fn lower_scalar_expression(
        &self,
        experience: cairn_core::Hash32,
        examples: &[SequenceScalarExample],
    ) -> ScalarExpressionTrace {
        ScalarExpressionTrace {
            experience,
            examples: examples
                .iter()
                .map(|example| ScalarExpressionExample {
                    inputs: example.channels.clone(),
                    output: example.result,
                })
                .collect(),
        }
    }

    /// Lower anonymous primitive attributes without constructing relations or
    /// selecting entity boundaries.
    #[must_use]
    pub fn lower_unrelated_primitives(
        &self,
        episode: cairn_core::Hash32,
        primitives: &[SequenceAttributedPrimitiveObservation],
        outputs: BTreeSet<i64>,
    ) -> RawRelationObservationTrace {
        RawRelationObservationTrace {
            episode,
            primitives: primitives
                .iter()
                .map(|primitive| {
                    (
                        primitive.id.clone(),
                        RawAttributedObservationPrimitive {
                            observation: primitive.observation,
                            attributes: primitive.attributes.clone(),
                        },
                    )
                })
                .collect(),
            outputs,
        }
    }

    /// Lower scalar primitives and unselected binary relations to the common
    /// entity-interpretation evidence contract.
    #[must_use]
    pub fn lower_entity_candidates(
        &self,
        primitives: &[SequencePrimitiveObservation],
        relations: &[RelationRecord],
    ) -> RawEntityObservationEvidence {
        RawEntityObservationEvidence {
            primitives: primitives
                .iter()
                .map(|primitive| {
                    (
                        primitive.id.clone(),
                        RawObservationPrimitive {
                            observation: primitive.observation,
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            relations: relations.to_vec(),
        }
    }

    /// Lower sequence spans to anonymous primitive pairs. Endpoint
    /// normalization and relational meaning remain cognition responsibilities.
    #[must_use]
    pub fn lower_span_observations(
        &self,
        observations: &[SequenceSpanObservation],
    ) -> Vec<PrimitivePairEntity> {
        observations
            .iter()
            .map(|observation| PrimitivePairEntity {
                id: observation.id.clone(),
                value: observation.value,
                observations: vec![observation.endpoints.0, observation.endpoints.1],
            })
            .collect()
    }

    /// Lower an exact sequence trace into the common visible transition
    /// protocol used by world-mechanism learning.
    #[must_use]
    pub fn lower_transition_episode(&self, transitions: &[SequenceTransition]) -> LatentEpisode {
        LatentEpisode {
            transitions: transitions
                .iter()
                .map(|transition| TransitionObservation {
                    before: WorldState(transition.before.clone()),
                    action: ActionId(transition.action),
                    after: WorldState(transition.after.clone()),
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn lower(
        &self,
        bound: i64,
        examples: &[SegmentShift],
        limits: InventionLimits,
    ) -> InventionProblem {
        let names = ["segment_start", "segment_end", "start_shift", "end_shift"];
        InventionProblem {
            definition_name: "adapter_neutral_mechanism".to_owned(),
            parameters: names
                .iter()
                .enumerate()
                .map(|(index, name)| Binder {
                    name: VarName((*name).to_owned()),
                    ty: Type::Int(IntRange {
                        lower: if index < 2 { -bound } else { -10 },
                        upper: if index < 2 { bound } else { 10 },
                    }),
                })
                .collect(),
            return_type: Type::Tuple(vec![
                Type::Int(IntRange {
                    lower: -bound - 10,
                    upper: bound + 10,
                }),
                Type::Int(IntRange {
                    lower: -bound - 10,
                    upper: bound + 10,
                }),
            ]),
            literals: Vec::new(),
            demonstrations: examples
                .iter()
                .map(|example| {
                    let values = [
                        example.endpoints.0,
                        example.endpoints.1,
                        example.shifts.0,
                        example.shifts.1,
                    ];
                    let mut inputs = RuntimeEnvironment::new();
                    for (name, value) in names.iter().zip(values) {
                        inputs.insert(VarName((*name).to_owned()), Value::Int(value));
                    }
                    BindingExample {
                        inputs,
                        output: Value::Tuple(vec![
                            Value::Int(example.endpoints.0 + example.shifts.0),
                            Value::Int(example.endpoints.1 + example.shifts.1),
                        ]),
                    }
                })
                .collect(),
            limits,
        }
    }

    /// Lower unique sequence elements to the generic procedural collection
    /// contract without consulting cognition or learned memory.
    #[must_use]
    pub fn lower_collection(
        &self,
        maximum_cardinality: u32,
        examples: &[CollectionSequenceExample],
        prediction_input: Vec<i64>,
    ) -> CollectionProcedureProblem {
        CollectionProcedureProblem {
            maximum_cardinality,
            demonstrations: examples
                .iter()
                .map(|example| CollectionDemonstration {
                    input: example.input.clone(),
                    output: example.output.iter().copied().collect::<BTreeSet<_>>(),
                })
                .collect(),
            prediction_input,
            maximum_predictions: 2,
        }
    }
}
