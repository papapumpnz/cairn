use cairn_field::{FiniteField, LayerFieldAdapter};
use cairn_memory::{LayerMechanismStore, LayerPromotionProgress, ModuleStatus};
use cairn_runtime::{CognitionRuntime, LayerMemoryRuntimeError};
use cairn_theory::{
    CoordinateLayer, LayerLearningExample, LayerLearningLimits, LayerMechanismLearner, LayerRender,
    LayerRenderFrame, LayerScene,
};
use cairn_vector::VectorAdapter;
use cairn_verifier::{LayerMechanismVerifier, LayerVerifyError};
use std::collections::BTreeSet;

#[test]
fn verified_vector_layer_mechanism_promotes_and_transfers_to_finite_fields() {
    let first = [example(9, 2, 11, 30), example(13, 4, 17, 50)];
    let second = [example(16, 6, 23, 70), example(20, 8, 31, 90)];
    let mechanism = VectorAdapter
        .learn_layer_mechanism(&first, LayerLearningLimits::default())
        .expect("first learned mechanism");
    assert_eq!(
        VectorAdapter
            .learn_layer_mechanism(&second, LayerLearningLimits::default())
            .expect("independent mechanism"),
        mechanism
    );
    let first_receipt = LayerMechanismVerifier
        .verify(&mechanism, &first, 1_000)
        .expect("first independent cohort");
    let second_receipt = LayerMechanismVerifier
        .verify(&mechanism, &second, 1_000)
        .expect("second independent cohort");
    let mut store = LayerMechanismStore::new();
    assert_eq!(
        store
            .record_verified_cohort(&mechanism, first_receipt)
            .expect("first cohort"),
        LayerPromotionProgress::Provisional {
            verified_cohorts: 1
        }
    );
    assert_eq!(
        store
            .record_verified_cohort(&mechanism, second_receipt)
            .expect("second cohort"),
        LayerPromotionProgress::Persistent {
            verified_cohorts: 2
        }
    );
    let id = mechanism.semantic_hash().expect("hash");
    let module = store.module(id).expect("module");
    assert_eq!(module.status(), ModuleStatus::Persistent);
    assert_eq!(module.verified_experiences(), 4);
    assert_eq!(module.support().observed_dimensions, BTreeSet::from([2]));
    assert_eq!(module.support().observed_extents.len(), 4);
    assert_eq!(module.support().observed_foregrounds.len(), 4);

    let held_out = example(24, 10, 41, 100);
    let runtime = CognitionRuntime::with_layer_mechanism_memory(store);
    let proposals = runtime
        .propose_layer_renders_from_memory(&held_out.scene, 1_000)
        .expect("runtime proposal");
    assert_eq!(proposals.proposals.len(), 1);
    assert_eq!(proposals.proposals[0].render, held_out.expected);
    LayerMechanismVerifier
        .verify(
            &mechanism,
            &[held_out.clone(), example(27, 12, 43, 120)],
            1_000,
        )
        .expect("held-out independent replay");

    let field = field_scene();
    let predicted = LayerFieldAdapter
        .predict(&mechanism, &field, 1_000)
        .expect("cross-domain field render");
    assert_eq!(predicted, expected_field_render());

    let mut wrong_renderer = mechanism.clone();
    wrong_renderer.frame = LayerRenderFrame::TightContent;
    assert_ne!(
        LayerMechanismLearner
            .execute(&wrong_renderer, &held_out.scene, 1_000)
            .expect("ablation render"),
        held_out.expected
    );
    assert!(matches!(
        CognitionRuntime::new().propose_layer_renders_from_memory(&held_out.scene, 1_000),
        Err(cairn_runtime::CognitionError::LayerMemory(
            LayerMemoryRuntimeError::NoVerifiedProposal
        ))
    ));
    eprintln!(
        "G5_LAYER_MECHANISM cohorts=2 experiences=4 vector_heldout=2 field_transfer=1 blank=0 renderer_ablation=fail"
    );
}

#[test]
fn changed_expected_render_is_rejected_by_independent_verifier() {
    let examples = [example(9, 2, 11, 30), example(13, 4, 17, 50)];
    let mechanism = VectorAdapter
        .learn_layer_mechanism(&examples, LayerLearningLimits::default())
        .expect("mechanism");
    let mut changed = examples.clone();
    changed[1].expected.foreground = 999;
    assert_eq!(
        LayerMechanismVerifier.verify(&mechanism, &changed, 1_000),
        Err(LayerVerifyError::ReplayMismatch)
    );
}

fn example(
    extent: u32,
    shift: i64,
    minimum_value: i64,
    maximum_value: i64,
) -> LayerLearningExample {
    let minimum = points(&[
        (shift, shift),
        (shift + 1, shift + 1),
        (shift + 2, shift + 2),
    ]);
    let maximum = points(&[
        (shift - 1, shift),
        (shift, shift),
        (shift + 1, shift + 1),
        (shift + 3, shift + 1),
        (shift + 4, shift + 2),
    ]);
    let union = minimum.union(&maximum).cloned().collect();
    LayerLearningExample {
        scene: LayerScene {
            extents: vec![extent, extent],
            background: -1,
            layers: vec![
                CoordinateLayer {
                    value: maximum_value,
                    points: maximum,
                },
                CoordinateLayer {
                    value: minimum_value,
                    points: minimum,
                },
            ],
        },
        expected: LayerRender {
            extents: vec![extent, extent],
            background: -1,
            foreground: maximum_value,
            points: union,
        },
    }
}

fn field_scene() -> FiniteField {
    let mut values = vec![0_i64; 64];
    for (row, column) in [(2_usize, 2_usize), (3, 3), (4, 4)] {
        values[row * 8 + column] = 2;
    }
    for (row, column) in [(1_usize, 1_usize), (1, 2), (1, 3), (5, 3), (6, 3)] {
        values[row * 8 + column] = 5;
    }
    FiniteField {
        shape: vec![8, 8],
        values,
    }
}

fn expected_field_render() -> FiniteField {
    let source = field_scene();
    FiniteField {
        shape: source.shape,
        values: source
            .values
            .into_iter()
            .map(|value| if value == 0 { 0 } else { 5 })
            .collect(),
    }
}

fn points(values: &[(i64, i64)]) -> BTreeSet<Vec<i64>> {
    values
        .iter()
        .map(|(row, column)| vec![*row, *column])
        .collect()
}
