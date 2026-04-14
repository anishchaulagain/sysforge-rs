use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use rand::seq::SliceRandom;
use chrono::Local;

#[derive(Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load environment variables
    dotenvy::dotenv().ok();
    let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set in .env");

    // 2. Pick a random phrase
    let phrases = vec![
        "Refactoring core data parser to improve memory efficiency.",
        "Fixing edge case in string normalization utility.",
        "Updating dependencies for security patches.",
        "Added comprehensive unit tests for the authentication module.",
        "Optimizing the database query layer for faster reads.",
        "Cleaning up deceased code blocks and deprecated functions.",
        "Enhancing logging output for better debugging in production.",
    ];
    let mut rng = rand::thread_rng();
    let selected_phrase = phrases.choose(&mut rng).unwrap();

    // Append a timestamp to make it unique every time
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let diff_text = format!("- [{}] {}\n", timestamp, selected_phrase);

    // 3. Append phrase to CHANGES.md
    let file_path = "CHANGES.md";
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(file_path)?;

    if let Err(e) = writeln!(file, "{}", diff_text) {
        eprintln!("Couldn't write to file: {}", e);
    }
    println!("Appended random change to {}", file_path);

    // 4. Query Groq API for commit msg
    let prompt = format!(
        "Generate a brief, semantic commit message (e.g., 'chore: ...' or 'refactor: ...') based on this change log entry:\n{}\n\nReply with ONLY the commit message and nothing else.",
        diff_text
    );

    let client = Client::new();
    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": "llama3-8b-8192",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?;

    let response_data: GroqResponse = res.json().await?;
    let mut commit_message = response_data.choices[0].message.content.trim().to_string();
    
    // Remove quotes if the AI wrapped it in double quotes
    commit_message = commit_message.trim_matches('"').to_string();

    println!("Generated commit message: {}", commit_message);

    // 5. Execute `git add CHANGES.md`
    println!("Staging changes...");
    Command::new("git")
        .arg("add")
        .arg(file_path)
        .output()
        .expect("Failed to execute git add");

    // 6. Execute `git commit -m ...`
    println!("Committing changes...");
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(&commit_message)
        .output()
        .expect("Failed to execute git commit");

    println!("Success! Random commit generated.");
    Ok(())
}
