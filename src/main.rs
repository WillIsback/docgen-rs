mod apply;
mod cli;
mod config;
mod detect;
mod error;
mod git;
mod llm;
mod process;
mod resolver;
mod tailwind;

use clap::Parser;
use cli::Cli;
use config::Config;

#[tokio::main]
async fn main() {
    if let Some(home) = std::env::var_os("HOME") {
        let _ = dotenvy::from_path(std::path::Path::new(&home).join(".config/docgen/.env"));
    }
    let _ = dotenvy::dotenv();

    let args = Cli::parse();
    let cfg = Config::load(std::path::Path::new("."));

    let dirty = git::dirty_files(std::path::Path::new("."));
    if !dirty.is_empty() {
        eprintln!("error: working tree has uncommitted changes. Commit or stash before running docgen:");
        for f in &dirty { eprintln!("  {f}"); }
        std::process::exit(1);
    }

    if !llm::check_reachable(&cfg).await {
        eprintln!("error: vLLM not reachable at {}", cfg.vllm_base_url);
        std::process::exit(1);
    }

    let model = match llm::resolve_model(&cfg).await {
        Ok(m) => { println!("Model: {m}"); m }
        Err(e) => { eprintln!("error: {e}"); std::process::exit(1); }
    };

    let files = resolver::resolve_files(&args.target, args.recursive, &cfg.exclude);
    if files.is_empty() {
        println!("warning: no Python or TypeScript files found.");
        std::process::exit(2);
    }

    use rayon::prelude::*;
    let to_process: Vec<_> = files.into_par_iter()
        .filter(|f| std::fs::read_to_string(f)
            .map(|src| detect::needs_docstrings(f, &src, args.force))
            .unwrap_or(false))
        .collect();

    if to_process.is_empty() {
        println!("nothing to do — all files already documented.");
        std::process::exit(2);
    }

    println!("Processing {} file(s) in batches of {}...", to_process.len(), cfg.batch_size);
    let patches = process::process_files(to_process, args.fmt.as_deref(), args.force, &model, &cfg).await;
    if patches.is_empty() {
        eprintln!("warning: no files were successfully processed.");
        std::process::exit(1);
    }

    println!("Applying changes via git branch...");
    if let Err(e) = apply::apply_with_git(patches, std::path::Path::new(".")) {
        eprintln!("error: git error: {e}");
        std::process::exit(1);
    }
    println!("Done. Docstrings applied.");
}
