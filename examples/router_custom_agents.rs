//! Router with Custom Specialized Agents
//!
//! Demonstrates the router pattern with custom macro-defined tools.
//! Shows how to create custom agents with AgentBuilder and use router for intent-based routing.
//!
//! This example shows:
//! - Creating custom tools with #[tool_fn] macro
//! - Building specialized agents with AgentBuilder
//! - Using router to classify intent and route to appropriate custom agent
//! - LLM-based intelligent routing to the right specialist
//! - "One-way ticket" pattern - router selects ONE agent to handle entire task

#![allow(unused_variables)]

use actorus::tool_fn;
use actorus::{init, router, AgentBuilder, AgentCollection};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

// ============================================================================
// Custom Tools - Weather Service
// ============================================================================

/// Get current weather for a location
#[tool_fn(
    name = "get_weather",
    description = "Get current weather conditions for a specific location"
)]
async fn get_weather(location: String, units: Option<String>) -> Result<String> {
    let unit_str = units.unwrap_or_else(|| "celsius".to_string());

    // Simulate weather API call
    Ok(format!(
        "Current weather in {}: 22°{}, Partly Cloudy, Humidity 65%",
        location,
        if unit_str == "fahrenheit" { "F" } else { "C" }
    ))
}

/// Get weather forecast
#[tool_fn(
    name = "get_forecast",
    description = "Get weather forecast for the next N days"
)]
async fn get_forecast(location: String, days: i64) -> Result<String> {
    Ok(format!(
        "{}-day forecast for {}: Mostly sunny, temperatures 20-25°C",
        days, location
    ))
}

// ============================================================================
// Custom Tools - Calendar Service
// ============================================================================

/// Schedule an event
#[tool_fn(
    name = "schedule_event",
    description = "Schedule a new event on the calendar"
)]
async fn schedule_event(title: String, date: String, time: String) -> Result<String> {
    Ok(format!(
        "Event '{}' scheduled for {} at {}",
        title, date, time
    ))
}

/// List upcoming events
#[tool_fn(name = "list_events", description = "List all upcoming events")]
async fn list_events(days_ahead: i64) -> Result<String> {
    Ok(format!(
        "Upcoming events in next {} days:\n1. Team Meeting - Tomorrow 10:00 AM\n2. Project Review - Friday 2:00 PM",
        days_ahead
    ))
}

// ============================================================================
// Custom Tools - Email Service
// ============================================================================

/// Send an email
#[tool_fn(name = "send_email", description = "Send an email to a recipient")]
async fn send_email(to: String, subject: String, body: String) -> Result<String> {
    Ok(format!("Email sent to {} with subject '{}'", to, subject))
}

/// Search emails
#[tool_fn(
    name = "search_emails",
    description = "Search for emails by keyword or sender"
)]
async fn search_emails(query: String, folder: Option<String>) -> Result<String> {
    let folder_str = folder.unwrap_or_else(|| "inbox".to_string());
    Ok(format!(
        "Found 3 emails matching '{}' in {}:\n1. Re: Project Update\n2. Meeting Notes\n3. Q4 Report",
        query, folder_str
    ))
}

// ============================================================================
// Custom Tools - File Operations
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileOperation {
    Create,
    Read,
    Update,
    Delete,
}

/// Perform file operation
#[tool_fn(
    name = "file_operation",
    description = "Perform file operations: create, read, update, delete"
)]
async fn file_operation(
    operation: FileOperation,
    filename: String,
    content: Option<String>,
) -> Result<String> {
    match operation {
        FileOperation::Create => Ok(format!(
            "Created file '{}' with content: {}",
            filename,
            content.unwrap_or_default()
        )),
        FileOperation::Read => Ok(format!(
            "Reading file '{}':\nFile contents here...",
            filename
        )),
        FileOperation::Update => Ok(format!("Updated file '{}' with new content", filename)),
        FileOperation::Delete => Ok(format!("Deleted file '{}'", filename)),
    }
}

/// List files in directory
#[tool_fn(name = "list_files", description = "List all files in a directory")]
async fn list_files(directory: String) -> Result<String> {
    Ok(format!(
        "Files in '{}':\n- document.txt\n- report.pdf\n- notes.md",
        directory
    ))
}

// ============================================================================
// Main - Router with Custom Agents
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    println!("\n=== Router with Custom Specialized Agents ===\n");

    init().await?;

    // ========================================================================
    // Build Custom Specialized Agents - Using AgentBuilder
    // ========================================================================

    // Weather Agent
    let weather_agent = AgentBuilder::new("weather_agent")
        .description("Handles weather-related queries - current conditions and forecasts")
        .system_prompt(
            "You are a weather specialist. Use your tools to provide weather information.",
        )
        .tool(GetWeatherTool::new())
        .tool(GetForecastTool::new());

    // Calendar Agent
    let calendar_agent = AgentBuilder::new("calendar_agent")
        .description(
            "Manages calendar and scheduling - create events and list upcoming appointments",
        )
        .system_prompt(
            "You are a calendar specialist. Use your tools to manage schedules and events.",
        )
        .tool(ScheduleEventTool::new())
        .tool(ListEventsTool::new());

    // Email Agent
    let email_agent = AgentBuilder::new("email_agent")
        .description("Handles email operations - send messages and search inbox")
        .system_prompt("You are an email specialist. Use your tools to manage emails.")
        .tool(SendEmailTool::new())
        .tool(SearchEmailsTool::new());

    // File Agent
    let file_agent = AgentBuilder::new("file_agent")
        .description("Manages file operations - create, read, update, delete, and list files")
        .system_prompt("You are a file system specialist. Use your tools to manage files.")
        .tool(FileOperationTool::new())
        .tool(ListFilesTool::new());

    // Collect agents
    let agents = AgentCollection::new()
        .add(weather_agent)
        .add(calendar_agent)
        .add(email_agent)
        .add(file_agent);

    println!("Created {} custom specialized agents:", agents.len());
    for (name, description) in agents.list_agents() {
        println!("  - {}: {}", name, description);
    }

    // Build agent configurations
    let agent_configs = agents.build();
    println!();

    // ========================================================================
    // Task 1: Weather Query - Should route to Weather Agent
    // ========================================================================
    println!("Task 1: Weather Query");
    println!("=====================");
    println!("User: What's the weather like in New York?\n");

    let result = router::route_task_with_custom_agents(
        agent_configs.clone(),
        "What's the weather like in New York?",
    )
    .await?;

    println!("Routed to agent: {}", result.result);
    println!();

    // ========================================================================
    // Task 2: Calendar Request - Should route to Calendar Agent
    // ========================================================================
    println!("Task 2: Calendar Request");
    println!("========================");
    println!("User: Schedule a team meeting for tomorrow at 10 AM\n");

    let result = router::route_task_with_custom_agents(
        agent_configs.clone(),
        "Schedule a team meeting for tomorrow at 10 AM",
    )
    .await?;

    println!("Routed to agent: {}", result.result);
    println!();

    // ========================================================================
    // Task 3: Email Task - Should route to Email Agent
    // ========================================================================
    println!("Task 3: Email Task");
    println!("==================");
    println!("User: Send an email to john@example.com about the project update\n");

    let result = router::route_task_with_custom_agents(
        agent_configs.clone(),
        "Send an email to john@example.com about the project update",
    )
    .await?;

    println!("Routed to agent: {}", result.result);
    println!();

    // ========================================================================
    // Task 4: File Operation - Should route to File Agent
    // ========================================================================
    println!("Task 4: File Operation");
    println!("======================");
    println!("User: Create a new file called report.txt with the quarterly results\n");

    let result = router::route_task_with_custom_agents(
        agent_configs.clone(),
        "Create a new file called report.txt with the quarterly results",
    )
    .await?;

    println!("Routed to agent: {}", result.result);
    println!();

    // ========================================================================
    // Task 5: Ambiguous Query - Tests Router's Classification
    // ========================================================================
    println!("Task 5: Ambiguous Query");
    println!("=======================");
    println!("User: What do I have coming up?\n");

    let result =
        router::route_task_with_custom_agents(agent_configs.clone(), "What do I have coming up?")
            .await?;

    println!("Routed to agent: {}", result.result);
    println!();

    println!("=== Key Concepts Demonstrated ===");
    println!("1. Custom tools created with #[tool_fn] macro");
    println!("2. AgentBuilder fluent API for clean agent creation");
    println!("3. AgentCollection for managing multiple agents");
    println!("4. Router uses LLM to classify user intent");
    println!("5. Router autonomously selects the most appropriate agent");
    println!("6. 'One-way ticket' pattern - single agent handles entire task");
    println!("7. Each agent is specialized with domain-specific tools");
    println!();

    println!("=== Router vs Supervisor ===");
    println!("Router: LLM classifies intent → routes to ONE agent → agent completes task");
    println!("Supervisor: LLM plans steps → orchestrates MULTIPLE agents → combines results");
    println!();

    println!("=== Router with Custom Agents Complete ===\n");

    Ok(())
}
