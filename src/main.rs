use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use chrono::Local;

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

    // 2. Make a small unique change to CHANGES.md
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let change_entry = format!("- Activity log update: {}\n", timestamp);
    
    let file_path = "CHANGES.md";
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_path)?;

    file.write_all(change_entry.as_bytes())?;
    println!("Added new entry to {}", file_path);

    // 3. Stage the change so we can diff it
    println!("Staging changes...");
    let add_status = Command::new("git")
        .args(["add", file_path])
        .status()?;
    
    if !add_status.success() {
        eprintln!("Failed to stage changes with git add.");
        return Ok(());
    }

    // 4. Capture the git diff
    println!("Capturing diff...");
    let diff_output = Command::new("git")
        .args(["diff", "--cached", file_path])
        .output()?;
    
    let diff_text = String::from_utf8_lossy(&diff_output.stdout);
    
    if diff_text.is_empty() {
        println!("No changes detected in diff.");
        return Ok(());
    }

    // 5. Query Groq API for commit msg based on actual diff
    println!("Requesting commit message from LLM...");
    let prompt = format!(
        "Analyze the following git diff and generate a concise, semantic commit message (e.g., 'chore: ...', 'feat: ...', 'docs: ...').\n\nDIFF:\n```diff\n{}\n```\n\nReply with ONLY the commit message and nothing else. No quotes, no markdown.",
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
                    "role": "system",
                    "content": "You are a git expert. You provide clear, concise, semantic commit messages."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        eprintln!("API Error: {}", err_text);
        return Ok(());
    }

    let response_data: GroqResponse = res.json().await?;
    let commit_message = response_data.choices[0].message.content.trim().to_string();
    
    // 6. Output the result for manual commitment
    println!("\n--- Suggested Commit Message ---");
    println!("{}", commit_message);
    println!("----------------------------------");
    println!("\nTo commit these changes, run:");
    println!("git commit -m \"{}\"", commit_message);

    Ok(())
}
