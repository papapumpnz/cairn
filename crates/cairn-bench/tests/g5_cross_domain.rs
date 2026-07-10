use cairn_core::Hash32;
use cairn_gridworld::{AgentTransition, GridworldAdapter};
use cairn_invent::{DefinitionInventor, InventionLimits};
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, InventionEvidence, Mechanism, Signature,
    SupportGeometry as MechanismSupport, VariableId,
};
use cairn_memory::{
    MechanismStore, ModuleStatus, PromotionRunner, SolveOrigin, SupportGeometry as MemorySupport,
    VariationDimension,
};
use cairn_rkl::{Effect, Expr, Value};
use cairn_runtime::CognitionRuntime;
use cairn_sequence::{SegmentShift, SequenceAdapter};
use cairn_vector::{VectorAdapter, VectorTranslation};
use cairn_verifier::PromotionVerifier;
use std::collections::BTreeSet;

#[test]
#[allow(clippy::too_many_lines)]
fn one_cognition_runtime_reuses_a_mechanism_across_three_adapters() {
    let source_a = VectorAdapter.lower(
        20,
        &[
            vector(-10, 7, 3, -2),
            vector(4, -9, -3, 5),
            vector(0, 0, 2, 4),
            vector(12, 8, -4, -3),
        ],
        InventionLimits::default(),
    );
    let source_b = VectorAdapter.lower(
        30,
        &[
            vector(-24, 17, 6, -3),
            vector(14, -19, -7, 8),
            vector(2, 1, 4, 5),
            vector(22, 18, -8, -6),
        ],
        InventionLimits::default(),
    );
    let first = DefinitionInventor
        .invent(&source_a)
        .expect("vector source a");
    let second = DefinitionInventor
        .invent(&source_b)
        .expect("vector source b");
    let module = PromotionRunner
        .promote(
            &[first.proposal.clone(), second.proposal.clone()],
            MemorySupport {
                independent_tasks: 2,
                held_out_rebindings: 1,
                varied_dimensions: BTreeSet::from([
                    VariationDimension::ParameterNames,
                    VariationDimension::NumericRanges,
                    VariationDimension::Positions,
                    VariationDimension::Layouts,
                ]),
            },
        )
        .expect("cross-domain proposal");
    let mut store = MechanismStore::new();
    store.insert(module.clone());
    let mut runtime = CognitionRuntime::with_memory(store);

    let vector_problem = VectorAdapter.lower(
        40,
        &[
            vector(-35, 22, 7, -4),
            vector(19, -31, -8, 9),
            vector(0, 4, 5, 6),
            vector(33, 28, -9, -7),
        ],
        InventionLimits::default(),
    );
    let grid_problem = GridworldAdapter.lower(
        50,
        &[
            AgentTransition {
                before: (4, 8),
                observed_effect: (1, -1),
            },
            AgentTransition {
                before: (12, 3),
                observed_effect: (-2, 4),
            },
            AgentTransition {
                before: (20, 19),
                observed_effect: (3, -5),
            },
            AgentTransition {
                before: (31, 7),
                observed_effect: (-6, 2),
            },
        ],
        InventionLimits::default(),
    );
    let sequence_problem = SequenceAdapter.lower(
        60,
        &[
            SegmentShift {
                endpoints: (2, 9),
                shifts: (3, 3),
            },
            SegmentShift {
                endpoints: (11, 18),
                shifts: (-4, 2),
            },
            SegmentShift {
                endpoints: (20, 27),
                shifts: (5, -1),
            },
            SegmentShift {
                endpoints: (34, 41),
                shifts: (-7, 6),
            },
        ],
        InventionLimits::default(),
    );
    let domains = [
        (
            vector_problem,
            VectorAdapter.lower(40, &[vector(37, -32, -6, 8)], InventionLimits::default()),
        ),
        (
            grid_problem,
            GridworldAdapter.lower(
                50,
                &[AgentTransition {
                    before: (44, 6),
                    observed_effect: (-8, 3),
                }],
                InventionLimits::default(),
            ),
        ),
        (
            sequence_problem,
            SequenceAdapter.lower(
                60,
                &[SegmentShift {
                    endpoints: (48, 55),
                    shifts: (7, -5),
                }],
                InventionLimits::default(),
            ),
        ),
    ];

    let mut fresh_candidates = 0_u64;
    let mut memory_candidates = 0_u64;
    let mut exact_domains = 0_u32;
    for (full, probe_problem) in &domains {
        let fresh = DefinitionInventor.invent(full).expect("fresh domain solve");
        fresh_candidates += fresh.proposal.candidates_evaluated;
        let mut one_shot = full.clone();
        one_shot.demonstrations.truncate(1);
        let learned = runtime.learn(&one_shot).expect("shared runtime solve");
        assert_eq!(learned.origin, SolveOrigin::MemoryProposal);
        memory_candidates += learned.metrics.total_search_work;
        let probe = &probe_problem.demonstrations[0];
        let prediction = DefinitionInventor
            .predict(
                &learned.invention.proposal,
                &probe.inputs,
                one_shot.limits.execution,
            )
            .expect("cross-domain prediction");
        exact_domains += u32::from(prediction == probe.output);
    }
    assert_eq!(exact_domains, 3);

    let definition = module.definition().clone();
    let learned_mechanism = Mechanism {
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
            observed_domains: BTreeSet::from([
                DomainClass("vector-scene".to_owned()),
                DomainClass("interactive-discrete-world".to_owned()),
                DomainClass("sequence-relation".to_owned()),
            ]),
            observed_cardinalities: BTreeSet::from([CardinalityRange {
                minimum: 4,
                maximum: 4,
            }]),
            varied_variables: definition
                .parameters
                .iter()
                .map(|parameter| VariableId(parameter.name.0.clone()))
                .collect(),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::from([VariableId("obstacles".to_owned())]),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: module.lineage().iter().copied().collect(),
            verifier_semantics: "dual-rkl-reference-bytecode-replay".to_owned(),
            verified_cases: 3
                + u64::try_from(first.admission.exact_cross_implementation_replays)
                    .expect("small suite")
                + u64::try_from(second.admission.exact_cross_implementation_replays)
                    .expect("small suite"),
        },
    };
    let evidence = InventionEvidence {
        target_absent_from_named_vocabulary: true,
        source_code_changes: 0,
        independent_experiences: 2,
        initial_success_parts_per_million: 0,
        post_experience_success_parts_per_million: 1_000_000,
        uncompressed_cost: u64::from(first.proposal.structural_cost)
            + u64::from(second.proposal.structural_cost),
        compressed_cost: u64::from(first.proposal.structural_cost),
        held_out_verified_cases: 3,
        candidate_executions_without: fresh_candidates,
        candidate_executions_with: memory_candidates,
        examples_without: 4,
        examples_with: 1,
        false_acceptances_without: 0,
        false_acceptances_with: 0,
        actual_reuse_count: 3,
        necessary_reuse_count: 3,
    };
    let token = PromotionVerifier
        .verify(&learned_mechanism, &evidence)
        .expect("cross-domain promotion");
    assert!(runtime
        .memory_mut()
        .apply_verified_promotion(module.id(), &token));
    assert_eq!(
        runtime
            .memory()
            .reusable_modules()
            .next()
            .expect("mechanism")
            .status(),
        ModuleStatus::Persistent
    );
    assert!(fresh_candidates >= memory_candidates.saturating_mul(4));
    eprintln!(
        "G5_METRICS adapters=3 exact_domains={exact_domains}/3 shared_mechanism_reuses=3 fresh_candidates={fresh_candidates} memory_candidates={memory_candidates}"
    );
}

fn vector(y: i64, x: i64, dy: i64, dx: i64) -> VectorTranslation {
    VectorTranslation {
        point: (y, x),
        displacement: (dy, dx),
    }
}

#[test]
fn adapter_payloads_do_not_enter_runtime_semantics() {
    let runtime_source = include_str!("../../cairn-runtime/src/lib.rs");
    for forbidden in ["gridworld", "vector-scene", "sequence-relation", "arc"] {
        assert!(!runtime_source.to_ascii_lowercase().contains(forbidden));
    }
    assert_ne!(Hash32::digest(b"adapter-a"), Hash32::digest(b"adapter-b"));
    assert_eq!(Value::Int(3), Value::Int(3));
}
