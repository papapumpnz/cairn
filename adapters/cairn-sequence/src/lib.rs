//! Sequence segment relations lowered to generic typed examples.

use cairn_core::{CollectionDemonstration, CollectionProcedureProblem};
use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};
use cairn_world::{ActionId, LatentEpisode, TransitionObservation, WorldState};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SegmentShift {
    pub endpoints: (i64, i64),
    pub shifts: (i64, i64),
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
