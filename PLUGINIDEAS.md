// File: examples/plugin_architecture.rs

use std::collections::HashMap;

// Core plugin trait
trait Plugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn initialize(&mut self) -> Result<(), String>;
    fn execute(&self, input: &str) -> Result<String, String>;
    fn shutdown(&mut self);
}

// Plugin metadata
#[derive(Debug, Clone)]
struct PluginMetadata {
    name: String,
    version: String,
    author: String,
    description: String,
}

// Enhanced plugin trait with metadata
trait ExtendedPlugin: Plugin {
    fn metadata(&self) -> PluginMetadata;
    fn can_handle(&self, input: &str) -> bool;
}

// Concrete plugin implementations
struct TextTransformPlugin {
    name: String,
    initialized: bool,
}

impl TextTransformPlugin {
    fn new() -> Self {
        TextTransformPlugin {
            name: "TextTransform".to_string(),
            initialized: false,
        }
    }
}

impl Plugin for TextTransformPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<(), String> {
        println!("  [{}] Initializing...", self.name);
        self.initialized = true;
        Ok(())
    }

    fn execute(&self, input: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("Plugin not initialized".to_string());
        }
        Ok(input.to_uppercase())
    }

    fn shutdown(&mut self) {
        println!("  [{}] Shutting down...", self.name);
        self.initialized = false;
    }
}

impl ExtendedPlugin for TextTransformPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: self.version().to_string(),
            author: "Rust Team".to_string(),
            description: "Transforms text to uppercase".to_string(),
        }
    }

    fn can_handle(&self, input: &str) -> bool {
        !input.is_empty()
    }
}

struct ReversePlugin {
    name: String,
    initialized: bool,
}

impl ReversePlugin {
    fn new() -> Self {
        ReversePlugin {
            name: "Reverse".to_string(),
            initialized: false,
        }
    }
}

impl Plugin for ReversePlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<(), String> {
        println!("  [{}] Initializing...", self.name);
        self.initialized = true;
        Ok(())
    }

    fn execute(&self, input: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("Plugin not initialized".to_string());
        }
        Ok(input.chars().rev().collect())
    }

    fn shutdown(&mut self) {
        println!("  [{}] Shutting down...", self.name);
        self.initialized = false;
    }
}

impl ExtendedPlugin for ReversePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: self.version().to_string(),
            author: "Rust Team".to_string(),
            description: "Reverses input text".to_string(),
        }
    }

    fn can_handle(&self, input: &str) -> bool {
        !input.is_empty()
    }
}

struct CounterPlugin {
    name: String,
    initialized: bool,
    count: usize,
}

impl CounterPlugin {
    fn new() -> Self {
        CounterPlugin {
            name: "Counter".to_string(),
            initialized: false,
            count: 0,
        }
    }
}

impl Plugin for CounterPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<(), String> {
        println!("  [{}] Initializing...", self.name);
        self.initialized = true;
        self.count = 0;
        Ok(())
    }

    fn execute(&self, input: &str) -> Result<String, String> {
        if !self.initialized {
            return Err("Plugin not initialized".to_string());
        }
        let word_count = input.split_whitespace().count();
        let char_count = input.chars().count();
        Ok(format!("Words: {}, Chars: {}", word_count, char_count))
    }

    fn shutdown(&mut self) {
        println!(
            "  [{}] Shutting down... (processed {} times)",
            self.name, self.count
        );
        self.initialized = false;
    }
}

impl ExtendedPlugin for CounterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: self.version().to_string(),
            author: "Rust Team".to_string(),
            description: "Counts words and characters".to_string(),
        }
    }

    fn can_handle(&self, input: &str) -> bool {
        !input.is_empty()
    }
}

// Plugin manager
struct PluginManager {
    plugins: HashMap<String, Box<dyn ExtendedPlugin>>,
}

impl PluginManager {
    fn new() -> Self {
        PluginManager {
            plugins: HashMap::new(),
        }
    }

    fn register<P: ExtendedPlugin + 'static>(&mut self, mut plugin: P) {
        let name = plugin.name().to_string();
        if let Err(e) = plugin.initialize() {
            eprintln!("Failed to initialize plugin {}: {}", name, e);
            return;
        }

        println!(
            "✓ Registered plugin: {} v{}",
            plugin.name(),
            plugin.version()
        );
        self.plugins.insert(name, Box::new(plugin));
    }

    fn list_plugins(&self) {
        println!("\nRegistered Plugins:");
        for (_name, plugin) in &self.plugins {
            let meta = plugin.metadata();
            println!(
                "  • {} v{} by {} - {}",
                meta.name, meta.version, meta.author, meta.description
            );
        }
    }

    fn get_plugin_info(&self, name: &str) -> Option<PluginMetadata> {
        self.plugins.get(name).map(|p| p.metadata())
    }

    fn execute_plugin(&self, name: &str, input: &str) -> Result<String, String> {
        self.plugins
            .get(name)
            .ok_or_else(|| format!("Plugin '{}' not found", name))?
            .execute(input)
    }

    fn execute_all(&self, input: &str) {
        println!("\nExecuting all plugins with input: '{}'", input);
        for (name, plugin) in &self.plugins {
            if plugin.can_handle(input) {
                match plugin.execute(input) {
                    Ok(result) => println!("  [{}] → {}", name, result),
                    Err(e) => println!("  [{}] Error: {}", name, e),
                }
            } else {
                println!("  [{}] Skipped (cannot handle input)", name);
            }
        }
    }

    fn shutdown_all(&mut self) {
        println!("\nShutting down all plugins...");
        for (_name, plugin) in self.plugins.iter_mut() {
            plugin.shutdown();
        }
    }
}

// Plugin with hooks
trait HookablePlugin: Plugin {
    fn on_before_execute(&self, input: &str) {
        println!("  [{}] Before: '{}'", self.name(), input);
    }

    fn on_after_execute(&self, input: &str, output: &str) {
        println!("  [{}] After: '{}' → '{}'", self.name(), input, output);
    }
}

struct LoggingPlugin {
    name: String,
    initialized: bool,
}

impl LoggingPlugin {
    fn new() -> Self {
        LoggingPlugin {
            name: "Logger".to_string(),
            initialized: false,
        }
    }
}

impl Plugin for LoggingPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    fn execute(&self, input: &str) -> Result<String, String> {
        self.on_before_execute(input);
        let output = format!("[LOGGED] {}", input);
        self.on_after_execute(input, &output);
        Ok(output)
    }

    fn shutdown(&mut self) {
        self.initialized = false;
    }
}

impl HookablePlugin for LoggingPlugin {}

impl ExtendedPlugin for LoggingPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: self.version().to_string(),
            author: "Rust Team".to_string(),
            description: "Logs all operations".to_string(),
        }
    }

    fn can_handle(&self, input: &str) -> bool {
        !input.is_empty()
    }
}

// Plugin factory
type PluginFactory = Box<dyn Fn() -> Box<dyn ExtendedPlugin>>;

struct PluginRegistry {
    factories: HashMap<String, PluginFactory>,
}

impl PluginRegistry {
    fn new() -> Self {
        PluginRegistry {
            factories: HashMap::new(),
        }
    }

    fn register_factory<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn ExtendedPlugin> + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(factory));
    }

    fn create(&self, name: &str) -> Option<Box<dyn ExtendedPlugin>> {
        self.factories.get(name).map(|f| f())
    }

    fn list_available(&self) {
        println!("Available plugin types:");
        for name in self.factories.keys() {
            println!("  • {}", name);
        }
    }
}

fn main() {
    println!("✅ Plugin Architecture System\n");
    println!("═══════════════════════════════════════════════════════════");

    // Create plugin manager
    println!("1. Creating Plugin Manager");
    println!("═══════════════════════════════════════════════════════════\n");

    let mut manager = PluginManager::new();

    // Register plugins
    manager.register(TextTransformPlugin::new());
    manager.register(ReversePlugin::new());
    manager.register(CounterPlugin::new());
    manager.register(LoggingPlugin::new());

    // List all plugins (now shows author too)
    manager.list_plugins();

    // Show detailed plugin information
    println!("\n═══════════════════════════════════════════════════════════");
    println!("1.5. Plugin Detailed Information");
    println!("═══════════════════════════════════════════════════════════\n");

    if let Some(meta) = manager.get_plugin_info("TextTransform") {
        println!("Plugin Details:");
        println!("  Name: {}", meta.name);
        println!("  Version: {}", meta.version);
        println!("  Author: {}", meta.author);
        println!("  Description: {}", meta.description);
    }

    // Execute specific plugin
    println!("\n═══════════════════════════════════════════════════════════");
    println!("2. Executing Specific Plugins");
    println!("═══════════════════════════════════════════════════════════\n");

    let input = "Hello Rust Plugins";

    match manager.execute_plugin("TextTransform", input) {
        Ok(result) => println!("TextTransform result: {}", result),
        Err(e) => println!("Error: {}", e),
    }

    match manager.execute_plugin("Reverse", input) {
        Ok(result) => println!("Reverse result: {}", result),
        Err(e) => println!("Error: {}", e),
    }

    match manager.execute_plugin("Counter", input) {
        Ok(result) => println!("Counter result: {}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Execute all plugins
    println!("\n═══════════════════════════════════════════════════════════");
    println!("3. Executing All Plugins");
    println!("═══════════════════════════════════════════════════════════");

    manager.execute_all(input);

    // Plugin factory pattern
    println!("\n═══════════════════════════════════════════════════════════");
    println!("4. Plugin Factory Pattern");
    println!("═══════════════════════════════════════════════════════════\n");

    let mut registry = PluginRegistry::new();

    // Register plugin factories
    registry.register_factory("text-transform", || Box::new(TextTransformPlugin::new()));

    registry.register_factory("reverse", || Box::new(ReversePlugin::new()));

    registry.register_factory("counter", || Box::new(CounterPlugin::new()));

    registry.list_available();

    // Create plugins dynamically
    println!("\nCreating plugins dynamically:");
    if let Some(mut plugin) = registry.create("reverse") {
        let _ = plugin.initialize();
        let meta = plugin.metadata();
        println!("  Created: {} by {}", meta.name, meta.author);
        match plugin.execute("Dynamic Creation") {
            Ok(result) => println!("  Result: {}", result),
            Err(e) => println!("  Error: {}", e),
        }
        plugin.shutdown();
    }

    // Pipeline of plugins
    println!("\n═══════════════════════════════════════════════════════════");
    println!("5. Plugin Pipeline");
    println!("═══════════════════════════════════════════════════════════\n");

    struct PluginPipeline {
        plugins: Vec<Box<dyn ExtendedPlugin>>,
    }

    impl PluginPipeline {
        fn new() -> Self {
            PluginPipeline {
                plugins: Vec::new(),
            }
        }

        fn add<P: ExtendedPlugin + 'static>(mut self, mut plugin: P) -> Self {
            let _ = plugin.initialize();
            self.plugins.push(Box::new(plugin));
            self
        }

        fn execute(&self, input: &str) -> Result<String, String> {
            let mut result = input.to_string();
            for plugin in &self.plugins {
                result = plugin.execute(&result)?;
                println!("  After {}: {}", plugin.name(), result);
            }
            Ok(result)
        }
    }

    let pipeline = PluginPipeline::new()
        .add(TextTransformPlugin::new())
        .add(ReversePlugin::new());

    println!("Pipeline execution:");
    match pipeline.execute("hello") {
        Ok(result) => println!("  Final result: {}", result),
        Err(e) => println!("  Error: {}", e),
    }

    // Shutdown
    println!("\n═══════════════════════════════════════════════════════════");
    println!("6. Cleanup");
    println!("═══════════════════════════════════════════════════════════");

    manager.shutdown_all();

    println!("\n✅ Plugin Architecture Demonstrates:");
    println!("  • Trait-based plugin system");
    println!("  • Dynamic plugin registration");
    println!("  • Plugin lifecycle (init/execute/shutdown)");
    println!("  • Plugin metadata and discovery");
    println!("  • Factory pattern for plugin creation");
    println!("  • Plugin pipelines");
    println!("  • Extensible architecture");
}
