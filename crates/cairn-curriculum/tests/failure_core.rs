use cairn_curriculum::{
    DeterminismStatus, FailureCoreExtractor, RedactionStatus, SearchFailure, TwoParameterFamily,
};
use cairn_invent::{BindingExample, InventionError, InventionLimits, InventionProblem};
use cairn_rkl::{Binder, IntRange, RuntimeEnvironment, Type, Value, VarName};

fn example(x: i64, y: i64, output: i64) -> BindingExample {
    let mut inputs = RuntimeEnvironment::new();
    inputs.insert(VarName("private_x".to_owned()), Value::Int(x));
    inputs.insert(VarName("private_y".to_owned()), Value::Int(y));
    BindingExample {
        inputs,
        output: Value::Int(output),
    }
}

#[test]
fn redacted_core_detects_a_multi_parameter_gap_without_content() {
    let integer = Type::Int(IntRange {
        lower: -10,
        upper: 10,
    });
    let problem = InventionProblem {
        definition_name: "private_task_name".to_owned(),
        parameters: vec![
            Binder {
                name: VarName("private_x".to_owned()),
                ty: integer.clone(),
            },
            Binder {
                name: VarName("private_y".to_owned()),
                ty: integer.clone(),
            },
        ],
        return_type: integer,
        literals: vec![Value::Int(0)],
        demonstrations: vec![
            example(0, 0, 0),
            example(0, 1, 1),
            example(1, 0, 1),
            example(1, 1, 2),
        ],
        limits: InventionLimits::default(),
    };
    let core = FailureCoreExtractor
        .extract(
            &problem,
            &InventionError::CandidateCapacity { maximum: 1_000 },
        )
        .expect("redacted core");
    assert_eq!(
        core.failure,
        SearchFailure::CandidateCapacity { maximum: 1_000 }
    );
    assert_eq!(core.varying_parameters, 2);
    assert_eq!(core.single_parameter_explanations, 0);
    assert_eq!(core.minimum_dependency_arity_lower_bound, 2);
    assert!(core
        .exact_two_parameter_families
        .contains(&TwoParameterFamily::Add));
    assert_eq!(core.determinism, DeterminismStatus::Deterministic);
    assert_eq!(core.redaction, RedactionStatus::ContentFree);
}
