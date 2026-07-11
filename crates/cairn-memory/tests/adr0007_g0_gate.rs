use cairn_consolidator::{
    ConsolidationLimits, ConsolidationRuntime, ProofDatum, ProofGraph, ProofNode, ProofValueType,
};
use cairn_invent::{
    BindingExample, DefinitionInventor, InventionLimits, InventionOutcome, InventionProblem,
};
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, InventionEvidence, Mechanism, Signature,
    SupportGeometry as MechanismSupport, VariableId,
};
use cairn_memory::{
    MechanismCompiler, MechanismStore, MemoryAssistedInventor, ModuleStatus, PromotionError,
    PromotionRunner, SolveOrigin, SupportGeometry as MemorySupport, VariationDimension,
};
use cairn_rkl::{
    BinaryOp, Binder, Effect, ExecLimits, Expr, IntRange, RuntimeEnvironment, Type, UnaryOp, Value,
    VarName,
};
use cairn_verifier::{CompressionVerifier, PromotionVerifier};
use std::collections::BTreeSet;

fn limits(maximum_cost: u16, maximum_candidates: u64) -> InventionLimits {
    InventionLimits {
        maximum_cost,
        maximum_candidates,
        maximum_expression_nodes: 1_000,
        execution: ExecLimits::default(),
    }
}

fn example(name: &str, reading: i64) -> BindingExample {
    let mut inputs = RuntimeEnvironment::new();
    inputs.insert(VarName(name.to_owned()), Value::Int(reading));
    BindingExample {
        inputs,
        output: Value::Int(reading.abs()),
    }
}

fn problem(
    name: &str,
    bound: i64,
    readings: &[i64],
    maximum_cost: u16,
    maximum_candidates: u64,
) -> InventionProblem {
    InventionProblem {
        definition_name: "episode_local_definition".to_owned(),
        parameters: vec![Binder {
            name: VarName(name.to_owned()),
            ty: Type::Int(IntRange {
                lower: -bound,
                upper: bound,
            }),
        }],
        return_type: Type::Int(IntRange {
            lower: 0,
            upper: bound,
        }),
        literals: vec![Value::Int(0)],
        demonstrations: readings.iter().map(|value| example(name, *value)).collect(),
        limits: limits(maximum_cost, maximum_candidates),
    }
}

fn proof_datum(ty: ProofValueType, value: &[u8]) -> ProofDatum {
    ProofDatum::digest(ty, value)
}

fn expression_proof(expression: &Expr, nodes: &mut Vec<ProofNode>) -> u32 {
    let (operation, output_type, inputs, arguments) = match expression {
        Expr::Bool(value) => (
            "literal:boolean".to_owned(),
            ProofValueType::Boolean,
            vec![],
            vec![proof_datum(ProofValueType::Boolean, &[u8::from(*value)])],
        ),
        Expr::Int(value) => (
            "literal:integer".to_owned(),
            ProofValueType::Integer,
            vec![],
            vec![proof_datum(ProofValueType::Integer, &value.to_be_bytes())],
        ),
        Expr::Var(name) => (
            "bind-variable".to_owned(),
            ProofValueType::Integer,
            vec![],
            vec![proof_datum(ProofValueType::Atom, name.0.as_bytes())],
        ),
        Expr::Unary { op, argument } => {
            let input = expression_proof(argument, nodes);
            let output_type = if *op == UnaryOp::Not {
                ProofValueType::Boolean
            } else {
                ProofValueType::Integer
            };
            (format!("unary:{op:?}"), output_type, vec![input], vec![])
        }
        Expr::Binary { op, left, right } => {
            let left = expression_proof(left, nodes);
            let right = expression_proof(right, nodes);
            let output_type = if matches!(
                op,
                BinaryOp::And | BinaryOp::Or | BinaryOp::Equal | BinaryOp::LessThan
            ) {
                ProofValueType::Boolean
            } else {
                ProofValueType::Integer
            };
            (
                format!("binary:{op:?}"),
                output_type,
                vec![left, right],
                vec![],
            )
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition = expression_proof(condition, nodes);
            let then_branch = expression_proof(then_branch, nodes);
            let else_branch = expression_proof(else_branch, nodes);
            (
                "if".to_owned(),
                ProofValueType::Integer,
                vec![condition, then_branch, else_branch],
                vec![],
            )
        }
        _ => (
            "bounded-rkl-node".to_owned(),
            ProofValueType::Mechanism,
            vec![],
            vec![],
        ),
    };
    let id = u32::try_from(nodes.len()).expect("small proof graph");
    nodes.push(ProofNode {
        operation,
        output_type,
        inputs,
        arguments,
    });
    id
}

fn proof_graph(outcome: &InventionOutcome) -> ProofGraph {
    let mut nodes = Vec::new();
    let output = expression_proof(&outcome.proposal.definition.body, &mut nodes);
    ProofGraph {
        episode: outcome.proposal.semantic_hash,
        nodes,
        output,
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn hidden_law_is_invented_promoted_reused_and_reduces_future_search() {
    let initial_store = MechanismStore::new();
    let held_out_specs = [
        ("a", 18, -17),
        ("b", 20, -19),
        ("c", 22, -21),
        ("d", 24, -23),
        ("e", 26, -25),
        ("f", 28, -27),
        ("g", 30, -29),
        ("h", 32, -31),
        ("i", 34, -33),
        ("j", 36, -35),
    ];

    let initial_successes = held_out_specs
        .iter()
        .filter(|(name, bound, reading)| {
            let underpowered = problem(name, *bound, &[*reading, 2], 1, 1);
            MemoryAssistedInventor
                .invent(&underpowered, &initial_store)
                .is_ok()
        })
        .count();
    assert_eq!(initial_successes, 0);

    let first = DefinitionInventor
        .invent(&problem(
            "reading",
            10,
            &[-9, -4, -1, 0, 2, 7],
            8,
            1_000_000,
        ))
        .expect("first independent invention");
    let second = DefinitionInventor
        .invent(&problem(
            "signal",
            16,
            &[-15, -6, -2, 1, 5, 13],
            8,
            1_000_000,
        ))
        .expect("second independent invention");
    let mut consolidator = ConsolidationRuntime::new(ConsolidationLimits {
        maximum_episodes: 8,
        maximum_nodes_per_episode: 64,
        minimum_independent_episodes: 2,
    });
    let proof_graphs = [proof_graph(&first), proof_graph(&second)];
    consolidator
        .submit(proof_graphs[0].clone())
        .expect("first proof graph");
    consolidator
        .submit(proof_graphs[1].clone())
        .expect("second proof graph");
    let report = consolidator.consolidate().expect("offline consolidation");
    assert_eq!(report.metrics.proposals_created, 1);
    let compression = &report.proposals[0];
    let verified_compression = CompressionVerifier
        .verify(compression, &proof_graphs)
        .expect("independent compression replay");
    assert!(compression.compressed_nodes() < compression.uncompressed_nodes());
    eprintln!(
        "CONSOLIDATION_METRICS episodes={} cohorts={} proposals={} uncompressed_nodes={} compressed_nodes={} verified_replays={}",
        report.metrics.episodes_compared,
        report.metrics.structural_cohorts,
        report.metrics.proposals_created,
        compression.uncompressed_nodes(),
        compression.compressed_nodes(),
        compression.verified_replays()
    );
    assert_eq!(
        PromotionRunner
            .promote_compressed(
                std::slice::from_ref(&first.proposal),
                MemorySupport {
                    independent_tasks: 2,
                    held_out_rebindings: 1,
                    varied_dimensions: BTreeSet::from([VariationDimension::ParameterNames]),
                },
                compression,
                &verified_compression,
            )
            .expect_err("partial lineage rejected"),
        PromotionError::CompressionLineageMismatch
    );
    let compiled = MechanismCompiler
        .compile(
            &[first.proposal.clone(), second.proposal.clone()],
            compression,
        )
        .expect("compiled invented mechanism");
    assert_eq!(compiled.exact_source_replays, 2);
    assert!(compiled
        .proposal
        .definition
        .name
        .starts_with("invented_mechanism_"));
    assert_ne!(
        compiled.proposal.semantic_hash,
        first.proposal.semantic_hash
    );
    assert_ne!(
        compiled.proposal.semantic_hash,
        second.proposal.semantic_hash
    );
    assert_eq!(
        compiled.proposal.definition.parameters[0].ty,
        Type::Int(IntRange {
            lower: -16,
            upper: 16,
        })
    );
    assert_eq!(
        compiled.proposal.definition.return_type,
        Type::Int(IntRange {
            lower: 0,
            upper: 16,
        })
    );
    let module = PromotionRunner
        .promote_compressed(
            &[first.proposal.clone(), second.proposal.clone()],
            MemorySupport {
                independent_tasks: 2,
                held_out_rebindings: 1,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::ParameterNames,
                    VariationDimension::NumericRanges,
                ]),
            },
            compression,
            &verified_compression,
        )
        .expect("provisional module");
    assert!(module.definition().name.starts_with("invented_mechanism_"));
    assert_eq!(
        module
            .compression_evidence()
            .expect("compiled compression evidence")
            .compiled_definition,
        compiled.proposal.semantic_hash
    );
    assert_eq!(
        module
            .compression_evidence()
            .expect("compression evidence")
            .verified_replays,
        2
    );
    let mut store = MechanismStore::new();
    store.insert(module.clone());
    assert!(!store.set_status(module.id(), ModuleStatus::Persistent));

    let mut post_successes = 0_u32;
    let mut memory_candidates = 0_u64;
    let mut fresh_candidates = 0_u64;
    let mut verified_predictions = 0_u64;
    for (name, bound, reading) in held_out_specs {
        let full = problem(name, bound, &[reading, -7, -2, 0, 3, 11], 8, 1_000_000);
        let fresh = DefinitionInventor.invent(&full).expect("fresh baseline");
        fresh_candidates += fresh.proposal.candidates_evaluated;

        let one_shot = problem(name, bound, &[reading], 8, 1_000_000);
        let reused = MemoryAssistedInventor
            .invent(&one_shot, &store)
            .expect("memory reuse");
        assert_eq!(reused.origin, SolveOrigin::MemoryProposal);
        memory_candidates += reused.metrics.total_search_work;
        let probe = example(name, bound - 1);
        assert_eq!(
            DefinitionInventor
                .predict(
                    &reused.invention.proposal,
                    &probe.inputs,
                    one_shot.limits.execution,
                )
                .expect("held-out execution"),
            probe.output
        );
        post_successes += 1;
        verified_predictions += 1;
    }
    assert_eq!(post_successes, 10);
    eprintln!(
        "G0_METRICS initial={initial_successes}/10 post={post_successes}/10 fresh_candidates={fresh_candidates} memory_candidates={memory_candidates} verified_predictions={verified_predictions}"
    );
    assert!(fresh_candidates >= memory_candidates.saturating_mul(4));

    let definition = module.definition().clone();
    let variable = VariableId("input".to_owned());
    let mechanism = Mechanism {
        signature: Signature {
            inputs: definition
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            output: definition.return_type.clone(),
        },
        variables: definition.parameters.clone(),
        entity_definitions: vec![definition.clone()],
        preconditions: Expr::Bool(true),
        transition_rules: Vec::new(),
        invariants: vec![Expr::Bool(true)],
        termination: Expr::Bool(true),
        effects: vec![Effect::Emit(Expr::Call {
            function: definition.name.clone(),
            arguments: definition
                .parameters
                .iter()
                .map(|parameter| Expr::Var(parameter.name.clone()))
                .collect(),
        })],
        support: MechanismSupport {
            observed_domains: BTreeSet::from([DomainClass("bounded-scalar".to_owned())]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 1,
                maximum: 1,
            }]),
            varied_variables: BTreeSet::from([variable]),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::from([VariableId("collection-shape".to_owned())]),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: module.lineage().iter().copied().collect(),
            verifier_semantics: "dual-rkl-reference-bytecode-replay".to_owned(),
            verified_cases: verified_predictions
                + u64::try_from(first.admission.exact_cross_implementation_replays)
                    .expect("small evidence")
                + u64::try_from(second.admission.exact_cross_implementation_replays)
                    .expect("small evidence"),
        },
    };
    let evidence = InventionEvidence {
        target_absent_from_named_vocabulary: initial_store.is_empty(),
        source_code_changes: 0,
        independent_experiences: 2,
        initial_success_parts_per_million: 0,
        post_experience_success_parts_per_million: 1_000_000,
        uncompressed_cost: u64::from(first.proposal.structural_cost)
            + u64::from(second.proposal.structural_cost),
        compressed_cost: u64::from(first.proposal.structural_cost),
        held_out_verified_cases: verified_predictions,
        candidate_executions_without: fresh_candidates,
        candidate_executions_with: memory_candidates,
        examples_without: 6,
        examples_with: 1,
        false_acceptances_without: 0,
        false_acceptances_with: 0,
        actual_reuse_count: post_successes,
        necessary_reuse_count: post_successes,
    };
    let verified = PromotionVerifier
        .verify(&mechanism, &evidence)
        .expect("strict verifier token");
    assert!(store.apply_verified_promotion(module.id(), &verified));
    assert!(!store.apply_verified_promotion(module.id(), &verified));
    assert_eq!(
        store
            .reusable_modules()
            .find(|stored| stored.id() == module.id())
            .expect("stored module")
            .status(),
        ModuleStatus::Persistent
    );
}
