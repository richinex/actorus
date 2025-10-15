# Session-Based Context Management

## Overview

LLM Fusion now supports **persistent multi-turn conversations** with agents. Sessions maintain conversation context across multiple tasks, allowing the agent to remember previous interactions.

## Key Features

- **Persistent Context**: Agent remembers previous messages and can reference them
- **Swappable Storage**: Choose between in-memory, file system, SQLite, or Redis backends
- **Information Hiding**: Storage implementation completely hidden from users
- **Automatic Persistence**: Conversation history saved automatically after each message
- **Multi-User Support**: Each session has a unique ID for isolation

## Architecture

```
User
  │
  ├─> Session (session_id: "user-123")
  │     ├─> ConversationStorage (trait)
  │     │     ├─> InMemoryStorage
  │     │     ├─> FileSystemStorage
  │     │     ├─> SqliteStorage (future)
  │     │     └─> RedisStorage (future)
  │     │
  │     └─> AgentSession
  │           ├─> Conversation History (Vec<ChatMessage>)
  │           ├─> LLM Client
  │           ├─> Tool Registry
  │           └─> Tool Executor
  │
  └─> Multiple messages in same session
        ├─> "Create file.txt"
        └─> "Now read the file" (remembers previous context)
```

## Storage Backends

### Current Implementations

#### 1. In-Memory Storage
- **Use Case**: Testing, temporary sessions, single-instance apps
- **Persistence**: Lost on process termination
- **Dependencies**: None
- **Thread-Safe**: Yes (uses RwLock)

```rust
use llm_fusion::api::session::{self, StorageType};

let mut session = session::create_session(
    "user-123",
    StorageType::Memory,
).await?;
```

#### 2. File System Storage
- **Use Case**: Simple persistence, single-instance apps, easy debugging
- **Persistence**: JSON files on disk
- **Dependencies**: None (uses tokio::fs)
- **Format**: Human-readable JSON (one file per session)

```rust
use llm_fusion::api::session::{self, StorageType};
use std::path::PathBuf;

let mut session = session::create_session(
    "user-123",
    StorageType::FileSystem(PathBuf::from("./sessions")),
).await?;
```

### Future Implementations

#### 3. SQLite Storage (Coming Soon)
- **Use Case**: Structured queries, embedded database
- **Benefits**: SQL queries, transactions, migrations
- **Dependency**: `sqlx` with `sqlite` feature

#### 4. Redis Storage (Coming Soon)
- **Use Case**: Distributed systems, fast access, multi-instance apps
- **Benefits**: High performance, TTL support, pub/sub
- **Dependency**: `redis` crate

## API Reference

### Creating a Session

```rust
use llm_fusion::api::session::{self, StorageType};

// In-memory (ephemeral)
let mut session = session::create_session("session-id", StorageType::Memory).await?;

// File system (persistent)
let mut session = session::create_session(
    "session-id",
    StorageType::FileSystem(PathBuf::from("./sessions")),
).await?;
```

### Sending Messages

```rust
// Send a message (default 10 iterations max)
let result = session.send_message("Create a file called test.txt").await?;

if result.success {
    println!("Result: {}", result.result);
    println!("Steps taken: {}", result.steps.len());
}

// Send with custom max iterations
let result = session.send_message_with_iterations("Complex task", 20).await?;
```

### Managing History

```rust
// Get session info
println!("Session ID: {}", session.session_id());
println!("Message count: {}", session.message_count());

// Clear conversation history
session.clear_history().await?;
```

## Usage Examples

### Example 1: Basic Multi-Turn Conversation

```rust
use llm_fusion::api::session::{self, StorageType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut session = session::create_session("user-123", StorageType::Memory).await?;

    // First task
    let result = session.send_message("What files are in /tmp?").await?;
    println!("Files: {}", result.result);

    // Second task remembers context from first
    let result = session.send_message("Delete the .txt files you just found").await?;
    println!("Deleted: {}", result.result);

    Ok(())
}
```

### Example 2: Persistent Session

```rust
use llm_fusion::api::session::{self, StorageType};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut session = session::create_session(
        "persistent-user",
        StorageType::FileSystem(PathBuf::from("./sessions")),
    ).await?;

    if session.message_count() == 0 {
        // First run
        session.send_message("Remember: my favorite color is blue").await?;
        println!("Information stored. Run this program again!");
    } else {
        // Subsequent runs
        let result = session.send_message("What is my favorite color?").await?;
        println!("Agent recalled: {}", result.result);
    }

    Ok(())
}
```

### Example 3: Interactive REPL

See `examples/interactive_session.rs` for a full interactive REPL implementation with commands:
- `/clear` - Clear conversation history
- `/history` - Show message count
- `/exit` - Exit the session
- `/help` - Show help

Run with:
```bash
RUST_LOG=info cargo run --example interactive_session
```

## How It Works

### Conversation Flow

1. **Session Creation**
   - Load existing history from storage (if exists)
   - Initialize LLM client, tools, and executor

2. **Message Handling**
   ```rust
   send_message() -> {
       1. Add user message to history
       2. Run ReAct loop with full history
       3. Agent sees ALL previous messages
       4. Generate response using context
       5. Persist updated history to storage
       6. Return result
   }
   ```

3. **Context Preservation**
   - System prompt added once (first message)
   - All subsequent messages appended to history
   - Agent decisions include conversation context
   - Tool results preserved in history

### Storage Trait

```rust
#[async_trait]
pub trait ConversationStorage: Send + Sync {
    async fn save(&self, session_id: &str, history: &[ChatMessage]) -> Result<()>;
    async fn load(&self, session_id: &str) -> Result<Vec<ChatMessage>>;
    async fn delete(&self, session_id: &str) -> Result<()>;
    async fn list_sessions(&self) -> Result<Vec<String>>;
}
```

This abstraction allows swapping storage backends without changing client code.

## Design Principles

### Information Hiding (Parnas Principles)

Each component hides implementation details:

1. **ConversationStorage Trait**
   - Hides: Storage format, file structure, database schema
   - Exposes: Simple save/load/delete interface

2. **AgentSession**
   - Hides: ReAct loop, tool execution, prompt engineering
   - Exposes: send_message(), clear_history()

3. **Storage Implementations**
   - InMemoryStorage hides: HashMap, RwLock
   - FileSystemStorage hides: JSON serialization, file I/O

### Benefits

- **Changeability**: Swap storage backends with one line
- **Testability**: Use InMemoryStorage for tests
- **Scalability**: Start with FileSystem, move to Redis for distributed systems
- **Maintainability**: Each module can be understood in isolation

## Performance Considerations

### Message Count Growth

As conversation history grows, token usage increases:

```
Turn 1:  System + User₁ + Agent₁
Turn 2:  System + User₁ + Agent₁ + User₂ + Agent₂
Turn 3:  System + User₁ + Agent₁ + User₂ + Agent₂ + User₃ + Agent₃
...
```

**Strategies:**
1. **Periodic Clear**: Call `clear_history()` to reset
2. **Summarization**: Summarize old messages (future feature)
3. **Sliding Window**: Keep only last N messages (future feature)
4. **Context Pruning**: Remove less important messages (future feature)

### Storage Performance

| Backend | Read | Write | Scalability | Use Case |
|---------|------|-------|-------------|----------|
| Memory | O(1) | O(1) | Single instance | Development, testing |
| FileSystem | O(n) | O(n) | Single instance | Simple apps, debugging |
| SQLite | O(log n) | O(log n) | Single instance | Structured queries |
| Redis | O(1) | O(1) | Multi-instance | Production, distributed |

## Running Examples

### Basic Session Usage
```bash
RUST_LOG=info cargo run --example session_usage
```

### Interactive REPL
```bash
RUST_LOG=info cargo run --example interactive_session
```

## Testing

All storage backends have comprehensive tests:

```bash
# Run all tests
cargo test --all

# Run storage tests only
cargo test --lib storage

# Test memory storage
cargo test --lib storage::memory

# Test filesystem storage
cargo test --lib storage::filesystem
```

Test coverage:
- ✅ Save and load
- ✅ Load nonexistent session (returns empty)
- ✅ Delete session
- ✅ List sessions
- ✅ Directory creation (FileSystem)

## File Structure

```
src/
  storage/
    mod.rs           - ConversationStorage trait
    memory.rs        - InMemoryStorage implementation
    filesystem.rs    - FileSystemStorage implementation
  session.rs         - AgentSession implementation
  api.rs             - Public session API

examples/
  session_usage.rs         - Basic usage examples
  interactive_session.rs   - Interactive REPL

tests/
  (storage tests in src/storage/*.rs)
```

## Next Steps

### Planned Features

1. **SQLite Backend**
   ```rust
   StorageType::Sqlite("sqlite://sessions.db".to_string())
   ```

2. **Redis Backend**
   ```rust
   StorageType::Redis("redis://localhost:6379".to_string())
   ```

3. **Conversation Summarization**
   ```rust
   session.summarize_history(last_n_messages: 10).await?;
   ```

4. **Sliding Window**
   ```rust
   session.set_max_history_length(50);
   ```

5. **Context Pruning**
   ```rust
   session.prune_unimportant_messages().await?;
   ```

6. **Session Management**
   ```rust
   session::list_all_sessions(storage).await?;
   session::delete_session("session-id", storage).await?;
   ```

## Summary

Session-based context management enables:
- ✅ Multi-turn conversations
- ✅ Persistent memory across runs
- ✅ Swappable storage backends
- ✅ Information hiding architecture
- ✅ Comprehensive testing
- ✅ Interactive REPL mode

The system is production-ready with two storage backends (Memory and FileSystem) and an extensible architecture for adding more (SQLite, Redis, etc.).
