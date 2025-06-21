# resume

Resume work on the current pull request by checking its status and providing updates.

## Usage

```
/resume
```

## What it does

1. **Check Current PR Status**
   - View the current pull request details
   - Check CI/CD pipeline status
   - Review any failing tests or checks

2. **Analyze Issues**
   - Identify any failing tests or build issues
   - Check for merge conflicts with main branch
   - Review feedback or requested changes
   - Check if branch is behind origin/main

3. **Resolve Conflicts & Update**
   - Rebase against latest origin/main if conflicts exist
   - Resolve any merge conflicts automatically where possible
   - Fix any identified issues
   - Update code based on feedback
   - Re-run tests if necessary
   - Push updates to the PR

4. **Final Verification**
   - Ensure all checks pass
   - Verify PR is ready for review/merge
   - Update PR title and description to reflect actual implementation
   - Ensure PR body includes comprehensive feature summary
   - Check that usage examples and test results are current
   - Display "MISSION COMPLETE!" when everything is perfect

## Example Output

The command will show:
- Current PR number and status
- Branch comparison with origin/main
- CI check results (✓ pass / ✗ fail)
- Any failing tests with details
- Merge conflict detection and resolution
- Rebase progress if needed
- PR description update recommendations
- Suggested actions to resolve issues
- Progress updates as fixes are applied
- **"MISSION COMPLETE!"** when all tasks are successfully finished

## Notes

- Automatically detects the current branch's associated PR
- Performs safe rebase operations against origin/main when conflicts detected
- Works with GitHub Actions, Travis CI, and other CI systems
- Provides detailed logs for any failures
- Can handle multiple types of issues simultaneously
- Preserves commit history during rebase operations

## Common Scenarios

- **Returning to work**: Resume after time away from PR
- **Addressing feedback**: Handle reviewer comments and suggestions
- **Fixing CI failures**: Resolve failing tests or build issues
- **Resolving conflicts**: Rebase against updated main branch
- **Updating documentation**: Refresh PR description with latest implementation details
- **Final review prep**: Ensure PR is ready for merge with comprehensive documentation

This command is particularly useful when returning to work on a PR after some time, or when addressing reviewer feedback and potential conflicts with the main branch.