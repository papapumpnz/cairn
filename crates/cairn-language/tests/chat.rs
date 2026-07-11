use cairn_language::{GoalExpression, TechnicalChatbot};

#[test]
fn returns_verified_typed_python_for_basic_prompt() {
    let mut chatbot = TechnicalChatbot::new();
    let response = chatbot
        .respond("Give me a basic Python function")
        .expect("verified response");
    assert!(matches!(
        response.intent.objective,
        GoalExpression::GenerateProgram { .. }
    ));
    assert!(response
        .rendered
        .contains("def add_numbers(a: float, b: float) -> float:"));
    assert!(response.rendered.contains("return a + b"));
    assert!(chatbot.dialogue().current_goal.is_some());
}

#[test]
fn asks_for_an_explicit_objective_outside_the_initial_language_slice() {
    let mut chatbot = TechnicalChatbot::new();
    let response = chatbot.respond("surprise me").expect("controlled response");
    assert_eq!(
        response.rendered,
        "What would you like the program or explanation to do?"
    );
}
