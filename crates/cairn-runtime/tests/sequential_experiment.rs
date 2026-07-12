use cairn_core::Hash32;
use cairn_runtime::{
    SequentialExperimentError, SequentialExperimentLimits, SequentialTheoryHypothesis,
    SequentialTransition, SequentialVersionSpace,
};
use std::collections::BTreeMap;

#[test]
fn invents_a_two_action_experiment_when_every_single_action_is_ambiguous() {
    let probe = id("probe");
    let reveal = id("reveal");
    let common = id("common");
    let left = id("left");
    let right = id("right");
    let mut space = SequentialVersionSpace::new(vec![
        hypothesis("theory-a", "a0", "a1", probe, reveal, common, left),
        hypothesis("theory-b", "b0", "b1", probe, reveal, common, right),
    ])
    .expect("version space");

    assert_eq!(
        space.next_discriminating_plan(
            &[probe, reveal],
            SequentialExperimentLimits {
                maximum_depth: 1,
                maximum_plans: 10,
            }
        ),
        Err(SequentialExperimentError::NoDistinguishingPlan)
    );
    let plan = space
        .next_discriminating_plan(
            &[reveal, probe],
            SequentialExperimentLimits {
                maximum_depth: 2,
                maximum_plans: 10,
            },
        )
        .expect("two-step diagnostic");
    assert_eq!(plan.actions, vec![probe, reveal]);
    assert_eq!(plan.outcome_partitions, 2);
    assert_eq!(plan.theories_compared, 2);
    assert!(plan.plans_examined <= 6);

    space
        .observe_plan(&plan.actions, &[common, right])
        .expect("observed experiment");
    assert_eq!(space.sole_hypothesis_id(), Some(id("theory-b")));
}

#[test]
fn plan_caps_and_refuting_observations_fail_closed() {
    let probe = id("probe");
    let reveal = id("reveal");
    let common = id("common");
    let mut space = SequentialVersionSpace::new(vec![
        hypothesis("theory-a", "a0", "a1", probe, reveal, common, id("left")),
        hypothesis("theory-b", "b0", "b1", probe, reveal, common, id("right")),
    ])
    .expect("version space");
    assert_eq!(
        space.next_discriminating_plan(
            &[probe, reveal],
            SequentialExperimentLimits {
                maximum_depth: 2,
                maximum_plans: 2,
            }
        ),
        Err(SequentialExperimentError::Capacity { maximum: 2 })
    );
    let before = space.clone();
    assert_eq!(
        space.observe_plan(&[probe, reveal], &[common, id("impossible")]),
        Err(SequentialExperimentError::AllTheoriesRefuted)
    );
    assert_eq!(space, before);
}

#[allow(clippy::too_many_arguments)]
fn hypothesis(
    name: &str,
    initial: &str,
    after_probe: &str,
    probe: Hash32,
    reveal: Hash32,
    common: Hash32,
    revealed: Hash32,
) -> SequentialTheoryHypothesis {
    let initial = id(initial);
    let after_probe = id(after_probe);
    SequentialTheoryHypothesis {
        id: id(name),
        initial_state: initial,
        transitions: BTreeMap::from([
            (
                (initial, probe),
                SequentialTransition {
                    next_state: after_probe,
                    visible_outcome: common,
                },
            ),
            (
                (initial, reveal),
                SequentialTransition {
                    next_state: initial,
                    visible_outcome: common,
                },
            ),
            (
                (after_probe, probe),
                SequentialTransition {
                    next_state: after_probe,
                    visible_outcome: common,
                },
            ),
            (
                (after_probe, reveal),
                SequentialTransition {
                    next_state: after_probe,
                    visible_outcome: revealed,
                },
            ),
        ]),
    }
}

fn id(value: &str) -> Hash32 {
    Hash32::digest(value.as_bytes())
}
