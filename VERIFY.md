# Verification Guide - How to Know Everything Works

## Quick Verification (No API Key Required)

### 1. Run Unit Tests
```bash
cargo test --lib
```

**Expected Output:**
```
test result: ok. 13 passed; 0 failed; 1 ignored
```

These tests verify:
- ✅ Shell command execution
- ✅ File system operations (read/write)
- ✅ HTTP tool validation
- ✅ Tool registry functionality
- ✅ Tool executor with retry logic

### 2. Run Integration Tests
```bash
cargo test --test integration_test
```

**Expected Output:**
```
test result: ok. 10 passed; 0 failed; 0 ignored
```

These tests verify:
- ✅ Tool registry initialization
- ✅ Shell tool whitelisting
- ✅ File system size limits
- ✅ HTTP domain filtering
- ✅ Executor retry with backoff
- ✅ Complete write-read cycle

### 3. Check Build
```bash
cargo build --release
cargo clippy --all-targets
```

**Expected Output:**
```
Finished `release` profile [optimized]
```
No errors, only minor naming convention warnings (acceptable).

## Full System Test (Requires OpenAI API Key)

### 1. Set API Key
```bash
export OPENAI_API_KEY=your_key_here
export RUST_LOG=info
```

### 2. Run Comprehensive Test
```bash
cargo run --example test_agent
```

**This will test:**
```
✅ Test 1: System Initialization
✅ Test 2: File Write Task
✅ Test 3: File Read Task
✅ Test 4: Shell Command Execution
✅ Test 5: Multi-Step Reasoning
✅ Test 6: Iteration Control
✅ Test 7: Error Handling
✅ Test 8: Agent Lifecycle - Stop
✅ Test 9: System Shutdown
```

**Expected Output:** A detailed report showing each test passing with green checkmarks.

### 3. Run Agent Usage Example
```bash
cargo run --example agent_usage
```

**This demonstrates:**
- Creating and reading files autonomously
- Executing shell commands
- Multi-step tasks with reasoning
- Complete agent thought process

## What Each Test Verifies

### Unit Tests (No API Key)
| Test | Verifies |
|------|----------|
| `test_shell_tool_success` | Shell commands execute correctly |
| `test_shell_tool_whitelist` | Command filtering works |
| `test_read_file_success` | File reading works |
| `test_write_file_success` | File writing works |
| `test_file_size_limit` | Size quotas enforced |
| `test_http_domain_whitelist` | Domain filtering works |
| `test_registry_*` | Tool registry manages tools |
| `test_executor_retry_*` | Retry logic with backoff |

### Integration Tests (No API Key)
| Test | Verifies |
|------|----------|
| `test_tool_registry_initialization` | All default tools load |
| `test_filesystem_write_and_read` | Complete file I/O cycle |
| `test_shell_tool_execution` | Commands produce output |
| `test_tool_executor_backoff` | Exponential backoff works |
| `test_tool_metadata` | Tool descriptions available |

### System Tests (Requires API Key)
| Test | Verifies |
|------|----------|
| System Init | Actor system starts |
| File Write | Agent creates files |
| File Read | Agent reads files |
| Shell Execution | Agent runs commands |
| Multi-Step | Agent chains operations |
| Iteration Control | Max iterations enforced |
| Error Handling | Graceful failure |
| Agent Stop | Graceful shutdown |
| System Shutdown | Clean termination |

## Verification Checklist

- [ ] Unit tests pass (13 tests)
- [ ] Integration tests pass (10 tests)
- [ ] Build completes without errors
- [ ] Clippy shows only style warnings
- [ ] Test agent example runs successfully (with API key)
- [ ] Agent usage example demonstrates features (with API key)

## Manual Verification

You can also test manually:

```rust
use llm_fusion::{init, agent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init().await?;

    let result = agent::run_task("echo 'Hello, World!'").await?;

    println!("Success: {}", result.success);
    println!("Result: {}", result.result);
    println!("Steps: {}", result.steps.len());

    Ok(())
}
```

## Troubleshooting

### Tests Fail
- Check Rust version: `rustc --version` (should be 1.70+)
- Clean build: `cargo clean && cargo build`
- Check dependencies: `cargo update`

### API Key Tests Fail
- Verify API key is set: `echo $OPENAI_API_KEY`
- Check API key is valid (try with curl)
- Check internet connectivity
- Review logs: `RUST_LOG=debug cargo run --example test_agent`

### Build Errors
- Update Rust: `rustup update`
- Check Cargo.toml for correct dependencies
- Ensure all files are present

## Success Indicators

### You know it's working when:

1. **All tests pass**
   ```
   cargo test --all
   # Should show 23+ tests passed
   ```

2. **Agent creates files**
   ```bash
   cargo run --example test_agent
   # Creates and verifies multiple test files
   ```

3. **Agent shows reasoning**
   - You see iteration logs
   - Tool execution messages
   - Successful task completion

4. **No warnings in build**
   ```bash
   cargo build --release
   # Completes without errors
   ```

5. **Clean shutdown**
   - No panic messages
   - Graceful actor termination
   - Clean file cleanup

## Performance Indicators

A working system should:
- Initialize in < 1 second
- Complete simple tasks in 2-5 seconds (depending on LLM API)
- Handle 10+ concurrent tasks
- Recover from failures automatically
- Show clear logs at each step

## Architecture Verification

Verify the actor system:
```bash
RUST_LOG=debug cargo run --example test_agent 2>&1 | grep -E "actor|Actor"
```

You should see:
- Router actor started
- LLM actor started
- MCP actor started
- Agent actor started
- Supervisor actor started
- Heartbeat messages
- Clean shutdown messages

## Summary

**Without API Key:**
```bash
cargo test --all
```
Should pass 23+ tests

**With API Key:**
```bash
export OPENAI_API_KEY=your_key
cargo run --example test_agent
```
Should complete all 9 test scenarios successfully

If both work, your autonomous agent system is **fully operational**!
