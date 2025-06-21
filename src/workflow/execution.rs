use crate::agent::Agent;
use crate::ruler::rule_types::{ActionType, CmdKind};
use anyhow::Result;
use tokio::time::Duration;

pub struct ActionExecutor {
    test_mode: bool,
}

impl ActionExecutor {
    pub fn new(test_mode: bool) -> Self {
        Self { test_mode }
    }

    /// Execute an action based on the ActionType system
    pub async fn execute_action(&self, agent: &Agent, action: ActionType) -> Result<()> {
        match action {
            ActionType::SendKeys(keys) => {
                println!("→ Sending keys to agent {}: {:?}", agent.id(), keys);
                self.send_keys_to_agent(agent, keys).await?;
            }
            ActionType::Workflow(workflow_name, args) => {
                println!(
                    "→ Executing workflow '{}' for agent {} with args: {:?}",
                    workflow_name,
                    agent.id(),
                    args
                );
                self.execute_workflow(agent, &workflow_name, args).await?;
            }
            ActionType::Legacy(cmd_kind, args) => {
                // Handle legacy commands during transition period
                println!(
                    "→ Executing legacy command {:?} for agent {} with args: {:?}",
                    cmd_kind,
                    agent.id(),
                    args
                );
                self.send_command_to_agent(agent, cmd_kind, args).await?;
            }
        }
        Ok(())
    }

    /// Send keys directly to the terminal
    async fn send_keys_to_agent(&self, agent: &Agent, keys: Vec<String>) -> Result<()> {
        if self.test_mode {
            println!(
                "ℹ️ Test mode: would send keys {:?} to agent {}",
                keys,
                agent.id()
            );
            return Ok(());
        }

        for key in keys {
            println!("  → Sending key: '{}'", key);
            agent
                .send_keys(&key)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send key '{}': {}", key, e))?;
            // Small delay between keys to avoid overwhelming the terminal
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }

    /// Execute a workflow by name
    async fn execute_workflow(
        &self,
        agent: &Agent,
        workflow_name: &str,
        args: Vec<String>,
    ) -> Result<()> {
        match workflow_name {
            "github_issue_resolution" => {
                // Use the existing entry command logic for GitHub issue resolution
                self.execute_entry_command(agent, args).await?;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown workflow: {}", workflow_name));
            }
        }
        Ok(())
    }

    async fn send_command_to_agent(
        &self,
        agent: &Agent,
        command: CmdKind,
        args: Vec<String>,
    ) -> Result<()> {
        match command {
            CmdKind::Entry => {
                println!(
                    "→ Executing entry for agent {} with args: {:?}",
                    agent.id(),
                    args
                );
                self.execute_entry_command(agent, args).await?;
            }
            CmdKind::Resume => {
                println!("→ Sending resume to agent {}", agent.id());
                // Resume command should be handled by the workflow module
                return Err(anyhow::anyhow!(
                    "Resume command should be handled by Workflow module"
                ));
            }
        }

        Ok(())
    }

    async fn execute_entry_command(&self, agent: &Agent, args: Vec<String>) -> Result<()> {
        // Extract issue number from args or parse from agent_id
        let issue_number = if !args.is_empty() {
            args[0].clone()
        } else {
            // Try to extract from agent_id if it contains issue number
            agent.id().split('-').next_back().unwrap_or("1").to_string()
        };

        println!("🚀 Starting entry workflow for issue #{}", issue_number);

        // Step 1: Git operations
        self.handle_git_operations(agent, &issue_number).await?;

        // Step 2: Create and switch to worktree
        self.setup_worktree(agent, &issue_number).await?;

        // Step 3: Create draft PR
        self.create_draft_pr(agent, &issue_number).await?;

        // Step 4: Implementation phase (this would integrate with actual implementation logic)
        self.coordinate_implementation(agent, &issue_number).await?;

        println!("✅ Entry workflow completed for issue #{}", issue_number);
        Ok(())
    }

    async fn handle_git_operations(&self, agent: &Agent, issue_number: &str) -> Result<()> {
        println!("📦 Handling git operations for issue #{}", issue_number);

        // Skip actual git operations in test environment
        if self.test_mode {
            println!("ℹ️ Skipping git operations in test environment");
            return Ok(());
        }

        // Check if we're in a worktree first
        let pwd_result = agent
            .execute_command("pwd")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;

        let current_dir = pwd_result.output.trim();

        // If we're already in a worktree for this issue, we don't need to do git operations
        if current_dir.contains(&format!(".worktree/issue-{}", issue_number)) {
            println!("ℹ️ Already in correct worktree, skipping git operations");
            return Ok(());
        }

        // Check if we're in the main worktree directory and need to navigate elsewhere
        if current_dir.contains(".worktree") {
            println!("ℹ️ In worktree environment, git operations handled by worktree setup");
            return Ok(());
        }

        // Check current branch first
        let branch_result = agent
            .execute_command("git branch --show-current")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get current branch: {}", e))?;

        let current_branch = branch_result.output.trim();

        // Only checkout main if we're not already on it and not in a worktree
        if current_branch != "main" {
            let result = agent
                .execute_command("git checkout main")
                .await
                .map_err(|e| anyhow::anyhow!("Failed to checkout main: {}", e))?;

            if result.exit_code != Some(0) {
                return Err(anyhow::anyhow!(
                    "Git checkout main failed: {}",
                    result.error
                ));
            }
        }

        let result = agent
            .execute_command("git pull origin main")
            .await
            .map_err(|e| anyhow::anyhow!("Failed to pull main: {}", e))?;

        if result.exit_code != Some(0) {
            return Err(anyhow::anyhow!("Git pull failed: {}", result.error));
        }

        println!("✅ Git operations completed successfully");
        Ok(())
    }

    async fn setup_worktree(&self, agent: &Agent, issue_number: &str) -> Result<()> {
        println!("🌳 Setting up worktree for issue #{}", issue_number);

        // Skip actual worktree operations in test environment
        if self.test_mode {
            println!("ℹ️ Skipping worktree operations in test environment");
            return Ok(());
        }

        // Check if worktree already exists
        let worktree_path = format!(".worktree/issue-{}", issue_number);
        let check_result = agent
            .execute_command(&format!("test -d {}", worktree_path))
            .await;

        if check_result.is_ok() && check_result.unwrap().exit_code == Some(0) {
            println!("ℹ️ Worktree already exists, switching to it");
            agent
                .set_working_directory(&worktree_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to switch to worktree: {}", e))?;
        } else {
            // Check if branch already exists first
            let branch_name = format!("issue-{}", issue_number);
            let branch_check = agent
                .execute_command(&format!(
                    "git show-ref --verify --quiet refs/heads/{}",
                    branch_name
                ))
                .await;

            let cmd = if branch_check.is_ok() && branch_check.unwrap().exit_code == Some(0) {
                // Branch exists, add worktree without creating new branch
                format!("git worktree add {} {}", worktree_path, branch_name)
            } else {
                // Branch doesn't exist, create new branch
                format!("git worktree add {} -b {}", worktree_path, branch_name)
            };

            let result = agent
                .execute_command(&cmd)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create worktree: {}", e))?;

            if result.exit_code != Some(0) {
                return Err(anyhow::anyhow!(
                    "Worktree creation failed: {}",
                    result.error
                ));
            }

            agent
                .set_working_directory(&worktree_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to switch to worktree: {}", e))?;
        }

        println!("✅ Worktree setup completed");
        Ok(())
    }

    async fn create_draft_pr(&self, agent: &Agent, issue_number: &str) -> Result<()> {
        println!("📝 Creating draft PR for issue #{}", issue_number);

        // Skip actual PR operations in test environment
        if self.test_mode {
            println!("ℹ️ Skipping PR operations in test environment");
            return Ok(());
        }

        // Get issue details
        let issue_cmd = format!("gh issue view {}", issue_number);
        let issue_result = agent
            .execute_command(&issue_cmd)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get issue details: {}", e))?;

        if issue_result.exit_code != Some(0) {
            return Err(anyhow::anyhow!(
                "Failed to retrieve issue: {}",
                issue_result.error
            ));
        }

        // Extract title from issue output (simplified - would need proper parsing)
        let title = format!("Fix issue #{}", issue_number);

        // Create empty commit
        let commit_cmd = format!("git commit --allow-empty -m \"{}\"", title);
        let commit_result = agent
            .execute_command(&commit_cmd)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create empty commit: {}", e))?;

        if commit_result.exit_code != Some(0) {
            return Err(anyhow::anyhow!(
                "Empty commit failed: {}",
                commit_result.error
            ));
        }

        // Push branch
        let branch_name = format!("issue-{}", issue_number);
        let push_cmd = format!("git push -u origin {}", branch_name);
        let push_result = agent
            .execute_command(&push_cmd)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to push branch: {}", e))?;

        if push_result.exit_code != Some(0) {
            return Err(anyhow::anyhow!("Branch push failed: {}", push_result.error));
        }

        // Create draft PR
        let pr_body = format!("Closes #{}\n\n## Summary\nImplementation for issue #{}\n\n🤖 Generated with Terminal Backend", issue_number, issue_number);
        let pr_cmd = format!(
            "gh pr create --draft --title \"{}\" --body \"{}\"",
            title, pr_body
        );

        let pr_result = agent
            .execute_command(&pr_cmd)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create PR: {}", e))?;

        if pr_result.exit_code != Some(0) {
            return Err(anyhow::anyhow!("PR creation failed: {}", pr_result.error));
        }

        println!("✅ Draft PR created successfully");
        println!("📎 PR URL: {}", pr_result.output.trim());
        Ok(())
    }

    async fn coordinate_implementation(&self, agent: &Agent, issue_number: &str) -> Result<()> {
        println!("🔧 Coordinating implementation for issue #{}", issue_number);

        // This is where the actual implementation logic would be coordinated
        // For now, this is a placeholder that demonstrates the integration point

        // Example: Run tests
        self.run_quality_checks(agent).await?;

        println!("✅ Implementation coordination completed");
        Ok(())
    }

    async fn run_quality_checks(&self, agent: &Agent) -> Result<()> {
        println!("🧪 Running quality checks");

        // Skip actual quality checks in test environment
        if self.test_mode {
            println!("ℹ️ Skipping quality checks in test environment");
            return Ok(());
        }

        // Check if we have Cargo.toml (Rust project)
        let cargo_check = agent.execute_command("test -f Cargo.toml").await;

        if cargo_check.is_ok() && cargo_check.unwrap().exit_code == Some(0) {
            // Run Rust quality checks
            let checks = vec!["cargo fmt --check", "cargo clippy", "cargo test"];

            for check in checks {
                println!("Running: {}", check);
                let result = agent
                    .execute_command(check)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to run {}: {}", check, e))?;

                if result.exit_code != Some(0) {
                    println!("⚠️ Check failed: {}", check);
                    println!("Error: {}", result.error);
                    // Continue with other checks but log the failure
                }
            }
        }

        println!("✅ Quality checks completed");
        Ok(())
    }
}
