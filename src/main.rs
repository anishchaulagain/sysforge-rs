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

#[derive(Deserialize)]
struct CommitInfo {
    function_name: String,
    commit_message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load environment variables
    dotenvy::dotenv().ok();
    let api_key = env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set in .env");

    // 2. Select focus area and request LLM generation
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

    println!("⚡ Requesting LLM-generated function name and commit message...");
    let prompt = format!(
        "You are an elite Systems Engineer working on a high-performance Rust codebase. \
        Generate a highly technical Rust function name for a stub and a concise commit message. \
        Focus: {} \
        \n\n\
        Rules:\n\
        1. Return ONLY valid JSON: {{\"function_name\": \"...\", \"commit_message\": \"...\"}}\n\
        2. function_name must be technical snake_case (e.g. 'align_simd_buffers').\n\
        3. commit_message MUST be under 10 words and use '{}({})' format.\n\
        4. No markdown formatting or extra text.",
        selected_domain, selected_type, selected_component
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
                    "content": "You are a world-class Rust Systems Engineer. You return precise JSON outputs."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "response_format": { "type": "json_object" }
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        eprintln!("API Error: {}", err_text);
        return Ok(());
    }

    let response_data: GroqResponse = res.json().await?;
    let content = response_data.choices[0].message.content.trim();
    let info: CommitInfo = serde_json::from_str(content)?;

    // 3. Make small unique changes
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 3a. Update CHANGES.md
    let log_path = "CHANGES.md";
    let change_entry = format!("- Activity log update: {}\n", timestamp);
    {
        let mut file = OpenOptions::new().write(true).append(true).create(true).open(log_path)?;
        file.write_all(change_entry.as_bytes())?;
    }
    println!("✔ Updated {}", log_path);

    // 3b. Update src/stubs.rs (Code change using LLM-generated name)
    let code_path = "src/stubs.rs";
    let stub_code = format!(
        "\n/// Generated stub for: {}\npub fn {}() -> bool {{ true }}\n",
        info.commit_message, info.function_name
    );
    {
        let mut file = OpenOptions::new().write(true).append(true).create(true).open(code_path)?;
        file.write_all(stub_code.as_bytes())?;
    }
    println!("✔ Generated function '{}' in {}", info.function_name, code_path);

    // 4. Stage and finish
    Command::new("git").args(["add", log_path, code_path]).status()?;
    
    // 6. Output the result
    println!("\n--- Suggested Commit Message ---");
    println!("{}", info.commit_message);
    println!("----------------------------------");
    println!("\nTo commit these changes, run:");
    println!("git commit -m \"{}\"", info.commit_message);

    Ok(())
}
