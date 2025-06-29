use rule_agents::agent::pty_terminal::PtyTerminal;
use rule_agents::agent::pty_session::{PtyEvent, PtyEventData};
use std::time::Instant;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create event channel
    let (event_tx, mut event_rx) = broadcast::channel(1024);
    let start_time = Instant::now();

    // Create PTY terminal
    println!("Creating PTY terminal...");
    let terminal = PtyTerminal::new(
        "/bin/zsh".to_string(),
        80,
        24,
        event_tx.clone(),
        start_time,
    ).await?;

    // Spawn task to print events
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            match &event.data {
                PtyEventData::Output { data } => {
                    println!("📤 PTY Output: {:?}", data);
                }
                _ => {}
            }
        }
    });

    // Send ls command
    println!("Sending 'ls --color=always' command...");
    terminal.write_input(b"ls --color=always\r").await?;

    // Wait a bit for output
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Send another command
    println!("Sending 'echo test' command...");
    terminal.write_input(b"echo test\r").await?;

    // Wait for output
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("Test complete!");
    Ok(())
}