use regex::Regex;

fn main() {
    // Test the exact pattern from config
    let pattern = r"こんにちは|Hello";
    let regex = Regex::new(pattern).unwrap();
    
    println!("Testing pattern: {:?}", pattern);
    println!();
    
    // Test strings that should match
    let test_cases = vec![
        "こんにちは",
        "こんにちは！",
        "⏺ こんにちは！",
        "Hello",
        "Welcome to Claude Code!",
        "> say hello, in Japanese\n\n⏺ こんにちは！\n\n",
    ];
    
    for test in &test_cases {
        if regex.is_match(test) {
            println!("✓ MATCHES: {:?}", test);
            if let Some(capture) = regex.find(test) {
                println!("  Matched substring: {:?}", capture.as_str());
            }
        } else {
            println!("✗ NO MATCH: {:?}", test);
        }
    }
    
    // Test the actual content from the log
    let actual_content = "Users/sonesuke/rule-agents                │\n╰───────────────────────────────────────────────────╯\n\n\n> say hello, in Japanese\n\n⏺ こんにちは！\n\n╭──────────────────────────────────────────────────────────────────────────────╮\n│\u{a0}>\u{a0}Try \"how does compiled_rule.rs work?\"                                      │\n╰──────────────────────────────────────────────────────────────────────────────╯\n  ? for shortcuts";
    
    println!("\nTesting actual log content:");
    if regex.is_match(actual_content) {
        println!("✓ MATCHES actual content!");
        if let Some(capture) = regex.find(actual_content) {
            println!("  Matched substring: {:?}", capture.as_str());
        }
    } else {
        println!("✗ NO MATCH for actual content");
    }
}