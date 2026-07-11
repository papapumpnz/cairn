use cairn_language::{GoalExpression, TechnicalChatbot};
use cairn_language::{LanguageLesson, OutputSpecification, ProgramOperation, ProgrammingLanguage};

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

#[test]
fn installed_lesson_transfers_over_structural_paraphrase_but_not_wrong_artifact() {
    let mut chatbot = TechnicalChatbot::new();
    chatbot.install_lesson(LanguageLesson {
        utterance: "fashion a py callable that totals two values".to_owned(),
        objective: GoalExpression::GenerateProgram {
            language: ProgrammingLanguage::Python,
            operation: ProgramOperation::AddNumbers,
        },
        requested_output: OutputSpecification::SourceCode,
    });
    let transferred = chatbot
        .respond("make a Python function to sum numbers")
        .expect("structural paraphrase");
    assert!(transferred.rendered.contains("def add_numbers"));
    let negative = chatbot
        .respond("explain a Python class")
        .expect("controlled negative");
    assert!(negative.rendered.starts_with("What would you like"));
}
