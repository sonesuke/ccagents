# solve

Handles GitHub issues by creating a worktree, draft PR, and implementing the solution.

## Usage

```
/solve <issue-number>
```

## Workflow

1. **Check Existing Work and Setup**
   - Check if worktree `.worktree/issue-<issue-number>` already exists
   - If exists: Switch to existing worktree and review current progress
   - If not exists: Create new branch from latest main and create worktree
   - Always ensure main branch is up-to-date before creating new branches

2. **Create Draft Pull Request**
   - Create an empty commit using `git commit --allow-empty` to enable PR creation
   - Push the branch to remote
   - Create a draft PR immediately after branch creation
   - Link the PR to the issue using "Closes #<issue-number>" in the PR description
   - Set PR title to match the issue title

3. **Implement Solution**
   - Read and analyze the issue description
   - Plan the implementation using TodoWrite
   - Execute the tasks according to the issue requirements
   - Commit changes with descriptive messages

4. **Update Pull Request**
   - Push commits to the remote branch
   - Update PR description with implementation details
   - Update all related documentation (README.md, docs/, examples/) if needed
   - Run code quality checks before final push

5. **Verify CI and Finalize**
   - Check CI status with `gh pr checks`
   - If CI fails, fix issues (commonly formatting with `cargo fmt`)
   - Push fixes and wait for CI to pass
   - Remove draft status with `gh pr ready`
   - Mark as ready for review

## Example Commands

```bash
# Check if worktree already exists
if [ -d ".worktree/issue-<issue-number>" ]; then
  echo "Existing worktree found. Switching to continue work..."
  cd .worktree/issue-<issue-number>
else
  echo "Creating new worktree..."
  git checkout main
  git pull origin main
  git worktree add .worktree/issue-<issue-number> -b issue-<issue-number>
  cd .worktree/issue-<issue-number>
fi

# Create empty commit for draft PR
git commit --allow-empty -m "<issue-title>"
git push -u origin issue-<issue-number>

# Create draft PR
gh pr create --draft --title "<issue-title>" --body "Closes #<issue-number>\n\n## Summary\n[Implementation details]\n\n## Test plan\n[Testing approach]"

# After implementation - run quality checks
cargo fmt
cargo clippy
cargo test
git add -A && git commit -m "Implementation complete"
git push

# Check CI and finalize
gh pr checks
# If CI fails, fix issues and push again
gh pr ready
```

## Notes

- Always work within the worktree to keep the main working directory clean
- Run `cargo fmt`, `cargo clippy`, and `cargo test` before pushing final implementation
- Check CI status with `gh pr checks` after pushing - common failures are formatting issues
- Update all relevant documentation (README.md, docs/, examples/) when changing core functionality
- Follow the project's coding standards and conventions
- Update PR description with clear summary of changes
- Wait for all CI checks to pass before marking PR as ready for review