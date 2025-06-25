use regex::Regex;

fn main() {
    // Test various patterns against potential Claude UI text
    let test_strings = vec![
        "Welcome to Claude Code!",
        "What can I help you with today?",
        "> q· Wizarding… (2s · ↑ 0 tokens · esc to interrupt)",
        "Hello",
        "hello",
        "こんにちは！",
        "Assistant: こんにちは！",
        "/help for help, /status for your current setup",
        "What's new: • Added support for MCP OAuth Authorization Server discovery",
    ];
    
    let patterns = vec![
        ("Original pattern", r"こんにちは|Hello"),
        ("Specific Japanese", r"^こんにちは[!！]?$"),
        ("Word boundary Hello", r"\bHello\b"),
        ("Line start Hello", r"^Hello"),
        ("Any q character", r"q"),
    ];
    
    println!("Testing patterns against Claude UI strings:\n");
    
    for (pattern_name, pattern_str) in &patterns {
        if let Ok(regex) = Regex::new(pattern_str) {
            println!("Pattern: {} => \"{}\"", pattern_name, pattern_str);
            println!("Matches:");
            
            for test_str in &test_strings {
                if regex.is_match(test_str) {
                    println!("  ✓ \"{}\"", test_str);
                }
            }
            println!();
        }
    }
}