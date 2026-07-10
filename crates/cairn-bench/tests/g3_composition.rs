use cairn_bench::g3_unseen_skill_compositions;
use cairn_skill::{ResourceModel, SkillInducer, SkillPlanError, SkillPlanner};

#[test]
fn independently_induced_skills_compose_without_a_composite_trace() {
    let suite = g3_unseen_skill_compositions();
    let skills = suite
        .independent_skill_traces
        .iter()
        .map(|traces| {
            SkillInducer
                .induce(
                    traces,
                    ResourceModel {
                        maximum_steps: 8,
                        maximum_cost: 100,
                    },
                )
                .expect("independent skill induction")
        })
        .collect::<Vec<_>>();
    assert_eq!(skills.len(), 4);

    let mut exact = 0_u32;
    let mut explicit_invocations = 0_u64;
    for (initial, goal) in &suite.scenarios {
        let plan = SkillPlanner
            .plan(initial, goal, &skills, 100)
            .expect("unseen skill composition");
        exact += u32::from(goal.holds(&plan.final_state));
        explicit_invocations += u64::try_from(plan.invocations.len()).expect("small plan");
        assert_eq!(plan.invocations.len(), 4);
    }
    assert_eq!(exact, 100);
    assert_eq!(explicit_invocations, 400);

    let mut predictable_ablation_failures = 0_u32;
    for removed in 0..skills.len() {
        let ablated = skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| (index != removed).then_some(skill.clone()))
            .collect::<Vec<_>>();
        for (initial, goal) in &suite.scenarios {
            if SkillPlanner.plan(initial, goal, &ablated, 100) == Err(SkillPlanError::NoPlan) {
                predictable_ablation_failures += 1;
            }
        }
    }
    assert_eq!(predictable_ablation_failures, 400);
    eprintln!(
        "G3_METRICS exact={exact}/100 explicit_invocations={explicit_invocations} predictable_ablation_failures={predictable_ablation_failures}/400"
    );
}
