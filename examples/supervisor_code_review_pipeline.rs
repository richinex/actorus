//! Real-World Supervisor Example: Automated Code Review Pipeline
//!
//! This demonstrates supervisor coordinating a complex, multi-step workflow:
//! 1. Fetch latest code from repository
//! 2. Run linter to check code quality
//! 3. Run tests to verify functionality
//! 4. Generate a summary report
//! 5. Save report to file
//!
//! This mimics a real CI/CD pipeline where multiple tools/agents work together.

#![allow(unused_variables)]

use anyhow::Result;
use actorus::tool_fn;
use actorus::{init, supervisor, AgentBuilder, AgentCollection};
use serde::{Deserialize, Serialize};

// ============================================================================
// Custom Tools - Git Operations
// ============================================================================

/// Clone or pull a git repository
#[tool_fn(
    name = "git_fetch",
    description = "Clone or pull the latest code from a git repository"
)]
async fn git_fetch(repo_url: String, branch: String) -> Result<String> {
    // Simulate git fetch
    Ok(format!(
        "Fetched latest code from {}\n\
         Branch: {}\n\
         Latest commit: abc123 - 'Fix bug in user authentication'\n\
         Files changed: src/auth.rs, tests/auth_tests.rs\n\
         Lines added: +45, removed: -12",
        repo_url, branch
    ))
}

/// Get repository statistics
#[tool_fn(
    name = "git_stats",
    description = "Get statistics about the repository"
)]
async fn git_stats() -> Result<String> {
    Ok(
        "Repository Statistics:\n\
         - Total commits: 1,247\n\
         - Contributors: 8\n\
         - Branches: 12\n\
         - Last commit: 2 hours ago"
            .to_string(),
    )
}

// ============================================================================
// Custom Tools - Code Analysis
// ============================================================================

/// Run linter on codebase
#[tool_fn(
    name = "run_linter",
    description = "Run linter to check code quality and style"
)]
async fn run_linter(directory: String) -> Result<String> {
    // Simulate linter results
    Ok(format!(
        "Linter Results for {}:\n\
         Files scanned: 47\n\
         Warnings found: 3\n\
         Errors found: 1\n\n\
         Issues:\n\
         1. [ERROR] src/auth.rs:45 - Unused variable 'temp_token'\n\
         2. [WARN] src/api.rs:120 - Function complexity too high (12 > 10)\n\
         3. [WARN] src/utils.rs:78 - Missing documentation comment\n\
         4. [WARN] tests/auth_tests.rs:34 - Consider using assert_eq! instead\n\n\
         Overall Score: 87/100 (Good)",
        directory
    ))
}

/// Run security scanner
#[tool_fn(
    name = "security_scan",
    description = "Scan code for security vulnerabilities"
)]
async fn security_scan(directory: String) -> Result<String> {
    Ok(
        "Security Scan Results:\n\
         No critical vulnerabilities found\n\
         1 medium severity issue:\n\
           - Hardcoded API endpoint in config.rs:23\n\
         Dependencies: All up-to-date\n\
         No known CVEs in dependencies\n\n\
         Security Score: 92/100 (Excellent)"
            .to_string(),
    )
}

// ============================================================================
// Custom Tools - Testing
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TestType {
    Unit,
    Integration,
    All,
}

/// Run test suite
#[tool_fn(
    name = "run_tests",
    description = "Run test suite (unit, integration, or all tests)"
)]
async fn run_tests(test_type: TestType) -> Result<String> {
    match test_type {
        TestType::Unit => Ok(
            "Unit Tests Results:\n\
             Running 134 tests...\n\
             132 passed\n\
             2 failed\n\n\
             Failures:\n\
             1. test_auth_token_validation - Expected true, got false\n\
             2. test_user_session_cleanup - Assertion failed at line 67\n\n\
             Test Coverage: 87%\n\
             Duration: 2.3s"
                .to_string(),
        ),
        TestType::Integration => Ok(
            "Integration Tests Results:\n\
             Running 24 tests...\n\
             24 passed\n\
             0 failed\n\n\
             All integration tests passing!\n\
             Duration: 8.7s"
                .to_string(),
        ),
        TestType::All => Ok(
            "Complete Test Suite Results:\n\
             Running 158 tests total...\n\
             156 passed (98.7%)\n\
             2 failed (1.3%)\n\n\
             Unit Tests: 132/134 passed\n\
             Integration Tests: 24/24 passed\n\n\
             Overall Coverage: 87%\n\
             Total Duration: 11.0s"
                .to_string(),
        ),
    }
}

/// Get code coverage report
#[tool_fn(
    name = "coverage_report",
    description = "Generate detailed code coverage report"
)]
async fn coverage_report() -> Result<String> {
    Ok(
        "Code Coverage Report:\n\
         \n\
         Module           Coverage    Lines\n\
         \n\
         src/auth.rs      94%         340/362\n\
         src/api.rs       89%         567/637\n\
         src/db.rs        91%         234/257\n\
         src/utils.rs     78%         145/186\n\
         src/config.rs    100%        89/89\n\
         \n\
         TOTAL            87%         1375/1531\n\
         \n\n\
         Uncovered lines:\n\
         - src/utils.rs: 45-52, 89-94, 123-127\n\
         - src/api.rs: 234-245, 456-467"
            .to_string(),
    )
}

// ============================================================================
// Custom Tools - Reporting
// ============================================================================

/// Generate comprehensive review report
#[tool_fn(
    name = "generate_report",
    description = "Generate a comprehensive code review report from all analysis results"
)]
async fn generate_report(
    git_info: String,
    linter_results: String,
    test_results: String,
    security_results: String,
) -> Result<String> {
    let report = format!(
        "\n\
                   AUTOMATED CODE REVIEW REPORT                        \n\
                   Generated: 2024-01-15 14:30:00 UTC                  \n\
         \n\n\
         REPOSITORY INFORMATION\n\
         \n\
         {}\n\n\
         CODE QUALITY ANALYSIS\n\
         \n\
         {}\n\n\
         TEST RESULTS\n\
         \n\
         {}\n\n\
         SECURITY ANALYSIS\n\
         \n\
         {}\n\n\
         OVERALL ASSESSMENT\n\
         \n\
         Code Quality:  87/100 (Good)\n\
         Test Coverage: 87% (Good)\n\
         Security:      92/100 (Excellent)\n\
         Action Items:  5 issues require attention\n\n\
         RECOMMENDATION: MERGE WITH CAUTION\n\
         - Fix 2 failing unit tests before deployment\n\
         - Address linter error in src/auth.rs:45\n\
         - Consider improving test coverage for src/utils.rs\n\n\
         \n\
           Review completed successfully - Report generated            \n\
         ",
        git_info, linter_results, test_results, security_results
    );

    Ok(report)
}

/// Save report to file
#[tool_fn(
    name = "save_report",
    description = "Save the generated report to a file"
)]
async fn save_report(filename: String, content: String) -> Result<String> {
    // Simulate file write
    Ok(format!(
        "Report saved successfully\n\
         Filename: {}\n\
         Size: {} bytes\n\
         Location: ./reports/{}\n\
         Timestamp: 2024-01-15 14:30:15 UTC",
        filename,
        content.len(),
        filename
    ))
}

// ============================================================================
// Custom Tools - Notifications
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum NotificationChannel {
    Slack,
    Email,
    Discord,
}

/// Send notification about review results
#[tool_fn(
    name = "send_notification",
    description = "Send notification about code review results to a channel"
)]
async fn send_notification(channel: NotificationChannel, message: String) -> Result<String> {
    match channel {
        NotificationChannel::Slack => Ok(format!(
            "Notification sent to Slack\n\
             Channel: #code-reviews\n\
             Message: {}\n\
             Mentions: @dev-team",
            message.chars().take(100).collect::<String>()
        )),
        NotificationChannel::Email => Ok(format!(
            "Email sent successfully\n\
             To: dev-team@company.com\n\
             Subject: Code Review Complete\n\
             Recipients: 8"
        )),
        NotificationChannel::Discord => Ok(format!(
            "Message posted to Discord\n\
             Server: Engineering Team\n\
             Channel: #deployments"
        )),
    }
}

// ============================================================================
// Main - Complex Real-World Pipeline
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    println!("\n");
    println!("   SUPERVISOR ORCHESTRATING AUTOMATED CODE REVIEW PIPELINE   ");
    println!("\n");

    init().await?;

    // ========================================================================
    // Build Specialized Agents for CI/CD Pipeline
    // ========================================================================

    // Git Agent - Handles repository operations
    let git_agent = AgentBuilder::new("git_agent")
        .description("Handles git repository operations - fetch, pull, clone, and stats")
        .system_prompt(
            "You are a git operations specialist. Use your tools to interact with git repositories. \
             Provide clear, concise information about repository state and changes.",
        )
        .tool(GitFetchTool::new())
        .tool(GitStatsTool::new());

    // Code Quality Agent - Runs linters and security scans
    let quality_agent = AgentBuilder::new("quality_agent")
        .description("Analyzes code quality using linters and security scanners")
        .system_prompt(
            "You are a code quality specialist. Run linters and security scans to assess code quality. \
             Provide detailed analysis of issues found and overall quality scores.",
        )
        .tool(RunLinterTool::new())
        .tool(SecurityScanTool::new());

    // Testing Agent - Runs test suites and generates coverage reports
    let testing_agent = AgentBuilder::new("testing_agent")
        .description("Runs test suites (unit, integration, all) and generates coverage reports")
        .system_prompt(
            "You are a testing specialist. Run tests and analyze results. \
             Report on test failures, coverage, and overall testing health.",
        )
        .tool(RunTestsTool::new())
        .tool(CoverageReportTool::new());

    // Reporting Agent - Generates and saves reports
    let reporting_agent = AgentBuilder::new("reporting_agent")
        .description("Generates comprehensive reports and saves them to files")
        .system_prompt(
            "You are a reporting specialist. Generate comprehensive reports from analysis data. \
             Save reports to files with clear organization and formatting.",
        )
        .tool(GenerateReportTool::new())
        .tool(SaveReportTool::new());

    // Notification Agent - Sends alerts and notifications
    let notification_agent = AgentBuilder::new("notification_agent")
        .description("Sends notifications via Slack, email, or Discord")
        .system_prompt(
            "You are a notification specialist. Send concise, actionable notifications \
             about code review results through various channels.",
        )
        .tool(SendNotificationTool::new());

    // Collect all agents
    let agents = AgentCollection::new()
        .add(git_agent)
        .add(quality_agent)
        .add(testing_agent)
        .add(reporting_agent)
        .add(notification_agent);

    println!("Created {} specialized agents for the pipeline:", agents.len());
    for (name, description) in agents.list_agents() {
        println!("   - {}: {}", name, description);
    }
    println!();

    let agent_configs = agents.build();

    // ========================================================================
    // Execute Complex Multi-Step Pipeline
    // ========================================================================

    println!("Starting automated code review pipeline...\n");

    let complex_task = "
        Execute a complete code review pipeline:

        1. Fetch the latest code from repository 'https://github.com/company/app' on branch 'main'
        2. Run the linter on the './src' directory to check code quality
        3. Run security scan on './src' directory
        4. Run all tests (unit and integration)
        5. Generate a comprehensive code review report using all the results from steps 1-4
        6. Save the report to a file named 'code_review_2024-01-15.txt'
        7. Send a notification to Slack about the review completion with a summary

        Make sure to:
        - Collect all results from each step
        - Use the results from previous steps when generating the final report
        - Include actionable recommendations in the notification
    ";

    let result = supervisor::orchestrate_custom_agents(agent_configs, complex_task).await?;

    println!("\n");
    println!("                    PIPELINE RESULTS                          ");
    println!("\n");

    println!("Success: {}", result.success);
    println!("\nFinal Result:\n{}\n", result.result);

    println!("Pipeline Execution Details:");
    println!("   - Total orchestration steps: {}", result.steps.len());
    println!("   - Agents coordinated: Multiple specialists");
    println!("   - Tasks completed: End-to-end code review\n");

    println!("Step-by-Step Breakdown:");
    for (i, step) in result.steps.iter().enumerate() {
        println!("\n   Step {}: {}", i + 1, step.thought);
        if let Some(action) = &step.action {
            // Parse agent:task format
            if let Some((agent, task)) = action.split_once(':') {
                println!("      Agent: {}", agent);
                println!("      Task: {}", task);
            } else {
                println!("      Action: {}", action);
            }
        }
        if let Some(obs) = &step.observation {
            let preview = if obs.len() > 150 {
                format!("{}...", &obs[..150])
            } else {
                obs.clone()
            };
            println!("      Result: {}", preview);
        }
    }

    println!("\n");
    println!("              KEY CONCEPTS DEMONSTRATED                       ");
    println!("\n");
    println!("1. Multi-Agent Coordination: 5 specialized agents working together");
    println!("2. Complex Workflow: 7-step pipeline with dependencies");
    println!("3. Data Flow: Results from each step feed into next steps");
    println!("4. Real-World Use Case: Automated CI/CD pipeline");
    println!("5. Go-Style Channels: Agents communicate via message passing");
    println!("6. No Shared Memory: Each agent isolated with own tools");
    println!("7. Return Ticket Pattern: Supervisor can invoke agents multiple times\n");

    println!("");
    println!("        AUTOMATED CODE REVIEW PIPELINE COMPLETE               ");
    println!("\n");

    Ok(())
}
