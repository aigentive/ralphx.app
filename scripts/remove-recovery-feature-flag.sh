#!/bin/bash
# Remove Session Recovery Feature Flag
#
# This script removes the ENABLE_SESSION_RECOVERY feature flag from the code
# and makes session recovery the default behavior.
#
# ⚠️  ONLY RUN THIS AFTER:
#    1. Success rate ≥95% confirmed
#    2. No critical bugs
#    3. Team approval obtained
#
# Usage: ./scripts/remove-recovery-feature-flag.sh [--dry-run]

set -euo pipefail

DRY_RUN=false
if [ "${1:-}" = "--dry-run" ]; then
    DRY_RUN=true
    echo "🔍 DRY RUN MODE - No changes will be made"
    echo ""
fi

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m' # No Color

TARGET_FILE="src-tauri/src/application/chat_service/chat_service_send_background.rs"

echo -e "${BOLD}=== Session Recovery Feature Flag Removal ===${NC}"
echo ""

# Check if file exists
if [ ! -f "$TARGET_FILE" ]; then
    echo -e "${RED}Error: Target file not found: $TARGET_FILE${NC}"
    exit 1
fi

# Verify feature flag exists
if ! grep -q "ENABLE_SESSION_RECOVERY" "$TARGET_FILE"; then
    echo -e "${YELLOW}Warning: Feature flag not found in file${NC}"
    echo "The feature flag may have already been removed."
    exit 0
fi

echo "Target file: $TARGET_FILE"
echo ""

# Show what will be removed
echo -e "${BOLD}Current implementation (lines 767-783):${NC}"
echo ""
grep -A 16 "// Feature flag check" "$TARGET_FILE" | head -17
echo ""

# Create backup
BACKUP_FILE="${TARGET_FILE}.backup-$(date +%Y%m%d-%H%M%S)"
if [ "$DRY_RUN" = false ]; then
    cp "$TARGET_FILE" "$BACKUP_FILE"
    echo -e "${GREEN}✓ Backup created: $BACKUP_FILE${NC}"
    echo ""
fi

# Show what will replace it
echo -e "${BOLD}New implementation (feature flag removed):${NC}"
echo ""
cat << 'EOF'
                        // Check retry flag (prevent infinite loop)
                        if is_retry_attempt {
                            tracing::error!(
                                conversation_id = conversation_id.as_str(),
                                "Session recovery failed on retry, aborting"
                            );
                            // Fall through to normal error handling below
                        } else if let (Some(msg), Some(conv)) =
                            (user_message_content.as_ref(), conversation.as_ref())
                        {
                            // Attempt recovery (always enabled in production)
EOF
echo ""

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}[DRY RUN] Would remove feature flag check (lines 767-783)${NC}"
    echo -e "${YELLOW}[DRY RUN] Would simplify conditional logic${NC}"
    echo ""
    echo "To apply changes, run without --dry-run flag:"
    echo "  ./scripts/remove-recovery-feature-flag.sh"
    exit 0
fi

# Prompt for confirmation
echo -e "${YELLOW}This will make session recovery the DEFAULT behavior.${NC}"
echo -e "${YELLOW}Make sure you have verified success criteria before proceeding.${NC}"
echo ""
echo "Prerequisites checklist:"
echo "  [ ] Success rate ≥95% confirmed"
echo "  [ ] No critical bugs in testing period"
echo "  [ ] Team review completed"
echo "  [ ] Rollback plan ready"
echo ""
read -p "Do you want to proceed? (yes/no): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

# Apply the change
# Remove lines 767-783 (feature flag check and disabled branch)
# Keep the retry check and recovery logic
sed -i.tmp '
# Remove feature flag check lines (767-770)
/\/\/ Feature flag check/,/unwrap_or(false);/d
# Remove disabled branch lines (779-783)
/} else if !recovery_enabled {/,/\/\/ Fall through to clear session/d
' "$TARGET_FILE"

# Update comment on recovery branch
sed -i.tmp 's/\/\/ Attempt recovery$/\/\/ Attempt recovery (always enabled in production)/' "$TARGET_FILE"

# Clean up temp file
rm -f "${TARGET_FILE}.tmp"

echo ""
echo -e "${GREEN}✓ Feature flag removed successfully${NC}"
echo ""

# Show the result
echo -e "${BOLD}Updated code:${NC}"
echo ""
grep -A 12 "// Check retry flag" "$TARGET_FILE" | head -13
echo ""

# Verify the change
if grep -q "ENABLE_SESSION_RECOVERY" "$TARGET_FILE"; then
    echo -e "${RED}✗ Error: Feature flag still present in file${NC}"
    echo -e "${YELLOW}Restoring backup...${NC}"
    cp "$BACKUP_FILE" "$TARGET_FILE"
    exit 1
fi

echo -e "${GREEN}✓ Verification passed: Feature flag successfully removed${NC}"
echo ""

# Next steps
echo -e "${BOLD}Next steps:${NC}"
echo "1. Review the changes:"
echo "   git diff $TARGET_FILE"
echo ""
echo "2. Run tests to verify behavior:"
echo "   cargo test --package ralphx --lib -- chat_service"
echo ""
echo "3. Run type checking and linting:"
echo "   cd src-tauri && cargo clippy --all-targets --all-features"
echo ""
echo "4. Test manually with simulated stale session"
echo ""
echo "5. Commit the change:"
echo "   git add $TARGET_FILE"
echo "   git commit -m 'feat: enable session recovery by default (remove feature flag)'"
echo ""
echo "6. Remove .env file (no longer needed):"
echo "   rm .env"
echo ""
echo -e "${YELLOW}Backup saved at: $BACKUP_FILE${NC}"
echo "To restore backup if needed:"
echo "  cp $BACKUP_FILE $TARGET_FILE"
echo ""
