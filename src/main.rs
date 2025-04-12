use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use git2::{Repository, Signature};
use home::home_dir;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use thiserror::Error;

const APP_NAME: &str = "ac-garden";
const ATCODER_API_SUBMISSION_URL: &str = "https://kenkoooo.com/atcoder/atcoder-api/results?user=";

#[derive(Debug, Serialize, Deserialize)]
struct AtCoderSubmission {
    id: i64,
    epoch_second: i64,
    problem_id: String,
    contest_id: String,
    user_id: String,
    language: String,
    point: f64,
    length: i64,
    result: String,
    execution_time: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Service {
    repository_path: String,
    user_id: String,
    user_email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    atcoder: Service,
}

#[derive(Parser)]
#[command(name = "ac-garden")]
#[command(about = "Archive your AC submissions", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Archive your AC submissions
    Archive,
    /// Initialize your config
    Init {
        /// Force recreate config
        #[arg(short, long)]
        force: bool,
    },
    /// Edit your config file
    Edit,
}

#[derive(Error, Debug)]
enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),
    #[error("Config error: {0}")]
    Config(String),
}

fn language_to_file_name(language: &str) -> String {
    let language = if let Some(idx) = language.find('(') {
        &language[..idx].trim()
    } else {
        language
    };

    match language {
        "C++" | "C++14" | "C++17" | "C++20" => "Main.cpp",
        "Bash" => "Main.sh",
        "C" => "Main.c",
        "C#" => "Main.cs",
        "Clojure" => "Main.clj",
        "Common Lisp" => "Main.lisp",
        "D" => "Main.d",
        "Fortran" => "Main.f08",
        "Go" => "Main.go",
        "Haskell" => "Main.hs",
        "JavaScript" => "Main.js",
        "Java" => "Main.java",
        "OCaml" => "Main.ml",
        "Pascal" => "Main.pas",
        "Perl" => "Main.pl",
        "PHP" => "Main.php",
        "Python" | "Python3" | "PyPy2" | "PyPy3" => "Main.py",
        "Ruby" => "Main.rb",
        "Scala" => "Main.scala",
        "Scheme" => "Main.scm",
        "Visual Basic" => "Main.vb",
        "Objective-C" => "Main.m",
        "Swift" => "Main.swift",
        "Rust" => "Main.rs",
        "Sed" => "Main.sed",
        "Awk" => "Main.awk",
        "Brainfuck" => "Main.bf",
        "Standard ML" => "Main.sml",
        "Crystal" => "Main.cr",
        "F#" => "Main.fs",
        "Unlambda" => "Main.unl",
        "Lua" | "LuaJIT" => "Main.lua",
        "MoonScript" => "Main.moon",
        "Ceylon" => "Main.ceylon",
        "Julia" => "Main.jl",
        "Octave" => "Main.m",
        "Nim" => "Main.nim",
        "TypeScript" => "Main.ts",
        "Perl6" => "Main.p6",
        "Kotlin" => "Main.kt",
        "COBOL" => "Main.cob",
        _ => {
            eprintln!("Unknown language: {}", language);
            "Main.txt"
        }
    }.to_string()
}

fn is_dir_exist<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_dir()
}

fn is_file_exist<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_file()
}

fn get_config_dir() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| AppError::Config("Failed to get home directory".into()))?;
    Ok(home.join(format!(".{}", APP_NAME)))
}

fn get_config_file() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("config.json"))
}

fn init_config(force: bool) -> Result<()> {
    println!("Initialize your config...");
    
    let config_dir = get_config_dir()?;
    
    if !is_dir_exist(&config_dir){
        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;
    }

    let config_file = get_config_file()?;
    
    if force || !is_file_exist(&config_file) {
        // 初期設定
        let atcoder = Service {
            repository_path: String::new(),
            user_id: String::new(),
            user_email: String::new(),
        };

        let config = Config { atcoder };

        let json = serde_json::to_string_pretty(&config)
            .context("Failed to serialize config")?;

        let mut file = File::create(&config_file)
            .context("Failed to create config file")?;
            
        file.write_all(json.as_bytes())
            .context("Failed to write config file")?;
    }

    println!("Initialized your config at {}", config_file.display());
    Ok(())
}

fn load_config() -> Result<Config> {
    let config_file = get_config_file()?;
    let config_str = fs::read_to_string(&config_file)
        .context("Failed to read config file")?;
    
    let config: Config = serde_json::from_str(&config_str)
        .context("Failed to parse config")?;
    
    Ok(config)
}

async fn archive_file(code: &str, file_name: &str, path: &Path, submission: &AtCoderSubmission) -> Result<()> {
    fs::create_dir_all(path)
        .context("Failed to create directory")?;
        
    let file_path = path.join(file_name);
    
    fs::write(&file_path, code)
        .context("Failed to write source file")?;
    
    // 提出JSONを保存
    let json = serde_json::to_string_pretty(submission)
        .context("Failed to serialize submission")?;
        
    fs::write(path.join("submission.json"), json)
        .context("Failed to write submission.json")?;
    
    Ok(())
}

async fn archive_cmd() -> Result<()> {
    let config = load_config()?;
    
    let client = Client::new();
    let url = format!("{}{}", ATCODER_API_SUBMISSION_URL, &config.atcoder.user_id);
    
    // APIからレスポンスを取得
    let response = client.get(&url).send().await?;
    let text = response.text().await?;
    // 生のレスポンスを出力して内容を確認
    println!("Raw response: {}", text);
    
    // ここでエラーになっているので、レスポンスの形式をまず確認する
    let submissions: Vec<AtCoderSubmission> = serde_json::from_str(&text)
        .context("Failed to decode response as an array")?;
    
    // AC提出だけをフィルタリング
    let ac_submissions: Vec<AtCoderSubmission> = submissions.into_iter()
        .filter(|s| s.result == "AC")
        .collect();
    
    // すでにアーカイブされたコードをスキップ
    let mut archived_keys = std::collections::HashSet::new();
    
    let repo_path = Path::new(&config.atcoder.repository_path);
    
    if is_dir_exist(repo_path) {
        for entry in walkdir::WalkDir::new(repo_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with("submission.json"))
        {
            let content = fs::read_to_string(entry.path())?;
            let submission: AtCoderSubmission = serde_json::from_str(&content)?;
            let key = format!("{}_{}", submission.contest_id, submission.problem_id);
            archived_keys.insert(key);
        }
    }
    
    let mut filtered_submissions: Vec<AtCoderSubmission> = ac_submissions.into_iter()
        .filter(|s| {
            let key = format!("{}_{}", s.contest_id, s.problem_id);
            !archived_keys.contains(&key)
        })
        .collect();
    
    // 提出時間で逆順ソート
    filtered_submissions.sort_by(|a, b| b.epoch_second.cmp(&a.epoch_second));
    
    // 各問題の最新提出だけをフィルタリング
    let mut seen = std::collections::HashSet::new();
    let mut unique_submissions = Vec::new();
    
    for submission in filtered_submissions {
        let key = format!("{}_{}", submission.contest_id, submission.problem_id);
        if !seen.contains(&key) {
            seen.insert(key);
            unique_submissions.push(submission);
        }
    }
    
    println!("Archiving {} code...", unique_submissions.len());
    
    let mut start_time = Instant::now();
    
    for submission in unique_submissions {
        let url = format!(
            "https://atcoder.jp/contests/{}/submissions/{}",
            submission.contest_id, submission.id
        );
        
        let elapsed = start_time.elapsed();
        if elapsed < Duration::from_millis(1500) {
            let sleep_time = Duration::from_millis(1500) - elapsed;
            tokio::time::sleep(sleep_time).await;
        }
        
        let response = client.get(&url).send().await?;
        start_time = Instant::now();
        
        let html = response.text().await?;
        let document = Html::parse_document(&html);
        
        let selector = Selector::parse("#submission-code").unwrap();
        
        if let Some(element) = document.select(&selector).next() {
            let code = element.text().collect::<Vec<_>>().join("");
            
            if code.is_empty() {
                println!("Empty string...");
                continue;
            }
            
            let file_name = language_to_file_name(&submission.language);
            let archive_dir_path = repo_path
                .join("atcoder.jp")
                .join(&submission.contest_id)
                .join(&submission.problem_id);
            
            archive_file(&code, &file_name, &archive_dir_path, &submission).await?;
            
            println!("archived the code at {}", archive_dir_path.join(&file_name).display());
            
            // Gitリポジトリである場合、gitのaddとcommit
            let git_dir = repo_path.join(".git");
            if is_dir_exist(&git_dir) {
                let repo = Repository::open(repo_path)?;
                let mut index = repo.index()?;
                
                // ソースファイルをadd
                let rel_path = PathBuf::from("atcoder.jp")
                    .join(&submission.contest_id)
                    .join(&submission.problem_id)
                    .join(&file_name);
                    
                index.add_path(&rel_path)?;
                
                // submission.jsonをadd
                let json_path = PathBuf::from("atcoder.jp")
                    .join(&submission.contest_id)
                    .join(&submission.problem_id)
                    .join("submission.json");
                    
                index.add_path(&json_path)?;
                index.write()?;
                
                let tree_id = index.write_tree()?;
                let tree = repo.find_tree(tree_id)?;
                
                let head = repo.head()?;
                let parent_commit = repo.find_commit(head.target().unwrap())?;
                
                let user_id = &submission.user_id;
                let user_email = &config.atcoder.user_email;
                
                // タイムスタンプの処理
                // dt変数を削除または_dtにリネーム（未使用変数の警告を防ぐ）
                // let dt = Utc.timestamp_opt(submission.epoch_second, 0).unwrap();
                
                let signature = Signature::new(
                    user_id,
                    user_email,
                    &git2::Time::new(submission.epoch_second, 0),
                )?;
                
                let message = format!("[AC] {} {}", submission.contest_id, submission.problem_id);
                
                repo.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    &message,
                    &tree,
                    &[&parent_commit],
                )?;
            }
        }
    }
    
    Ok(())
}

fn edit_cmd() -> Result<()> {
    let config_file = get_config_file()?;
    
    // 設定ファイルが存在しない場合は初期化
    if !is_file_exist(&config_file) {
        init_config(true)?;
    }
    
    // 環境変数EDITORがあればそれを使う
    if let Ok(editor) = std::env::var("EDITOR") {
        Command::new(&editor)
            .arg(&config_file)
            .status()?;
    } else {
        // ブラウザで開く (Windows/Mac/Linux)
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(&["/c", "start", "", config_file.to_str().unwrap()])
                .spawn()?;
        }
        
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(&config_file)
                .spawn()?;
        }
        
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open")
                .arg(&config_file)
                .spawn()?;
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Archive => {
            archive_cmd().await?;
        },
        Commands::Init { force } => {
            init_config(force)?;
        },
        Commands::Edit => {
            edit_cmd()?;
        }
    }

    Ok(())
}
