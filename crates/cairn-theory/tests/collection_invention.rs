use cairn_core::{CollectionDemonstration, CollectionProcedureProblem, Hash32};
use cairn_invent::InventionLimits;
use cairn_mechanism::{
    CardinalityRange, DerivationCertificate, DomainClass, InventionEvidence, Mechanism, Signature,
    SupportGeometry, VariableId,
};
use cairn_memory::{
    MechanismStore, ModuleStatus, ProceduralMechanismStore, ProceduralStoreError, ReuseEvidence,
};
use cairn_rkl::{
    CheckedTheory, Effect, ExecLimits, Expr, Quantifier, ReferenceTheoryExecutor,
    RuntimeEnvironment, TheoryExecLimits, TheoryExecutionRequest, Type, TypeEnvironment, Value,
    VarName,
};
use cairn_runtime::CognitionRuntime;
use cairn_theory::{CollectionMechanismInventor, CollectionTrace};
use cairn_verifier::{
    CounterfactualReuseMetrics, ProceduralExperimentBaseline, ProceduralPromotionConsolidator,
    PromotionVerifier,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn variable_cardinality_traces_are_lifted_into_a_for_each_mechanism() {
    let learned = CollectionMechanismInventor
        .invent(
            &[
                trace("collection-a", &[-3, 2], &[-5, 5]),
                trace("collection-b", &[-4, -1, 0, 5], &[-7, -1, 1, 11]),
                trace("collection-c", &[-7, -2, 1, 4, 9], &[-13, -3, 3, 9, 19]),
            ],
            32,
            InventionLimits::default(),
        )
        .expect("collection mechanism");
    assert_eq!(learned.observed_cardinalities, BTreeSet::from([2, 4, 5]));
    let rule = &learned.theory.theory().rules[0];
    assert_eq!(rule.binders[0].quantifier, Quantifier::ForEach);
    let held_out = [-11, -6, -1, 0, 3, 8, 15];
    let trace = execute(&learned.theory, &held_out);
    let expected = held_out
        .into_iter()
        .map(|value| Value::Int(value * 2 + 1))
        .collect::<BTreeSet<_>>();
    assert_eq!(trace.rendered.facts, expected);
    assert_eq!(trace.rule_firings.len(), 7);
    eprintln!(
        "COLLECTION_METRICS episodes=3 cardinalities=2,4,5 element_candidates={} element_cost={} held_out_cardinality=7 exact=7 for_each_firings=7 body={:?}",
        learned.element.proposal.candidates_evaluated,
        learned.element.proposal.structural_cost,
        learned.element.proposal.definition.body
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Preserve whole-theory promotion and tamper audit together.
fn quantified_collection_theory_is_canonically_verified_and_persisted() {
    let first = learn_collection(&[
        trace("first-a", &[-3, 2], &[-5, 5]),
        trace("first-b", &[-4, -1, 0, 5], &[-7, -1, 1, 11]),
        trace("first-c", &[-7, -2, 1, 4, 9], &[-13, -3, 3, 9, 19]),
    ]);
    let second = learn_collection(&[
        trace("second-a", &[-8, 6], &[-15, 13]),
        trace("second-b", &[-9, -5, 2], &[-17, -9, 5]),
        trace(
            "second-c",
            &[-10, -6, 3, 7, 12, 20],
            &[-19, -11, 7, 15, 25, 41],
        ),
    ]);
    assert_eq!(first.theory, second.theory);
    let learned_mechanism = mechanism(&first, &second);
    let evidence = InventionEvidence {
        target_absent_from_named_vocabulary: true,
        source_code_changes: 0,
        independent_experiences: 2,
        initial_success_parts_per_million: 0,
        post_experience_success_parts_per_million: 1_000_000,
        uncompressed_cost: u64::from(first.element.proposal.structural_cost) * 2,
        compressed_cost: u64::from(first.element.proposal.structural_cost),
        held_out_verified_cases: 7,
        candidate_executions_without: first.element.proposal.candidates_evaluated,
        candidate_executions_with: 1,
        examples_without: 11,
        examples_with: 2,
        false_acceptances_without: 0,
        false_acceptances_with: 0,
        actual_reuse_count: 1,
        necessary_reuse_count: 1,
    };
    let token = PromotionVerifier
        .verify_procedural(&learned_mechanism, &first.theory, &evidence)
        .expect("canonical procedural token");
    let mut store = ProceduralMechanismStore::new();
    let id = store
        .insert_provisional(
            first.theory.clone(),
            learned_mechanism.support.clone(),
            learned_mechanism.proof.source_proofs.clone(),
            learned_mechanism.proof.verified_cases,
        )
        .expect("provisional procedure");
    assert!(!store.apply_verified_promotion(id, &token));
    store
        .record_verified_reuse(
            id,
            ReuseEvidence {
                experience: Hash32::digest(b"held-out-collection-reuse"),
                independently_verified: true,
                necessary_under_ablation: true,
                candidate_executions_without: first.element.proposal.candidates_evaluated,
                candidate_executions_with: 1,
                examples_without: 11,
                examples_with: 2,
                proof_nodes_without: u64::from(first.element.proposal.structural_cost),
                proof_nodes_with: 1,
            },
        )
        .expect("independent reuse receipt");
    assert!(store.apply_verified_promotion(id, &token));
    assert_eq!(store.len(), 1);
    let stored = store.modules().next().expect("stored theory");
    assert_eq!(stored.id(), id);
    assert_eq!(stored.status(), ModuleStatus::Persistent);
    assert_eq!(stored.reuse_metrics().necessary_invocations, 1);
    let runtime = CognitionRuntime::with_memories(MechanismStore::new(), store);
    assert_eq!(runtime.procedural_memory().len(), 1);
    let held_out = [-11, -6, -1, 0, 3, 8, 15];
    let replay = execute(
        runtime
            .procedural_memory()
            .modules()
            .next()
            .expect("stored theory")
            .theory(),
        &held_out,
    );
    assert_eq!(replay.rule_firings.len(), 7);
    assert_eq!(
        replay.rendered.facts,
        held_out
            .into_iter()
            .map(|value| Value::Int(value * 2 + 1))
            .collect()
    );

    let altered = learn_collection(&[
        trace("altered-a", &[1, 2], &[3, 4]),
        trace("altered-b", &[3, 4, 5], &[5, 6, 7]),
    ]);
    assert_eq!(
        ProceduralMechanismStore::new().insert_verified(
            altered.theory,
            learned_mechanism.support,
            &token,
        ),
        Err(ProceduralStoreError::TheoryHashMismatch)
    );
    eprintln!(
        "COLLECTION_PROMOTION_METRICS sources=2 whole_theory_persistent=1 candidates={}->1 examples=11->2 held_out=7 exact=7 tamper_rejected=1",
        first.element.proposal.candidates_evaluated
    );
}

#[test]
fn independent_replays_automatically_consolidate_into_procedural_promotion() {
    let first = learn_collection(&[
        trace("auto-first-a", &[-3, 2], &[-5, 5]),
        trace("auto-first-b", &[-4, -1, 0, 5], &[-7, -1, 1, 11]),
    ]);
    let second = learn_collection(&[
        trace("auto-second-a", &[-8, 6], &[-15, 13]),
        trace("auto-second-b", &[-9, -5, 2], &[-17, -9, 5]),
    ]);
    let mechanism = mechanism(&first, &second);
    let problems = [
        collection_problem(&[-6, -1, 4], &[-11, -1, 9], &[-10, 0, 7]),
        collection_problem(&[-11, 3], &[-21, 7], &[-7, -2, 5, 12]),
    ];
    let expected = [
        BTreeSet::from([-19, 1, 15]),
        BTreeSet::from([-13, -3, 11, 25]),
    ];
    let receipts = problems
        .iter()
        .zip(&expected)
        .enumerate()
        .map(|(index, (problem, expected))| {
            PromotionVerifier
                .verify_collection_reuse(
                    &first.theory,
                    problem,
                    expected,
                    Hash32::digest(format!("automatic-reuse-{index}").as_bytes()),
                    CounterfactualReuseMetrics {
                        blank_solved: false,
                        candidate_executions_without: 100,
                        candidate_executions_with: 1,
                        examples_without: 4,
                        examples_with: 1,
                        proof_nodes_without: 20,
                        proof_nodes_with: 3,
                    },
                )
                .expect("independent receipt")
        })
        .collect::<Vec<_>>();
    let token = ProceduralPromotionConsolidator
        .consolidate(
            &mechanism,
            &first.theory,
            ProceduralExperimentBaseline {
                target_absent_from_named_vocabulary: true,
                source_code_changes: 0,
                initial_success_parts_per_million: 0,
                uncompressed_cost: 40,
                compressed_cost: 10,
                false_acceptances_without: 0,
                false_acceptances_with: 0,
            },
            &receipts,
        )
        .expect("automatic promotion certificate");
    let mut store = ProceduralMechanismStore::new();
    let id = store
        .insert_provisional(
            first.theory,
            mechanism.support,
            mechanism.proof.source_proofs,
            mechanism.proof.verified_cases,
        )
        .expect("provisional");
    for receipt in &receipts {
        store
            .record_verifier_receipt(id, receipt)
            .expect("receipt import");
    }
    assert!(store.apply_verified_promotion(id, &token));
    assert_eq!(
        store.modules().next().expect("module").status(),
        ModuleStatus::Persistent
    );
}

fn execute(theory: &CheckedTheory, values: &[i64]) -> cairn_rkl::TheoryExecutionTrace {
    let collection_type = Type::List {
        element: Box::new(Type::Int(cairn_rkl::IntRange {
            lower: -i64::from(u16::MAX),
            upper: i64::from(u16::MAX),
        })),
        maximum: 32,
    };
    let collection = Value::Set {
        element_type: match &collection_type {
            Type::List { element, .. } => (**element).clone(),
            _ => unreachable!(),
        },
        maximum: 32,
        values: values.iter().copied().map(Value::Int).collect(),
    };
    let mut types = TypeEnvironment::new();
    types.insert(VarName("collection".to_owned()), collection_type);
    let mut environment = RuntimeEnvironment::new();
    environment.insert(VarName("collection".to_owned()), collection);
    ReferenceTheoryExecutor
        .execute(
            theory,
            &TheoryExecutionRequest {
                types,
                values: environment,
                initial_facts: BTreeSet::new(),
                initial_state: BTreeMap::new(),
            },
            TheoryExecLimits {
                expression: ExecLimits::default(),
                maximum_bindings: 100,
                maximum_rule_firings: 100,
            },
        )
        .expect("collection execution")
}

fn learn_collection(traces: &[CollectionTrace]) -> cairn_theory::LearnedCollectionMechanism {
    CollectionMechanismInventor
        .invent(traces, 32, InventionLimits::default())
        .expect("collection family")
}

fn mechanism(
    first: &cairn_theory::LearnedCollectionMechanism,
    second: &cairn_theory::LearnedCollectionMechanism,
) -> Mechanism {
    let theory = first.theory.theory();
    Mechanism {
        signature: Signature {
            inputs: theory.inputs.iter().map(|input| input.ty.clone()).collect(),
            output: theory.inputs[0].ty.clone(),
        },
        variables: theory.inputs.clone(),
        entity_definitions: theory.definitions.clone(),
        preconditions: Expr::Bool(true),
        transition_rules: theory.rules.clone(),
        invariants: vec![Expr::Bool(true)],
        termination: Expr::Bool(true),
        effects: theory
            .rules
            .iter()
            .flat_map(|rule| rule.effects.iter().cloned())
            .collect::<Vec<Effect>>(),
        support: SupportGeometry {
            observed_domains: BTreeSet::from([
                DomainClass("vector-entity-batch".to_owned()),
                DomainClass("interactive-entity-set".to_owned()),
            ]),
            observed_cardinalities: first
                .observed_cardinalities
                .union(&second.observed_cardinalities)
                .map(|size| CardinalityRange {
                    minimum: u32::try_from(*size).expect("small suite"),
                    maximum: u32::try_from(*size).expect("small suite"),
                })
                .collect(),
            varied_variables: BTreeSet::from([VariableId("collection".to_owned())]),
            invariant_variables: BTreeSet::new(),
            unknown_variables: BTreeSet::new(),
            validated_compositions: BTreeSet::new(),
            failed_conditions: BTreeSet::new(),
        },
        proof: DerivationCertificate {
            source_proofs: BTreeSet::from([
                first.element.proposal.semantic_hash,
                second.element.proposal.semantic_hash,
            ]),
            verifier_semantics: "checked-whole-theory-canonical-v1".to_owned(),
            verified_cases: 7,
        },
    }
}

fn trace(episode: &str, inputs: &[i64], outputs: &[i64]) -> CollectionTrace {
    CollectionTrace {
        episode: Hash32::digest(episode.as_bytes()),
        inputs: inputs.to_vec(),
        outputs: outputs.to_vec(),
    }
}

fn collection_problem(
    input: &[i64],
    output: &[i64],
    prediction_input: &[i64],
) -> CollectionProcedureProblem {
    CollectionProcedureProblem {
        maximum_cardinality: 32,
        demonstrations: vec![CollectionDemonstration {
            input: input.to_vec(),
            output: output.iter().copied().collect(),
        }],
        prediction_input: prediction_input.to_vec(),
        maximum_predictions: 1,
    }
}
