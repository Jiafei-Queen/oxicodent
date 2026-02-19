## Build & Run

```bash
cargo run --release
```

## Architecture Overview
The project employs a **multi-threaded architecture** with clear separation of concerns:
1. **Main Thread**: Handles UI rendering (crossterm) and event loop
2. **IO Thread**: Manages network I/O operations
3. **Worker Thread**: Executes background tasks
4. **Event Handler**: Processes system/keyboard events

Key components:
- `main.rs`: Entry point with thread management and TUI initializatio
- `io_thread.rs`: Handles asynchronous I/O operations
- `worker_thread.rs`: Processes background tasks
- `event_handler.rs`: Manages event routing and state updates
- `ui.rs`: Implements TUI rendering with crossterm
- `config_manager.rs`: Handles configuration persistence
- `api_client.rs`: Manages external API communications

### Module Structure
```
src/
├── app.rs            # Main application state management
├── api_client.rs     # Network API communication
├── config_manager.rs # Configuration file handling
├── event_handler.rs  # Event processing system
├── io_thread.rs      # Asynchronous I/O handling
├── main.rs           # Entry point and thread coordination
│── ui.rs             # TUI implementation with crossterm
└── worker_thread.rs  # Background task processing
```

## Project Features
- **M.A.G.I. Tripartite Architecture**: Separates architectural planning (Melchior), code implementation (Casper), and document research (Balthazar) into distinct modules with isolated contexts
- **Context Isolation Mechanism**: Uses CDSP protocol to split code modifications into "Evaluator/Planner" (logical decision) and "Generator/Surgeon" (physical implementation) phases
- **Implicit Tool Calling**: Leverages Markdown syntax (`exec`, `read`, `diff`) for shell commands and code patching without explicit API definitions
- **Threaded Architecture**: Separates UI, I/O, and worker tasks with clear coordination
- **TUI Support**: Uses `crossterm` for terminal interface with alternate screen management
- **Modular Design**: Clear separation of responsibilities across 7 core modules
- **Config Management**: Persistent configuration handling with file storage
- **Event-driven**: Comprehensive event processing system