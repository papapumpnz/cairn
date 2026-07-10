//! Gridworld transitions lowered without action meanings entering cognition.

use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentTransition {
    pub before: (i64, i64),
    pub observed_effect: (i64, i64),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GridworldAdapter;

impl GridworldAdapter {
    #[must_use]
    pub fn lower(
        &self,
        bound: i64,
        examples: &[AgentTransition],
        limits: InventionLimits,
    ) -> InventionProblem {
        let names = ["agent_row", "agent_column", "effect_row", "effect_column"];
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
                        example.before.0,
                        example.before.1,
                        example.observed_effect.0,
                        example.observed_effect.1,
                    ];
                    let mut inputs = RuntimeEnvironment::new();
                    for (name, value) in names.iter().zip(values) {
                        inputs.insert(VarName((*name).to_owned()), Value::Int(value));
                    }
                    BindingExample {
                        inputs,
                        output: Value::Tuple(vec![
                            Value::Int(example.before.0 + example.observed_effect.0),
                            Value::Int(example.before.1 + example.observed_effect.1),
                        ]),
                    }
                })
                .collect(),
            limits,
        }
    }
}
