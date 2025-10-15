use colored::*;

pub fn print_header(text: &str) {
    println!("\n{}", text.bright_cyan().bold());
    println!("{}", "=".repeat(text.len()).bright_cyan());
}

pub fn print_success(text: &str) {
    println!("{}", text.green());
}

pub fn print_error(text: &str) {
    eprintln!("{}", text.red().bold());
}

pub fn print_info(text: &str) {
    println!("{}", text.blue());
}

pub fn print_prompt(text: &str) {
    print!("{}", text.yellow().bold());
}
