# Oxicodent Assistant MD

> ⚠️ **MELCHIOR 维护区**：此文件由 MELCHIOR 模块维护，动态区块请勿手动编辑
> Last Updated: 2026-02-17 | Stage: Initialization

---

## Build & Run

```bash
# Build with mach.lua (injects prompts into config_manager.rs)
./mach.lua

# Or standard cargo commands (prompts will be placeholders)
cargo build
cargo run

# Run tests
cargo test
```

## Architecture Overview

Oxicodent is a TUI-based AI coding assistant using the **M.A.G.I. three-module architecture**:

### Module Structure

```
src/
├── main.rs           # Entry point, orchestrates all components
├── app.rs            # Core types: AppMessage, Model enum, Tool enum
├── config_manager.rs # Config loading, prompt injection via mach.lua
├── api_client.rs     # HTTP client for LLM API (streaming SSE)
├── ui.rs             # Ratatui TUI rendering
├── event_handler.rs  # Keyboard input handling
├── io_thread.rs      # Network I/O thread, manages conversation history per model
└── worker_thread.rs  # Command execution, file read, diff/patch application
```

### Threading Model

- **UI Thread (main)**: TUI rendering, event loop
- **IO Thread**: Blocking API calls, maintains separate conversation histories for MELCHIOR/CASPER_I/CASPER_II/BALTHAZAR
- **Worker Thread**: Executes shell commands, file operations, patch application

### M.A.G.I. Architecture

| Module | Role | Model Config |
|--------|------|--------------|
| MELCHIOR | Chief architect - discusses design with user | `melchior_model` |
| CASPER I | Senior dev - generates implementation spec | `casper_model` |
| CASPER II | Code surgeon - generates atomic diffs | `casper_model` |
| BALTHAZAR | Research assistant - web search & docs | `balthazar_model` |

Prompts are injected at build time via `mach.lua` into `config_manager.rs`.

### Tool Calling Protocol

Uses implicit Markdown code blocks instead of JSON Schema:

- `exec` - Execute shell commands
- `read:<filename>` - Read file with line numbers
- `diff:<filename>` - Apply unified diff patch
- `search` - Web search (via API)

### Key Design Patterns

- **Message passing**: `AppMessage` enum flows between threads via `mpsc` channels
- **Model state**: `CURRENT_MODEL` singleton tracks active module
- **Pending actions**: `ConfirmExec`/`ConfirmDiff` require user confirmation before execution
- **Path safety**: `resolve_safe_path()` prevents directory traversal attacks

### Configuration

Config stored at `~/.oxicodent/config.json`:
- `api_key`, `api_base`
- `melchior_model`, `casper_model`, `balthazar_model`
