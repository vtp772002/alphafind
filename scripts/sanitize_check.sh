#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# AlphaFind Pre-Push Sanitization Gatekeeper
# Prevents accidental leaks of proprietary Alpha IDs, FastExpr formulas, and secrets.
# ==============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}🔍 [SANITIZATION GATEKEEPER] Running Pre-Push Audit...${NC}"

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# 1. Check for staged/tracked sensitive files
SENSITIVE_FILES=(
    ".env"
    "submittable_alphas.csv"
    "portfolio_os.json"
    "STRATEGIC_ROADMAP.md"
)

for file in "${SENSITIVE_FILES[@]}"; do
    if git ls-files --error-unmatch "$file" 2>/dev/null; then
        echo -e "${RED}❌ ERROR: Proprietary file '$file' is tracked in Git!${NC}"
        echo "   Remove from Git tracking: git rm --cached $file"
        exit 1
    fi
done

# 2. Check for tracked scratch scripts
SCRATCH_TRACKED=$(git ls-files "scratch/*" 2>/dev/null || true)
if [ -n "$SCRATCH_TRACKED" ]; then
    echo -e "${RED}❌ ERROR: Temporary scratch scripts are tracked in Git:${NC}"
    echo "$SCRATCH_TRACKED"
    exit 1
fi

# 3. Collect Alpha IDs to guard against leaking
ALPHA_IDS=()
if [ -f "submittable_alphas.csv" ]; then
    while IFS=, read -r alpha_id rest || [ -n "$alpha_id" ]; do
        # Match exactly 8 alphanumeric chars
        if [[ "$alpha_id" =~ ^[A-Za-z0-9]{8}$ ]]; then
            ALPHA_IDS+=("$alpha_id")
        fi
    done < submittable_alphas.csv
fi

if [ -f "portfolio_os.json" ]; then
    while read -r alpha_id || [ -n "$alpha_id" ]; do
        if [[ "$alpha_id" =~ ^[A-Za-z0-9]{8}$ ]]; then
            ALPHA_IDS+=("$alpha_id")
        fi
    done < <(grep -oE '"id":\s*"[A-Za-z0-9]{8}"' portfolio_os.json | sed -E 's/.*"([A-Za-z0-9]{8})".*/\1/' | sort -u)
fi

# Deduplicate ALPHA_IDS
if [ ${#ALPHA_IDS[@]} -gt 0 ]; then
    ALPHA_IDS=($(printf "%s\n" "${ALPHA_IDS[@]}" | sort -u))
fi

# Also check outgoing commits against remote upstream or recent commit
RANGE=""
if git rev-parse --verify @{u} >/dev/null 2>&1; then
    RANGE="@{u}..HEAD"
else
    RANGE="HEAD~1..HEAD"
fi

# Check only added lines in diff to prevent false positives when removing an ID
DIFF_ADDED=$(git diff "$RANGE" 2>/dev/null | grep -E "^\+[^+]" || git diff --cached 2>/dev/null | grep -E "^\+[^+]" || true)

if [ -n "$DIFF_ADDED" ] && [ ${#ALPHA_IDS[@]} -gt 0 ]; then
    for aid in "${ALPHA_IDS[@]}"; do
        if echo "$DIFF_ADDED" | grep -q "$aid"; then
            echo -e "${RED}❌ ERROR: Alpha ID '$aid' detected in added lines of outgoing commits/diff!${NC}"
            echo "   Never include real platform Alpha IDs in tracked code, tests, or docs."
            exit 1
        fi
    done
fi

# Also check currently tracked files
if [ ${#ALPHA_IDS[@]} -gt 0 ]; then
    for aid in "${ALPHA_IDS[@]}"; do
        if git grep -q "$aid" 2>/dev/null; then
            echo -e "${RED}❌ ERROR: Alpha ID '$aid' detected in tracked source files!${NC}"
            exit 1
        fi
    done
fi

# 4. Check for commit message leaks in outgoing commits
if [ -n "$RANGE" ]; then
    COMMIT_MSGS=$(git log "$RANGE" --format="%B" 2>/dev/null || true)
    if [ -n "$COMMIT_MSGS" ] && [ ${#ALPHA_IDS[@]} -gt 0 ]; then
        for aid in "${ALPHA_IDS[@]}"; do
            if echo "$COMMIT_MSGS" | grep -q "$aid"; then
                echo -e "${RED}❌ ERROR: Alpha ID '$aid' detected in commit message!${NC}"
                echo "   Sanitize commit messages with 'git commit --amend'."
                exit 1
            fi
        done
    fi
fi

# 5. Check for hardcoded proprietary dataset fields across all tracked source and tests
PROPRIETARY_PATTERNS="<REDACTED_FIELD>|<REDACTED_FIELD>|<REDACTED_FIELD>|<REDACTED_FIELD>|<REDACTED_FIELD>|<REDACTED_FIELD>|<REDACTED_FIELD>"
SCAN_TARGETS=(src tests docs)
for target in "${SCAN_TARGETS[@]}"; do
    if [ -d "$target" ]; then
        MATCHES=$(git grep -En "$PROPRIETARY_PATTERNS" -- "$target" 2>/dev/null || true)
        if [ -n "$MATCHES" ]; then
            echo -e "${RED}❌ ERROR: Concrete dataset fields found in $target!${NC}"
            echo "$MATCHES"
            echo "   Use abstract tokens like <SHORT_FIELD>, <NEWS_SENTIMENT>, <IDIO_RISK_TERM>, <IMPLIED_VOL>."
            exit 1
        fi
    fi
done

echo -e "${GREEN}✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.${NC}"
exit 0
