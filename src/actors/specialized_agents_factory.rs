//! Factory for creating default specialized agents
//!
//! Information Hiding:
//! - Hides specific agent configurations
//! - Encapsulates tool assignment to agents
//! - Provides simple creation interface
//!
//! Implementation Note:
//! - Factory is a thin convenience layer over AgentBuilder
//! - Provides curated, pre-configured agents with sensible defaults

use crate::actors::agent_builder::AgentBuilder;
use crate::actors::specialized_agent::{SpecializedAgent, SpecializedAgentConfig};
use crate::config::Settings;
use crate::tools::*;

/// Create a file operations specialized agent
pub fn create_file_ops_agent(settings: Settings, api_key: String) -> SpecializedAgent {
    let (name, description, system_prompt, tools, response_schema, return_tool_output) = AgentBuilder::new("file_ops_agent")
        .description(
            "Handles file system operations including reading and writing files. \
             Use this agent for tasks involving file I/O operations."
        )
        .system_prompt(
            "You are a file operations specialist. Your role is to handle file system tasks. \
             You can read files, write files, and manage file contents. \
             Focus on providing accurate file operations and clear feedback about what was done."
        )
        .tool(filesystem::ReadFileTool::new(1024 * 1024 * 10))  // 10MB limit
        .tool(filesystem::WriteFileTool::new(1024 * 1024 * 10)) // 10MB limit
        .build();

    let config = SpecializedAgentConfig {
        name,
        description,
        system_prompt,
        tools,
        response_schema,
        return_tool_output,
    };

    SpecializedAgent::new(config, settings, api_key)
}

/// Create a shell command specialized agent
pub fn create_shell_agent(settings: Settings, api_key: String) -> SpecializedAgent {
    let (name, description, system_prompt, tools, response_schema, return_tool_output) = AgentBuilder::new("shell_agent")
        .description(
            "Executes shell commands and system operations. \
             Use this agent for tasks involving command-line operations, \
             directory listings, process management, and system queries."
        )
        .system_prompt(
            "You are a shell command specialist. Your role is to execute system commands. \
             You can run shell commands to interact with the operating system. \
             Always be cautious with commands and provide clear explanations of what each command does. \
             Focus on safe, read-only operations when possible."
        )
        .tool(shell::ShellTool::new(30)) // 30 second timeout
        .build();

    let config = SpecializedAgentConfig {
        name,
        description,
        system_prompt,
        tools,
        response_schema,
        return_tool_output,
    };

    SpecializedAgent::new(config, settings, api_key)
}

/// Create a web/HTTP specialized agent
pub fn create_web_agent(settings: Settings, api_key: String) -> SpecializedAgent {
    let (name, description, system_prompt, tools, response_schema, return_tool_output) = AgentBuilder::new("web_agent")
        .description(
            "Handles HTTP requests and web-based operations. \
             Use this agent for tasks involving fetching web content, \
             making API calls, and retrieving online information."
        )
        .system_prompt(
            "You are a web operations specialist. Your role is to handle HTTP requests. \
             You can fetch web pages, call APIs, and retrieve online information. \
             Always verify URLs and provide clear summaries of the data retrieved."
        )
        .tool(http::HttpTool::new(30)) // 30 second timeout
        .build();

    let config = SpecializedAgentConfig {
        name,
        description,
        system_prompt,
        tools,
        response_schema,
        return_tool_output,
    };

    SpecializedAgent::new(config, settings, api_key)
}

/// Create a general-purpose agent with all tools (for backwards compatibility)
pub fn create_general_agent(settings: Settings, api_key: String) -> SpecializedAgent {
    let (name, description, system_prompt, tools, response_schema, return_tool_output) = AgentBuilder::new("general_agent")
        .description(
            "General-purpose agent with access to all tools. \
             Use this agent for tasks that require multiple tool categories \
             or when the task doesn't clearly fit into a specific domain."
        )
        .system_prompt(
            "You are a general-purpose autonomous agent. \
             You have access to file operations, shell commands, and web requests. \
             Choose the appropriate tools for each task and execute them efficiently."
        )
        .tool(shell::ShellTool::new(30))
        .tool(filesystem::ReadFileTool::new(1024 * 1024 * 10))
        .tool(filesystem::WriteFileTool::new(1024 * 1024 * 10))
        .tool(http::HttpTool::new(30))
        .build();

    let config = SpecializedAgentConfig {
        name,
        description,
        system_prompt,
        tools,
        response_schema,
        return_tool_output,
    };

    SpecializedAgent::new(config, settings, api_key)
}

/// Create all default specialized agents
pub fn create_default_agents(settings: Settings, api_key: String) -> Vec<SpecializedAgent> {
    vec![
        create_file_ops_agent(settings.clone(), api_key.clone()),
        create_shell_agent(settings.clone(), api_key.clone()),
        create_web_agent(settings.clone(), api_key.clone()),
        create_general_agent(settings, api_key),
    ]
}
