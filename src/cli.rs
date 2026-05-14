use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "docgen", about = "Generate docstrings using a local vLLM instance.")]
pub struct Cli {
    pub target: PathBuf,
    #[arg(long = "format")]
    pub fmt: Option<String>,
    #[arg(long, short = 'r')]
    pub recursive: bool,
    #[arg(long)]
    pub force: bool,
}
