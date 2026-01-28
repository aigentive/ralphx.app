#!/bin/bash
# Verify that a refactor: commit exists in recent commits
# Exit 0 = allow (compliant), Exit 2 = block (non-compliant)

# Check if any code files were modified in this session
# For now, we'll check if there's a refactor: commit in the last 10 commits
REFACTOR_COMMIT=$(git log --oneline -10 2>/dev/null | grep -i "^[a-f0-9]* refactor:" | head -1)

if [ -n "$REFACTOR_COMMIT" ]; then
    # Found a refactor commit - allow
    exit 0
else
    # No refactor commit found - output reason and block
    echo '{"reason": "Missing refactor: commit. Launch Explore agent to find ONE quality improvement, fix it, and commit with refactor: prefix before completing."}'
    exit 2
fi
