use cairn_language::TechnicalChatbot;
use std::io::{self, BufRead};

fn main() {
    let mut chatbot = TechnicalChatbot::new();
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if !arguments.is_empty() {
        respond(&mut chatbot, &arguments.join(" "));
        return;
    }
    for line in io::stdin().lock().lines() {
        match line {
            Ok(line) if !line.trim().is_empty() => respond(&mut chatbot, &line),
            Ok(_) => {}
            Err(error) => {
                eprintln!("input error: {error}");
                std::process::exit(2);
            }
        }
    }
}

fn respond(chatbot: &mut TechnicalChatbot, prompt: &str) {
    match chatbot.respond(prompt) {
        Ok(response) => println!("{}", response.rendered),
        Err(error) => {
            eprintln!("response verification failed: {error:?}");
            std::process::exit(1);
        }
    }
}
