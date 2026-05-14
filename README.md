# docgen-rs

CLI tool that generates and inserts docstrings into TypeScript and Python source files using a locally-hosted [vLLM](https://github.com/vllm-project/vllm) instance. Designed for use during development or at build time.

The engine is written in **Rust**: source files are parsed with [tree-sitter](https://tree-sitter.github.io/) AST queries (no regex), LLM calls run in parallel with a tokio semaphore, and all file edits are isolated in a short-lived `docgen/<timestamp>` git branch that is merged back and deleted — keeping your working branch clean.

---

## Distribution wrappers

Install via your language ecosystem instead of building from source:

| Ecosystem | Package | Repo |
|-----------|---------|------|
| **npm / pnpm** | `@willisback/docgen` | [docgen-ts](https://github.com/WillIsback/docgen-ts) |
| **pip / uv** | `docgen-tool` | [docgen-python](https://github.com/WillIsback/docgen-python) |

Each wrapper downloads the correct pre-compiled binary for your platform from [GitHub Releases](https://github.com/WillIsback/docgen-rs/releases) automatically.

---

## Prerequisites

- A running [vLLM](https://github.com/vllm-project/vllm) server accessible from your machine or CI runner
- Model is auto-detected via `GET /v1/models` — no manual configuration needed
- Set `VLLM_MODEL` to skip auto-detection and use a specific model ID

---

## Building from source

```bash
git clone https://github.com/WillIsback/docgen-rs.git
cd docgen-rs
cargo build --release
# binary at: target/release/docgen
```

---

## Configuration

`docgen` loads `.env` files in two layers (project overrides global):

### Global config

Create `~/.config/docgen/.env` once — applies to every project on the machine:

```bash
mkdir -p ~/.config/docgen
cat > ~/.config/docgen/.env <<EOF
VLLM_BASE_URL=http://<your-host>:30000/v1
BATCH_SIZE=4
EOF
```

### Project-level config

Create a `.env` at the root of any project to override global values:

```
VLLM_BASE_URL=http://<your-host>:30000/v1
BATCH_SIZE=4
```

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `VLLM_BASE_URL` | `http://localhost:30000/v1` | vLLM server base URL |
| `BATCH_SIZE` | `4` | Max concurrent LLM requests |
| `VLLM_MODEL` | _(auto)_ | Override model ID; auto-detected from `/v1/models` if unset |

---

## Usage

```bash
# Single file
docgen path/to/file.ts

# Flat folder (top-level files only)
docgen src/

# Recurse into subdirectories
docgen src/ --recursive

# Regenerate existing docstrings
docgen src/ --force

# Explicit format override
docgen src/ --format tsdoc
```

## Options

| Flag | Default | Description |
|---|---|---|
| `target` | — | File or folder to process (required) |
| `--format` | auto | Docstring format: `mkdocs` (Python) or `tsdoc` (TypeScript/TSX) |
| `--recursive` / `-r` | off | Recurse into subdirectories when target is a folder |
| `--force` | off | Regenerate docstrings even if they already exist |

## Format auto-detection

| Extension | Default format |
|---|---|
| `.py` | `mkdocs` |
| `.ts`, `.tsx` | `tsdoc` |

---

## How it works

1. **Pre-flight checks** — aborts if the working tree has uncommitted changes, or if vLLM is unreachable
2. **Model detection** — uses `VLLM_MODEL` env var if set; otherwise queries `GET /v1/models`
3. **File resolution** — collects `.py` / `.ts` / `.tsx` files under the target (flat or recursive)
4. **AST parsing** — Python files are parsed with `tree-sitter-python`; TypeScript with `tree-sitter-typescript`. Files with all functions/classes documented are skipped (unless `--force`). Parsing runs in parallel via `rayon`.
5. **Batch LLM calls** — files are sent to vLLM concurrently, up to `BATCH_SIZE` in-flight requests via a `tokio` semaphore
6. **Git workflow** — creates a `docgen/<timestamp>` branch, writes patched files, commits, merges back into the original branch, deletes the feature branch. On failure the branch is left intact for manual inspection.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Pre-flight failure (dirty tree, vLLM unreachable) or git error |
| `2` | Nothing to do (no files found or all already documented) |

---

## Project structure

```
Cargo.toml          # Single crate
src/
├── main.rs         # Orchestration
├── cli.rs          # Clap argument definitions
├── resolver.rs     # File discovery
├── detect.rs       # tree-sitter AST detection
├── process.rs      # Parallel LLM calls
├── apply.rs        # Git branch/merge workflow
├── config.rs       # .env loading
├── llm.rs          # vLLM client
├── git.rs          # Git helpers
└── error.rs        # Error types
```

## License

MIT
