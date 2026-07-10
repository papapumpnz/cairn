//! Vector-scene observations lowered to the generic learning contract.

use cairn_invent::{BindingExample, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorTranslation {
    pub point: (i64, i64),
    pub displacement: (i64, i64),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VectorAdapter;

impl VectorAdapter {
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
