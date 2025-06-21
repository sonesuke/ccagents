# solve

Handles GitHub issues by creating a worktree, draft PR, and implementing the solution.

## Usage

```
/solve <issue-number>
```

## Workflow

1. **Create Branch and Worktree**
   - Create a new branch named `issue-<issue-number>` from the main branch
   - Create a git worktree under `.worktree/` directory with the same name as the branch
   - Switch to the new worktree

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
   - Ensure all tests pass and linting is clean

5. **Finalize Pull Request**
   - Verify all checks have passed
   - Remove draft status from the PR
   - Mark as ready for review

## Example Commands

```bash
# Create branch and worktree
git checkout -b issue-<issue-number>
git worktree add .worktree/issue-<issue-number> issue-<issue-number>
cd .worktree/issue-<issue-number>

# Create empty commit for draft PR
git commit --allow-empty -m "<issue-title>"
git push -u origin issue-<issue-number>

# Create draft PR
gh pr create --draft --title "<issue-title>" --body "Closes #<issue-number>\n\n## Summary\n[Implementation details]\n\n## Test plan\n[Testing approach]"

# After implementation
git push
gh pr ready
```

## Notes

- Always work within the worktree to keep the main working directory clean
- Ensure all CI checks pass before marking PR as ready
- Follow the project's coding standards and conventions
- Update PR description with clear summary of changes