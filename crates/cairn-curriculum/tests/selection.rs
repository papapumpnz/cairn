use cairn_curriculum::*;

#[test]
fn selects_information_over_redundant_easy_experience() {
    let candidates = [
        ExperienceCandidate {
            id: ExperienceId("repeat-known".to_owned()),
            expected_information_microunits: 1,
            expected_transfer_microunits: 1,
            underused_mechanism_bonus: 0,
            action_cost: 1,
            irreversible_risk: 0,
        },
        ExperienceCandidate {
            id: ExperienceId("distinguish-latent".to_owned()),
            expected_information_microunits: 20,
            expected_transfer_microunits: 5,
            underused_mechanism_bonus: 3,
            action_cost: 4,
            irreversible_risk: 1,
        },
    ];
    assert_eq!(
        Curriculum.select(&candidates).expect("selected").id,
        ExperienceId("distinguish-latent".to_owned())
    );
}
