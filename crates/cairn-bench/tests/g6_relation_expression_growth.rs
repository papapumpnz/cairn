use cairn_core::{EntityId, Hash32};
use cairn_curriculum::{
    DeterminismStatus, FailureDrivenObjectiveGenerator, LearningFailureCore, RedactionStatus,
    SearchFailure, ValueClass, FAILURE_CORE_SEMANTICS_VERSION,
};
use cairn_invent::InventionLimits;
use cairn_memory::RelationExpressionStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::{CognitionError, CognitionRuntime, RelationExpressionMemoryRuntimeError};
use cairn_sequence::{SequenceAdapter, SequenceAttributedPrimitiveObservation};
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationMechanism, RawRelationObservationInventor,
    RawRelationObservationTrace, RelationExpressionError, RelationExpressionInventor,
    RelationExpressionLimits, RelationKeyExpression, RelationKeyOperator,
    RelationObservationLimits,
};
use cairn_vector::{VectorAdapter, VectorAttributedPrimitiveObservation};
use cairn_verifier::{
    RawRelationObservationPromotionConsolidator, RawRelationObservationVerifier,
    RelationExpressionLanguageConsolidator, RelationExpressionLanguageError,
    RelationExpressionLanguageLimits, RelationExpressionPromotionConsolidator,
    RelationExpressionVerifier, RelationExpressionVerifyError,
    VerifiedRawRelationObservationPromotion,
};
use std::collections::BTreeSet;

type PrimitiveValues = Vec<(EntityId, i64, Vec<i64>)>;
type EpisodeValues = (PrimitiveValues, BTreeSet<i64>);

#[test]
#[allow(clippy::too_many_lines)] // Complete two-library -> expression invention -> transfer gate.
fn promoted_operators_compose_into_an_omitted_nested_relation_program() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();

    let add_cohorts = [
        [
            additive_episode("expression-add-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
            additive_episode("expression-add-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            additive_episode("expression-add-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
            additive_episode("expression-add-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let minimum_cohorts = [
        [
            minimum_episode("expression-min-sequence-a", &[2, 3, 4], 100, [0, 1, 2]),
            minimum_episode("expression-min-table-a", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            minimum_episode("expression-min-vector-b", &[3, 4, 5], -500, [1, 2, 0]),
            minimum_episode("expression-min-world-b", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let (add, add_promotion) = learn_and_promote(
        &add_cohorts,
        relation_limits,
        entity_limits,
        observation_limits,
        predicate_limits,
    );
    let (minimum, minimum_promotion) = learn_and_promote(
        &minimum_cohorts,
        relation_limits,
        entity_limits,
        observation_limits,
        predicate_limits,
    );
    assert_eq!(add.operator, RelationKeyOperator::Add);
    assert_eq!(minimum.operator, RelationKeyOperator::Minimum);

    let generated = FailureDrivenObjectiveGenerator
        .generate(&arity_failure(3), &BTreeSet::new())
        .expect("content-free composition objective");
    let request = generated.relation_language_growth_request();
    let sources = [(&add, &add_promotion), (&minimum, &minimum_promotion)];
    let language = RelationExpressionLanguageConsolidator
        .consolidate(
            &sources,
            &request,
            RelationExpressionLanguageLimits::default(),
        )
        .expect("verified expression language");
    assert_eq!(language.operators().len(), 2);
    assert_eq!(language.parameter_arity(), 3);
    assert_eq!(language.maximum_binary_nodes(), 2);
    assert_eq!(language.source_mechanisms().len(), 2);
    assert_eq!(
        RelationExpressionLanguageConsolidator.consolidate(
            &[(&add, &add_promotion)],
            &request,
            RelationExpressionLanguageLimits::default(),
        ),
        Err(RelationExpressionLanguageError::InsufficientOperators)
    );
    let over_cap = FailureDrivenObjectiveGenerator
        .generate(&arity_failure(9), &BTreeSet::new())
        .expect("over-cap composition objective")
        .relation_language_growth_request();
    assert_eq!(
        RelationExpressionLanguageConsolidator.consolidate(
            &sources,
            &over_cap,
            RelationExpressionLanguageLimits::default(),
        ),
        Err(RelationExpressionLanguageError::InvalidRequestedArity)
    );

    let resource_limits = RelationExpressionLimits {
        maximum_candidates: 1_024,
        maximum_relation_evaluations: 100_000_000,
        maximum_absolute_attribute: 1_000_000_000,
        ..RelationExpressionLimits::default()
    };
    assert_eq!(
        RelationExpressionInventor.invent(
            &[
                nested_episode(
                    Frontend::Sequence,
                    "expression-source-negative-a",
                    &[2, 3, 4],
                    100,
                    [0, 1, 2, 3],
                ),
                nested_episode(
                    Frontend::Sequence,
                    "expression-source-negative-b",
                    &[2, 3, 4, 5],
                    300,
                    [3, 0, 2, 1],
                ),
            ],
            &resource_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        ),
        Err(RelationExpressionError::EmptyLanguage)
    );
    let expression_limits = language
        .apply(resource_limits)
        .expect("apply verified expression language");
    let expression_cohorts = [
        [
            nested_episode(
                Frontend::Sequence,
                "expression-source-sequence-a",
                &[2, 3, 4],
                100,
                [0, 1, 2, 3],
            ),
            nested_episode(
                Frontend::Sequence,
                "expression-source-sequence-b",
                &[2, 3, 4, 5],
                300,
                [3, 0, 2, 1],
            ),
        ],
        [
            nested_episode(
                Frontend::Sequence,
                "expression-corroborating-sequence-a",
                &[3, 4, 5],
                -700,
                [2, 1, 3, 0],
            ),
            nested_episode(
                Frontend::Sequence,
                "expression-corroborating-sequence-b",
                &[2, 3, 5, 6],
                900,
                [1, 2, 0, 3],
            ),
        ],
    ];
    let learned = expression_cohorts
        .iter()
        .map(|cohort| {
            RelationExpressionInventor
                .invent(
                    cohort,
                    &expression_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("nested expression invention")
        })
        .collect::<Vec<_>>();
    let expected = RelationKeyExpression::Binary {
        operator: RelationKeyOperator::Minimum,
        left: Box::new(RelationKeyExpression::Binary {
            operator: RelationKeyOperator::Add,
            left: Box::new(RelationKeyExpression::Parameter(0)),
            right: Box::new(RelationKeyExpression::Parameter(1)),
        }),
        right: Box::new(RelationKeyExpression::Parameter(2)),
    };
    assert!(learned
        .iter()
        .all(|mechanism| mechanism.expression == expected));
    assert!(learned
        .iter()
        .all(|mechanism| mechanism.expression_candidates == 4));
    assert_eq!(learned[0].mechanism_hash(), learned[1].mechanism_hash());
    let expression_receipts = learned
        .iter()
        .zip(&expression_cohorts)
        .map(|(mechanism, cohort)| {
            RelationExpressionVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    &expression_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("independent expression reuse")
        })
        .collect::<Vec<_>>();
    let expression_promotion = RelationExpressionPromotionConsolidator
        .consolidate(&expression_receipts)
        .expect("expression promotion");
    assert_eq!(expression_promotion.verified_reuses(), 2);
    assert_eq!(expression_promotion.expressions_saved(), 6);
    assert_eq!(
        expression_promotion.mechanism(),
        learned[0].mechanism_hash().expect("hash")
    );
    let mut expression_store = RelationExpressionStore::new();
    let expression_id = expression_store
        .insert_provisional(learned[0].source_independent())
        .expect("provisional expression memory");
    for receipt in &expression_receipts {
        expression_store
            .record_verified_reuse(expression_id, receipt.clone())
            .expect("expression reuse ledger");
    }
    assert!(expression_store.apply_verified_promotion(expression_id, &expression_promotion));
    let expression_runtime = CognitionRuntime::with_relation_expression_memory(expression_store);

    let held_out = [
        nested_episode(
            Frontend::Vector,
            "expression-heldout-vector-a",
            &[2, 3, 4, 5],
            -500,
            [1, 3, 0, 2],
        ),
        nested_episode(
            Frontend::Vector,
            "expression-heldout-vector-b",
            &[2, 3, 4, 5, 6],
            700,
            [2, 0, 3, 1],
        ),
    ];
    let rebound = RelationExpressionInventor
        .invent(
            &held_out,
            &expression_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            InventionLimits::default(),
        )
        .expect("cross-adapter expression rebinding");
    assert_eq!(learned[0].mechanism_hash(), rebound.mechanism_hash());
    let receipt = RelationExpressionVerifier
        .verify(
            &rebound,
            &held_out,
            &expression_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        )
        .expect("independent expression replay");
    assert_eq!(receipt.replayed_groups(), 9);

    let mut proposal_attempts = 0_u64;
    for trace in &held_out {
        let expected = trace.outputs.clone();
        let mut answer_hidden = trace.clone();
        answer_hidden.outputs.clear();
        assert_eq!(
            CognitionRuntime::new().propose_relation_expressions_from_memory(
                &answer_hidden,
                &expression_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
            ),
            Err(CognitionError::RelationExpressionMemory(
                RelationExpressionMemoryRuntimeError::NoVerifiedProposal
            ))
        );
        let outcome = expression_runtime
            .propose_relation_expressions_from_memory(
                &answer_hidden,
                &expression_limits,
                entity_limits,
                observation_limits,
                predicate_limits,
            )
            .expect("answer-hidden expression proposals");
        assert_eq!(outcome.persistent_mechanisms_considered, 1);
        proposal_attempts = proposal_attempts
            .checked_add(u64::try_from(outcome.proposals.len()).expect("proposal count"))
            .expect("proposal work");
        assert!(outcome
            .proposals
            .iter()
            .any(|proposal| proposal.output == expected));
    }
    assert_eq!(proposal_attempts, 2);

    let mut corrupted = rebound.clone();
    corrupted.expression = RelationKeyExpression::Binary {
        operator: RelationKeyOperator::Minimum,
        left: Box::new(RelationKeyExpression::Binary {
            operator: RelationKeyOperator::Minimum,
            left: Box::new(RelationKeyExpression::Parameter(0)),
            right: Box::new(RelationKeyExpression::Parameter(1)),
        }),
        right: Box::new(RelationKeyExpression::Parameter(2)),
    };
    assert_eq!(
        RelationExpressionVerifier.verify(
            &corrupted,
            &held_out,
            &expression_limits,
            entity_limits,
            observation_limits,
            predicate_limits,
            ExecLimits::default(),
        ),
        Err(RelationExpressionVerifyError::RelationDefinition)
    );
    eprintln!(
        "G6_RELATION_EXPRESSION source_operators=add2,minimum2 expression_candidates=4->1 promotion_receipts=2 expressions_saved=6 selected=minimum(add(p0,p1),p2) source=sequence target=vector examples=4->0 answer_hidden_proposals=2 heldout_groups=9 exact=9 blank_exact=0 verifier_ablation_accepts=0"
    );
}

fn learn_and_promote(
    cohorts: &[[RawRelationObservationTrace; 2]; 2],
    relation_limits: RelationObservationLimits,
    entity_limits: EntityObservationLimits,
    observation_limits: ObservationPredicateLimits,
    predicate_limits: PredicateGroupingLimits,
) -> (
    RawRelationMechanism,
    VerifiedRawRelationObservationPromotion,
) {
    let learned = cohorts
        .iter()
        .map(|cohort| {
            RawRelationObservationInventor
                .invent(
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    InventionLimits::default(),
                )
                .expect("base operator invention")
        })
        .collect::<Vec<_>>();
    let receipts = learned
        .iter()
        .zip(cohorts)
        .map(|(mechanism, cohort)| {
            RawRelationObservationVerifier
                .verify_reuse(
                    mechanism,
                    cohort,
                    relation_limits,
                    entity_limits,
                    observation_limits,
                    predicate_limits,
                    ExecLimits::default(),
                )
                .expect("base operator reuse")
        })
        .collect::<Vec<_>>();
    (
        learned[0]
            .source_independent()
            .expect("source-independent base operator"),
        RawRelationObservationPromotionConsolidator
            .consolidate(&receipts)
            .expect("base operator promotion"),
    )
}

fn additive_episode(
    domain: &str,
    cardinalities: &[usize],
    offset: i64,
    permutation: [usize; 3],
) -> RawRelationObservationTrace {
    raw_episode(
        domain,
        cardinalities,
        offset,
        3,
        |component, member, key, count| {
            let first = i64::try_from(component * 100 + member).expect("first");
            permute3(
                [
                    first,
                    key - first,
                    i64::try_from((component + member) % count).expect("distractor"),
                ],
                permutation,
            )
        },
    )
}

fn minimum_episode(
    domain: &str,
    cardinalities: &[usize],
    offset: i64,
    permutation: [usize; 3],
) -> RawRelationObservationTrace {
    raw_episode(
        domain,
        cardinalities,
        offset,
        3,
        |component, member, key, count| {
            let step = i64::try_from(count + component + 1).expect("step") * 100;
            let logical = match member {
                0 => [key, key + step, key + step * 7],
                1 => [key + step * 2, key, key + step * 11],
                _ => [key, key + step * 3, key + step * 13],
            };
            permute3(logical, permutation)
        },
    )
}

fn permute3(logical: [i64; 3], permutation: [usize; 3]) -> Vec<i64> {
    let mut attributes = vec![0_i64; 3];
    for (source, destination) in permutation.into_iter().enumerate() {
        attributes[destination] = logical[source];
    }
    attributes
}

fn raw_episode(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    attribute_arity: usize,
    attributes: impl Fn(usize, usize, i64, usize) -> Vec<i64>,
) -> RawRelationObservationTrace {
    let (primitives, outputs) = episode_values(
        domain,
        cardinalities,
        signature_offset,
        attribute_arity,
        attributes,
    );
    RawRelationObservationTrace {
        episode: Hash32::digest(domain.as_bytes()),
        primitives: primitives
            .into_iter()
            .map(|(id, observation, attributes)| {
                (
                    id,
                    RawAttributedObservationPrimitive {
                        observation,
                        attributes,
                    },
                )
            })
            .collect(),
        outputs,
    }
}

#[derive(Clone, Copy)]
enum Frontend {
    Sequence,
    Vector,
}

fn nested_episode(
    frontend: Frontend,
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    permutation: [usize; 4],
) -> RawRelationObservationTrace {
    let (primitives, outputs) = episode_values(
        domain,
        cardinalities,
        signature_offset,
        4,
        |component, member, key, count| {
            let coordinate = i64::try_from(component * 17 + member + 3).expect("coordinate");
            let step = i64::try_from(count + component + 1).expect("step") * 100;
            let (left, right, control) = match member {
                0 => (coordinate, key - coordinate, key + step),
                1 => (coordinate, key + step * 2 - coordinate, key),
                _ => (coordinate, key - coordinate, key + step * 3),
            };
            let logical = [
                left,
                right,
                control,
                key + step * 20 + i64::try_from(member).expect("member"),
            ];
            let mut attributes = vec![0_i64; 4];
            for (source, destination) in permutation.into_iter().enumerate() {
                attributes[destination] = logical[source];
            }
            attributes
        },
    );
    let episode = Hash32::digest(domain.as_bytes());
    match frontend {
        Frontend::Sequence => SequenceAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| SequenceAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
        Frontend::Vector => VectorAdapter.lower_unrelated_primitives(
            episode,
            &primitives
                .iter()
                .map(
                    |(id, observation, attributes)| VectorAttributedPrimitiveObservation {
                        id: id.clone(),
                        observation: *observation,
                        attributes: attributes.clone(),
                    },
                )
                .collect::<Vec<_>>(),
            outputs,
        ),
    }
}

fn episode_values(
    domain: &str,
    cardinalities: &[usize],
    signature_offset: i64,
    attribute_arity: usize,
    attributes: impl Fn(usize, usize, i64, usize) -> Vec<i64>,
) -> EpisodeValues {
    let component_count = cardinalities.iter().sum::<usize>();
    let mut primitives = Vec::new();
    let mut outputs = BTreeSet::new();
    let mut component = 0_usize;
    for (group, cardinality) in cardinalities.iter().copied().enumerate() {
        let signature = signature_offset + i64::try_from(group).expect("group") * 10_000;
        outputs.insert(i64::try_from(cardinality - 1).expect("cardinality"));
        for _ in 0..cardinality {
            let first = signature_offset * 3 + i64::try_from(component).expect("component") * 37;
            let second = signature_offset * -2 + i64::try_from(component).expect("component") * 19;
            let key = signature_offset * 20_000
                + 1_000_000
                + i64::try_from(component).expect("component key") * 100_000;
            for (member, observation) in [first, second, signature - first - second]
                .into_iter()
                .enumerate()
            {
                let values = attributes(component, member, key, component_count);
                assert_eq!(values.len(), attribute_arity);
                primitives.push((
                    EntityId(Hash32::digest(
                        format!("{domain}:{component}:{member}").as_bytes(),
                    )),
                    observation,
                    values,
                ));
            }
            component += 1;
        }
    }
    (primitives, outputs)
}

fn arity_failure(minimum_dependency_arity_lower_bound: usize) -> LearningFailureCore {
    LearningFailureCore {
        semantics_version: FAILURE_CORE_SEMANTICS_VERSION.to_owned(),
        failure: SearchFailure::NoDefinition { maximum_cost: 2 },
        parameter_types: vec![ValueClass::Entity, ValueClass::Relation],
        return_type: ValueClass::Relation,
        demonstrations: 4,
        varying_parameters: 3,
        distinct_output_classes: 3,
        constant_explanation_possible: false,
        direct_projection_candidates: 0,
        single_parameter_explanations: 0,
        exact_two_parameter_families: BTreeSet::new(),
        exact_three_parameter_families: BTreeSet::new(),
        determinism: DeterminismStatus::Deterministic,
        minimum_dependency_arity_lower_bound,
        redaction: RedactionStatus::ContentFree,
    }
}
