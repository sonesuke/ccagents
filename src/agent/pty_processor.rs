use crate::agent::Agent;
use crate::config;
use anyhow::Result;

/// Process PTY output for pattern matching
pub async fn process_pty_output(
    pty_output: &str,
    agent: &Agent,
    rule_config: &config::RuleConfig,
) -> Result<()> {
    // Reset timeout activity for diff_timeout rules whenever ANY terminal output is received
    // This ensures diff_timeout detects "no terminal output" rather than "no pattern matches"
    rule_config.reset_timeout_activity().await;

    // Remove ANSI escape sequences for cleaner pattern matching
    let clean_output = strip_ansi_escapes(pty_output);

    tracing::debug!("=== PTY OUTPUT ===");
    tracing::debug!("Raw output: {:?}", pty_output);
    tracing::debug!("Clean output: {:?}", clean_output);
    tracing::debug!("==> Will check rules for PTY output");

    // Split by both \n and \r for better handling of carriage returns
    let lines: Vec<&str> = clean_output
        .split(['\n', '\r'])
        .filter(|line| !line.trim().is_empty())
        .collect();

    // Check each line for pattern matching and timeout rules
    for line in lines {
        tracing::debug!("Checking line: {:?}", line);

        let actions = rule_config.decide_actions_with_timeout(line).await;

        tracing::debug!("Actions decided: {:?}", actions);

        for action in actions {
            crate::agent::execution::execute_rule_action(&action, agent).await?;
        }
    }

    Ok(())
}

/// Strip ANSI escape sequences from text
fn strip_ansi_escapes(text: &str) -> String {
    let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[mGKHF]").unwrap();
    ansi_regex.replace_all(text, "").to_string()
}
