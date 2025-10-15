//! Agent Introspection Example
//!
//! Demonstrates how to discover available agents and their capabilities

use actorus::router;

fn main() {
    println!("\n=== Available Specialized Agents ===\n");

    let agents = router::list_agents();
    println!("Found {} specialized agents:\n", agents.len());

    for agent_name in agents {
        println!("Agent: {}", agent_name);
        if let Some(description) = router::agent_info(agent_name) {
            println!("  Description: {}", description);
        }
        println!();
    }

    println!("=== Agent Selection Guidelines ===\n");
    println!("Use 'file_ops_agent' for:");
    println!("  - Reading files");
    println!("  - Writing files");
    println!("  - File I/O operations\n");

    println!("Use 'shell_agent' for:");
    println!("  - Running shell commands");
    println!("  - Directory listings");
    println!("  - System operations\n");

    println!("Use 'web_agent' for:");
    println!("  - HTTP requests");
    println!("  - Fetching web content");
    println!("  - API calls\n");

    println!("Use 'general_agent' for:");
    println!("  - Mixed operations");
    println!("  - Tasks requiring multiple tool types");
    println!("  - When domain is unclear\n");

    println!("Note: The router automatically selects the best agent for your task!");
    println!("You don't need to choose manually.\n");
}
