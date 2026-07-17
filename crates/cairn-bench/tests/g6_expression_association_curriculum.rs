use cairn_core::{EntityId, Hash32};
use cairn_curriculum::{
    AmbiguityDrivenObjectiveGenerator, DeterminismStatus, FailureDrivenObjectiveGenerator,
    HypothesisAmbiguityCore, HypothesisClass, LearningFailureCore, MechanismTarget,
    RedactionStatus, SearchFailure, ValueClass, FAILURE_CORE_SEMANTICS_VERSION,
    HYPOTHESIS_AMBIGUITY_SEMANTICS_VERSION,
};
use cairn_invent::InventionLimits;
use cairn_memory::ScalarExpressionStore;
use cairn_rkl::ExecLimits;
use cairn_runtime::CognitionRuntime;
use cairn_sequence::{SequenceAdapter, SequenceScalarExample};
use cairn_theory::{
    EntityObservationLimits, ObservationPredicateLimits, PredicateGroupingLimits,
    RawAttributedObservationPrimitive, RawRelationMechanism, RawRelationObservationInventor,
    RawRelationObservationTrace, RelationExpressionLimits, RelationExpressionShapePolicy,
    RelationKeyExpression, RelationKeyOperator, RelationObservationLimits, ScalarExpressionError,
    ScalarExpressionInventor, ScalarExpressionTrace,
};
use cairn_vector::{VectorAdapter, VectorScalarConstraintExample};
use cairn_verifier::{
    RawRelationObservationPromotionConsolidator, RawRelationObservationVerifier,
    RelationExpressionLanguageConsolidator, RelationExpressionLanguageLimits,
    RelationExpressionShapeExtensionConsolidator, RelationExpressionShapeExtensionError,
    RelationExpressionShapeExtensionLimits, ScalarExpressionPromotionConsolidator,
    ScalarExpressionVerifier, VerifiedRawRelationObservationPromotion,
};
use std::collections::BTreeSet;

type PrimitiveValues = Vec<(EntityId, i64, Vec<i64>)>;
type EpisodeValues = (PrimitiveValues, BTreeSet<i64>);

#[test]
#[allow(clippy::too_many_lines)]
fn ambiguity_drives_contrasts_that_identify_a_non_left_associated_expression() {
    let relation_limits = RelationObservationLimits::default();
    let entity_limits = EntityObservationLimits::default();
    let observation_limits = ObservationPredicateLimits::default();
    let predicate_limits = PredicateGroupingLimits::default();
    let subtract_cohorts = [
        [
            subtract_episode("association-subtract-a", &[2, 3, 4], 100, [0, 1, 2]),
            subtract_episode("association-subtract-b", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            subtract_episode("association-subtract-c", &[3, 4, 5], -500, [1, 2, 0]),
            subtract_episode("association-subtract-d", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let minimum_cohorts = [
        [
            minimum_episode("association-min-a", &[2, 3, 4], 100, [0, 1, 2]),
            minimum_episode("association-min-b", &[2, 3, 4, 5], 300, [2, 0, 1]),
        ],
        [
            minimum_episode("association-min-c", &[3, 4, 5], -500, [1, 2, 0]),
            minimum_episode("association-min-d", &[2, 3, 5, 6], 700, [2, 1, 0]),
        ],
    ];
    let (subtract, subtract_promotion) = learn_and_promote(
        &subtract_cohorts,
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
    let generated = FailureDrivenObjectiveGenerator
        .generate(&arity_failure(), &BTreeSet::new())
        .expect("composition objective");
    let language = RelationExpressionLanguageConsolidator
        .consolidate(
            &[
                (&subtract, &subtract_promotion),
                (&minimum, &minimum_promotion),
            ],
            &generated.relation_language_growth_request(),
            RelationExpressionLanguageLimits::default(),
        )
        .expect("base expression language");
    let resources = RelationExpressionLimits {
        maximum_candidates: 4_096,
        maximum_relation_evaluations: 200_000_000,
        maximum_absolute_attribute: 1_000_000_000,
        ..RelationExpressionLimits::default()
    };
    let left_limits = language.apply(resources).expect("left language");

    let support = BTreeSet::from([
        MechanismTarget("verified-subtraction".to_owned()),
        MechanismTarget("verified-minimum".to_owned()),
    ]);
    let discrimination = AmbiguityDrivenObjectiveGenerator
        .generate(
            &HypothesisAmbiguityCore {
                semantics_version: HYPOTHESIS_AMBIGUITY_SEMANTICS_VERSION.to_owned(),
                hypothesis_class: HypothesisClass::ExpressionAssociation,
                parameter_arity: 3,
                surviving_hypotheses: 2,
                redaction: RedactionStatus::ContentFree,
            },
            &support,
        )
        .expect("content-free discrimination objective");
    let shape_extension = RelationExpressionShapeExtensionConsolidator
        .consolidate(
            &language,
            &discrimination.request(),
            RelationExpressionShapeExtensionLimits::default(),
        )
        .expect("verified shape extension");
    let full_limits = shape_extension
        .apply(&language, left_limits.clone())
        .expect("full-tree language");
    assert_eq!(
        full_limits.shape_policy,
        RelationExpressionShapePolicy::FullBinaryTrees
    );
    let wrong_arity = AmbiguityDrivenObjectiveGenerator
        .generate(
            &HypothesisAmbiguityCore {
                semantics_version: HYPOTHESIS_AMBIGUITY_SEMANTICS_VERSION.to_owned(),
                hypothesis_class: HypothesisClass::ExpressionAssociation,
                parameter_arity: 4,
                surviving_hypotheses: 2,
                redaction: RedactionStatus::ContentFree,
            },
            &support,
        )
        .expect("wrong-arity objective");
    assert_eq!(
        RelationExpressionShapeExtensionConsolidator.consolidate(
            &language,
            &wrong_arity.request(),
            RelationExpressionShapeExtensionLimits::default(),
        ),
        Err(RelationExpressionShapeExtensionError::InvalidRequest)
    );

    let contrast_cohorts = [
        [
            scalar_trace(
                ScalarFrontend::Sequence,
                "association-scalar-a",
                [0, 1, 2, 3],
                1,
                0,
            ),
            scalar_trace(
                ScalarFrontend::Sequence,
                "association-scalar-b",
                [3, 0, 2, 1],
                2,
                100,
            ),
        ],
        [
            scalar_trace(
                ScalarFrontend::Vector,
                "association-scalar-c",
                [2, 1, 3, 0],
                3,
                -200,
            ),
            scalar_trace(
                ScalarFrontend::Vector,
                "association-scalar-d",
                [1, 2, 0, 3],
                4,
                300,
            ),
        ],
    ];
    let left_result = ScalarExpressionInventor.invent(&contrast_cohorts[0], &left_limits);
    assert!(
        matches!(left_result, Err(ScalarExpressionError::NoExactTheory)),
        "left_result={left_result:?}"
    );
    let learned = contrast_cohorts
        .iter()
        .map(|cohort| {
            ScalarExpressionInventor
                .invent(cohort, &full_limits)
                .expect("association-discriminated scalar expression")
        })
        .collect::<Vec<_>>();
    let expected = RelationKeyExpression::Binary {
        operator: RelationKeyOperator::Subtract,
        left: Box::new(RelationKeyExpression::Parameter(0)),
        right: Box::new(RelationKeyExpression::Binary {
            operator: RelationKeyOperator::Minimum,
            left: Box::new(RelationKeyExpression::Parameter(1)),
            right: Box::new(RelationKeyExpression::Parameter(2)),
        }),
    };
    assert!(learned.iter().all(|item| item.expression == expected));
    assert!(learned.iter().all(|item| item.expression_candidates == 8));
    assert_eq!(learned[0].mechanism_hash(), learned[1].mechanism_hash());
    let receipts = learned
        .iter()
        .zip(&contrast_cohorts)
        .map(|(mechanism, cohort)| {
            ScalarExpressionVerifier
                .verify_reuse(mechanism, cohort, &full_limits)
                .expect("independent scalar full-tree replay")
        })
        .collect::<Vec<_>>();
    let promotion = ScalarExpressionPromotionConsolidator
        .consolidate(&receipts)
        .expect("scalar full-tree promotion");
    assert_eq!(promotion.expressions_saved(), 14);
    assert_eq!(promotion.verified_reuses(), 2);
    let mut memory = ScalarExpressionStore::new();
    let mechanism = learned[0].source_independent();
    let mechanism_id = memory
        .insert_provisional(mechanism)
        .expect("provisional scalar memory");
    for receipt in receipts {
        memory
            .record_verified_reuse(mechanism_id, receipt)
            .expect("disjoint scalar receipt");
    }
    assert!(memory.apply_verified_promotion(mechanism_id, &promotion));
    let runtime = CognitionRuntime::with_scalar_expression_memory(memory);
    let held_out = scalar_trace(
        ScalarFrontend::Vector,
        "association-scalar-heldout",
        [1, 3, 0, 2],
        5,
        -700,
    );
    let mut exact = 0_u64;
    let mut proposals_attempted = 0_u64;
    for example in &held_out.examples {
        let outcome = runtime
            .propose_scalar_expressions_from_memory(&example.inputs, &full_limits)
            .expect("answer-hidden scalar memory proposals");
        proposals_attempted = proposals_attempted.saturating_add(outcome.bindings_attempted);
        assert_eq!(outcome.persistent_mechanisms_considered, 1);
        if outcome
            .proposals
            .iter()
            .any(|proposal| proposal.output == example.output)
        {
            exact = exact.saturating_add(1);
        }
    }
    assert_eq!(
        exact,
        u64::try_from(held_out.examples.len()).expect("examples")
    );
    assert!(proposals_attempted > 0);
    eprintln!(
        "G6_ASSOCIATION_DISCRIMINATION ambiguous_before=1 request_content=0 scalar_tree_shapes=1->2 expressions=8->1 selected=subtract(p0,min(p1,p2)) receipts=2 saved=14 examples=4->0 persistent_modules=1 heldout_exact={exact} runtime_bindings={proposals_attempted}"
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
            .expect("source-independent base"),
        RawRelationObservationPromotionConsolidator
            .consolidate(&receipts)
            .expect("base promotion"),
    )
}

fn subtract_episode(
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
                    first - key,
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
enum ScalarFrontend {
    Sequence,
    Vector,
}

fn scalar_trace(
    frontend: ScalarFrontend,
    domain: &str,
    permutation: [usize; 4],
    scale: i64,
    offset: i64,
) -> ScalarExpressionTrace {
    let logical_inputs = [
        [20_i64, 3_i64, 9_i64],
        [20, 12, 4],
        [-5, -10, 2],
        [7, 11, -3],
        [100, 40, 60],
        [-20, -5, -30],
        [0, 8, 2],
        [31, 31, 17],
    ];
    let examples = logical_inputs
        .into_iter()
        .enumerate()
        .map(|(index, logical)| {
            let scaled = logical.map(|value| value * scale + offset);
            let distractor =
                10_000 * scale + offset + i64::try_from(index).expect("bounded example index");
            let logical_channels = [scaled[0], scaled[1], scaled[2], distractor];
            let mut inputs = vec![0_i64; logical_channels.len()];
            for (source, destination) in permutation.into_iter().enumerate() {
                inputs[destination] = logical_channels[source];
            }
            (inputs, scaled[0] - scaled[1].min(scaled[2]))
        })
        .collect::<Vec<_>>();
    let experience = Hash32::digest(domain.as_bytes());
    match frontend {
        ScalarFrontend::Sequence => SequenceAdapter.lower_scalar_expression(
            experience,
            &examples
                .iter()
                .map(|(channels, result)| SequenceScalarExample {
                    channels: channels.clone(),
                    result: *result,
                })
                .collect::<Vec<_>>(),
        ),
        ScalarFrontend::Vector => VectorAdapter.lower_scalar_expression(
            experience,
            &examples
                .iter()
                .map(|(channels, result)| VectorScalarConstraintExample {
                    channels: channels.clone(),
                    result: *result,
                })
                .collect::<Vec<_>>(),
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
                + i64::try_from(component).expect("component") * 100_000;
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

fn arity_failure() -> LearningFailureCore {
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
        minimum_dependency_arity_lower_bound: 3,
        redaction: RedactionStatus::ContentFree,
    }
}
