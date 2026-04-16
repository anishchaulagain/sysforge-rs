use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use chrono::Local;
use rand::seq::SliceRandom;

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

    {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(file_path)?;
        file.write_all(change_entry.as_bytes())?;
        file.flush()?; 
    } // File handle is closed here

    println!("✔ Added new entry to {}", file_path);

    // 3. Stage the change
    let add_status = Command::new("git")
        .args(["add", file_path])
        .status()?;
    
    if !add_status.success() {
        eprintln!("✘ Failed to stage changes with git add.");
        return Ok(());
    }

    // 4. Capture the git diff
    let diff_output = Command::new("git")
        .args(["diff", "--cached", "--", file_path])
        .output()?;
    
    let diff_text = String::from_utf8_lossy(&diff_output.stdout).to_string();
    
    if diff_text.trim().is_empty() {
        println!("⚠ No changes detected in diff. Ensure the file is being tracked by git.");
        return Ok(());
    }

    // 5. Query Groq API
    println!("⚡ Requesting semantic commit message...");

    let types = ["feat", "fix", "refactor", "perf", "chore", "docs", "test", "ci", "build"];
    let domains = [
        "memory safety", "concurrency", "asynchronous I/O", "trait refactors", 
        "optimization", "SIMD", "FFI", "serialization", "error handling", 
        "networking", "data structures", "compiler internal", "macro expansion"
    ];
    let components = [
        "core", "api", "worker", "store", "net", "parser", "crypto", "auth", 
        "cache", "scheduler", "logger", "config", "runtime"
    ];

    let mut rng = rand::thread_rng();
    let selected_type = types.choose(&mut rng).unwrap_or(&"feat");
    let selected_domain = domains.choose(&mut rng).unwrap_or(&"optimization");
    let selected_component = components.choose(&mut rng).unwrap_or(&"core");

    let prompt = format!(
        "You are an elite Systems Engineer working on a high-performance Rust codebase. \
        Generate a professional-sounding, highly technical commit message. \
        Even though the change might be minor, simulate a significant improvement in the area of {}. \
        \n\n\
        DIFF DATA (for context, but focus on the simulated change):\n{}\n\n\
        Rules:\n\
        1. Use semantic commit format using the type '{}' and component '{}'. \
           Example: {}({}): ...\n\
        2. Be creative: imagine a plausible low-level technical improvement related to {}.\n\
        3. Reply with ONLY the message. No quotes, no explanations.
        4. Keep it concise and impactful.
        ",
        selected_domain,
        diff_text,
        selected_type,
        selected_component,
        selected_type,
        selected_component,
        selected_domain
    );

    let client = Client::new();
    let res = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "model": "llama-3.3-70b-versatile",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a world-class Rust Systems Engineer. You write precise, technical, and professional commit messages."
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
