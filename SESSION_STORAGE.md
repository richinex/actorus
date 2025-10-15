# Session Storage Documentation

## Overview

The session storage system provides persistent multi-turn conversations with agents. Sessions maintain conversation context across multiple messages, allowing agents to remember previous interactions.

## Architecture

The storage system follows information hiding principles with a trait-based abstraction:

```
ConversationStorage Trait (interface)
├── InMemoryStorage (ephemeral)
├── FileSystemStorage (persistent)
├── SqliteStorage (optional, future)
└── RedisStorage (optional, future)
```

### Information Hiding Benefits

- **Storage backend swappable** - Change from in-memory to filesystem to database without API changes
- **Implementation details hidden** - Internal data structures and protocols hidden from users
- **Easy testing** - Use InMemoryStorage for tests, FileSystemStorage for production
- **Future extensibility** - Add SQLite or Redis without breaking existing code

## Storage Backends

### InMemoryStorage

**Use cases:**
- Testing
- Ephemeral sessions
- Single-process applications

**Characteristics:**
- Fast (no I/O)
- Data lost on process termination
- Thread-safe with RwLock
- Zero dependencies

**Example:**
```rust
use llm_fusion::api::session::{self, StorageType};

let mut session = session::create_session(
    "user-123",
    StorageType::Memory,
).await?;
```

### FileSystemStorage

**Use cases:**
- Persistent sessions
- Simple deployment
- Development and small-scale production

**Characteristics:**
- Sessions stored as JSON files
- One file per session: `{session_id}.json`
- Survives process restart
- Human-readable format

**Example:**
```rust
use llm_fusion::api::session::{self, StorageType};
use std::path::PathBuf;

let mut session = session::create_session(
    "user-123",
    StorageType::FileSystem(PathBuf::from("./sessions")),
).await?;
```

**File structure:**
```
./sessions/
├── user-123.json
├── user-456.json
└── session-abc.json
```

## Usage Patterns

### Pattern 1: Ephemeral Session

For temporary conversations that don't need persistence:

```rust
use llm_fusion::api::session::{self, StorageType};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut session = session::create_session("temp", StorageType::Memory).await?;

    let result = session.send_message("What is Rust?").await?;
    println!("{}", result.result);

    let result = session.send_message("Tell me more about ownership").await?;
    println!("{}", result.result);

    Ok(())
}
```

### Pattern 2: Persistent Session

For conversations that persist across application restarts:

```rust
use llm_fusion::api::session::{self, StorageType};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let storage = StorageType::FileSystem(PathBuf::from("./sessions"));

    // First run - create session
    {
        let mut session = session::create_session("user-123", storage.clone()).await?;
        session.send_message("Remember: my name is Alice").await?;
    }

    // Later run - session remembers previous context
    {
        let mut session = session::create_session("user-123", storage).await?;
        let result = session.send_message("What is my name?").await?;
        println!("{}", result.result); // "Your name is Alice"
    }

    Ok(())
}
```

### Pattern 3: Session Management

Managing multiple sessions and their lifecycle:

```rust
use llm_fusion::api::session::{self, StorageType};

async fn handle_user_request(user_id: &str, message: &str) -> anyhow::Result<String> {
    let mut session = session::create_session(
        user_id,
        StorageType::FileSystem(PathBuf::from("./sessions")),
    ).await?;

    let result = session.send_message(message).await?;
    Ok(result.result)
}

// Clear a user's history
async fn clear_user_history(user_id: &str) -> anyhow::Result<()> {
    let mut session = session::create_session(
        user_id,
        StorageType::FileSystem(PathBuf::from("./sessions")),
    ).await?;

    session.clear_history().await?;
    Ok(())
}
```

### Pattern 4: Interactive REPL

Building an interactive command-line interface:

```rust
use llm_fusion::api::session::{self, StorageType};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut session = session::create_session("repl", StorageType::Memory).await?;

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "/exit" => break,
            "/clear" => session.clear_history().await?,
            "/count" => println!("Messages: {}", session.message_count()),
            message => {
                let result = session.send_message(message).await?;
                println!("{}\n", result.result);
            }
        }
    }

    Ok(())
}
```

## Session API Reference

### Creating Sessions

```rust
pub async fn create_session(
    session_id: impl Into<String>,
    storage_type: StorageType,
) -> Result<Session>
```

Creates a new session or loads an existing one if found in storage.

**Parameters:**
- `session_id` - Unique identifier for the session
- `storage_type` - Storage backend (Memory or FileSystem)

**Returns:**
- `Session` - Session handle for multi-turn conversation

### Session Methods

#### send_message

```rust
pub async fn send_message(&mut self, message: &str) -> Result<AgentResult>
```

Send a message to the agent. Conversation history is automatically maintained.

**Parameters:**
- `message` - User message or task description

**Returns:**
- `AgentResult` - Contains result, steps, and success status

#### send_message_with_iterations

```rust
pub async fn send_message_with_iterations(
    &mut self,
    message: &str,
    max_iterations: usize,
) -> Result<AgentResult>
```

Send a message with custom maximum iterations for the ReAct loop.

**Parameters:**
- `message` - User message or task description
- `max_iterations` - Maximum ReAct iterations

**Returns:**
- `AgentResult` - Contains result, steps, and success status

#### clear_history

```rust
pub async fn clear_history(&mut self) -> Result<()>
```

Clear conversation history for this session. Removes from storage as well.

#### session_id

```rust
pub fn session_id(&self) -> &str
```

Get the session ID.

**Returns:**
- Session identifier string

#### message_count

```rust
pub fn message_count(&self) -> usize
```

Get the number of messages in conversation history.

**Returns:**
- Count of messages (user + assistant + system)

## Conversation Format

Sessions store conversation history as a sequence of `ChatMessage` objects:

```json
[
  {
    "role": "system",
    "content": "You are an autonomous agent..."
  },
  {
    "role": "user",
    "content": "What files are in /tmp?"
  },
  {
    "role": "assistant",
    "content": "{\"thought\": \"I need to list files\", ...}"
  },
  {
    "role": "user",
    "content": "Observation: file1.txt, file2.txt..."
  }
]
```

**Roles:**
- `system` - System prompt with tool descriptions
- `user` - User messages and tool observations
- `assistant` - Agent decisions and actions

## Implementation Details

### ConversationStorage Trait

```rust
#[async_trait]
pub trait ConversationStorage: Send + Sync {
    async fn save(&self, session_id: &str, history: &[ChatMessage]) -> Result<()>;
    async fn load(&self, session_id: &str) -> Result<Vec<ChatMessage>>;
    async fn delete(&self, session_id: &str) -> Result<()>;
    async fn list_sessions(&self) -> Result<Vec<String>>;
    async fn exists(&self, session_id: &str) -> Result<bool>;
}
```

All storage backends implement this trait, ensuring consistent behavior.

### InMemoryStorage Implementation

```rust
pub struct InMemoryStorage {
    sessions: Arc<RwLock<HashMap<String, Vec<ChatMessage>>>>,
}
```

Uses a thread-safe HashMap with RwLock for concurrent access.

### FileSystemStorage Implementation

```rust
pub struct FileSystemStorage {
    base_path: PathBuf,
}
```

Stores sessions as `{session_id}.json` files in the base directory.

## Future Extensions

### SQLite Storage (Future)

For structured queries and multi-instance deployments:

```rust
// Future API
let storage = StorageType::Sqlite("sqlite://sessions.db".to_string());
let mut session = session::create_session("user-123", storage).await?;
```

**Benefits:**
- Transaction support
- Query conversation history
- Better concurrency control
- Atomic operations

### Redis Storage (Future)

For distributed applications and caching:

```rust
// Future API
let storage = StorageType::Redis("redis://localhost:6379".to_string());
let mut session = session::create_session("user-123", storage).await?;
```

**Benefits:**
- Distributed caching
- Multiple application instances
- Fast access
- TTL support for automatic cleanup

## Examples

See the following example files:

- `examples/session_usage.rs` - Basic session usage patterns
- `examples/interactive_session.rs` - Interactive REPL with sessions

Run examples:
```bash
# Basic usage examples
cargo run --example session_usage

# Interactive REPL
cargo run --example interactive_session
```

## Testing

Storage backends include comprehensive tests:

```bash
# Run storage tests
cargo test storage::

# Run all tests
cargo test --all
```

Current test coverage:
- InMemoryStorage: 5 tests
- FileSystemStorage: 5 tests
- Total storage tests: 10 tests

## Best Practices

1. **Choose appropriate storage**
   - Use Memory for testing and ephemeral sessions
   - Use FileSystem for persistence in single-instance deployments
   - Use SQLite/Redis (future) for production multi-instance deployments

2. **Session ID management**
   - Use user IDs for per-user sessions
   - Use UUIDs for anonymous sessions
   - Include context in session IDs when managing multiple types

3. **History management**
   - Clear history when starting new topics
   - Monitor message count to avoid excessive context
   - Implement session timeout/cleanup for unused sessions

4. **Error handling**
   - Handle storage errors gracefully
   - Implement retry logic for transient failures
   - Log storage operations for debugging

5. **Security considerations**
   - Sanitize session IDs to prevent path traversal
   - Encrypt sensitive conversation data at rest
   - Implement access control for session data
   - Set appropriate file permissions for FileSystemStorage

## Migration Path

When moving from one storage backend to another:

1. Export sessions from old backend
2. Transform to new format if needed
3. Import to new backend
4. Update application configuration
5. Deploy with new storage type

Example migration from Memory to FileSystem:

```rust
// Not implemented yet, but conceptually:
async fn migrate_sessions() -> anyhow::Result<()> {
    let old_storage = Arc::new(InMemoryStorage::new());
    let new_storage = Arc::new(FileSystemStorage::new(PathBuf::from("./sessions")).await?);

    for session_id in old_storage.list_sessions().await? {
        let history = old_storage.load(&session_id).await?;
        new_storage.save(&session_id, &history).await?;
    }

    Ok(())
}
```
