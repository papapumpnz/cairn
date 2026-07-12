use cairn_theory::{
    CoordinateLayer, LayerLearningExample, LayerLearningLimits, LayerMechanismError,
    LayerMechanismLearner, LayerRender, LayerScene,
};
use cairn_vector::VectorAdapter;
use std::collections::BTreeSet;

#[test]
fn learns_intersection_with_tight_rendering_and_rebinds_values_positions_and_counts() {
    let training = [
        intersection_example(8, 3, 11, 20, 1),
        intersection_example(12, 5, 17, 40, 2),
    ];
    let mechanism = VectorAdapter
        .learn_layer_mechanism(&training, LayerLearningLimits::default())
        .expect("learn intersection renderer");
    let held_out = intersection_example(15, 7, 29, 70, 4);
    assert_eq!(
        LayerMechanismLearner.execute(&mechanism, &held_out.scene, 1_000),
        Ok(held_out.expected)
    );
}

#[test]
fn independently_learns_difference_and_union_renderers() {
    let difference = [
        difference_example(9, 2, 13, 30),
        difference_example(14, 4, 23, 50),
    ];
    let difference_mechanism = VectorAdapter
        .learn_layer_mechanism(&difference, LayerLearningLimits::default())
        .expect("difference");
    let difference_held_out = difference_example(18, 6, 31, 90);
    assert_eq!(
        LayerMechanismLearner.execute(&difference_mechanism, &difference_held_out.scene, 1_000),
        Ok(difference_held_out.expected)
    );

    let union = [union_example(7, 2, 19, 60), union_example(11, 3, 37, 80)];
    let union_mechanism = VectorAdapter
        .learn_layer_mechanism(&union, LayerLearningLimits::default())
        .expect("union");
    let union_held_out = union_example(16, 5, 41, 100);
    assert_eq!(
        LayerMechanismLearner.execute(&union_mechanism, &union_held_out.scene, 1_000),
        Ok(union_held_out.expected)
    );
}

#[test]
fn ties_and_candidate_caps_fail_closed() {
    let tied = LayerLearningExample {
        scene: LayerScene {
            extents: vec![5, 5],
            background: 0,
            layers: vec![
                CoordinateLayer {
                    value: 2,
                    points: points(&[(1, 1), (2, 2)]),
                },
                CoordinateLayer {
                    value: 7,
                    points: points(&[(1, 2), (2, 3)]),
                },
            ],
        },
        expected: LayerRender {
            extents: vec![5, 5],
            background: 0,
            foreground: 2,
            points: points(&[(1, 1)]),
        },
    };
    assert_eq!(
        VectorAdapter.learn_layer_mechanism(&[tied], LayerLearningLimits::default()),
        Err(LayerMechanismError::NoMechanism)
    );
    assert!(matches!(
        VectorAdapter.learn_layer_mechanism(
            &[intersection_example(8, 3, 11, 20, 1)],
            LayerLearningLimits {
                maximum_candidates: 1,
                maximum_points: 100
            }
        ),
        Err(LayerMechanismError::CandidateCapacity { maximum: 1 })
    ));
}

fn intersection_example(
    extent: u32,
    shift: i64,
    minimum_value: i64,
    maximum_value: i64,
    extra: i64,
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
        (shift + 2, shift + 2),
        (shift + 3, shift + extra),
    ]);
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
            extents: vec![3, 3],
            background: -1,
            foreground: minimum_value,
            points: points(&[(0, 0), (1, 1), (2, 2)]),
        },
    }
}

fn difference_example(
    extent: u32,
    shift: i64,
    minimum_value: i64,
    maximum_value: i64,
) -> LayerLearningExample {
    let minimum = points(&[(shift, shift), (shift + 1, shift + 1)]);
    let maximum = points(&[
        (shift, shift),
        (shift + 1, shift + 1),
        (shift + 2, shift),
        (shift + 3, shift + 1),
    ]);
    LayerLearningExample {
        scene: LayerScene {
            extents: vec![extent, extent],
            background: 0,
            layers: vec![
                CoordinateLayer {
                    value: minimum_value,
                    points: minimum,
                },
                CoordinateLayer {
                    value: maximum_value,
                    points: maximum,
                },
            ],
        },
        expected: LayerRender {
            extents: vec![2, 2],
            background: 0,
            foreground: maximum_value,
            points: points(&[(0, 0), (1, 1)]),
        },
    }
}

fn union_example(
    extent: u32,
    shift: i64,
    minimum_value: i64,
    maximum_value: i64,
) -> LayerLearningExample {
    let minimum = points(&[(shift, shift), (shift + 1, shift)]);
    let maximum = points(&[(shift, shift), (shift, shift + 1), (shift + 1, shift + 1)]);
    let union = minimum.union(&maximum).cloned().collect();
    LayerLearningExample {
        scene: LayerScene {
            extents: vec![extent, extent],
            background: -5,
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
            background: -5,
            foreground: maximum_value,
            points: union,
        },
    }
}

fn points(values: &[(i64, i64)]) -> BTreeSet<Vec<i64>> {
    values
        .iter()
        .map(|(first, second)| vec![*first, *second])
        .collect()
}
