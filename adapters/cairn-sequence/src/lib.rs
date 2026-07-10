//! Sequence segment relations lowered to generic typed examples.

use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SegmentShift {
    pub endpoints: (i64, i64),
    pub shifts: (i64, i64),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SequenceAdapter;

impl SequenceAdapter {
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
}
