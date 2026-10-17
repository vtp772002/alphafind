#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# AlphaFind Pre-Push Sanitization Gatekeeper
#
# Prevents accidental leaks of proprietary Alpha IDs, candidate formulas,
# downloaded research metadata, and secrets into tracked files or commits.
#
# Coverage notes (read before trusting a PASS):
#   - Scans tracked files under src, tests, docs, data, and scripts (excluding
#     this script's own pattern list).
#   - Scans added lines in outgoing commits and the current staged diff.
#   - Candidate names/expressions are loaded from the ignored local store at
#     data/private_candidates.json. When that file is absent (for example on a
#     fresh clone or CI), the expression scan cannot run and says so.
#   - It is a signal, not a proof: a formula rewritten by hand or a private
#     artifact stored elsewhere will not be detected.
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
    "data/private_candidates.json"
)

for file in "${SENSITIVE_FILES[@]}"; do
    if git ls-files --error-unmatch "$file" 2>/dev/null; then
        echo -e "${RED}❌ ERROR: Proprietary file '$file' is tracked in Git!${NC}"
        echo "   Remove from Git tracking: git rm --cached $file"
        exit 1
    fi
done

# 1b. Downloaded dataset/field metadata must stay in the ignored local store.
TRACKED_FIELD_METADATA=$(git ls-files "data/fields_*.json" 2>/dev/null || true)
if [ -n "$TRACKED_FIELD_METADATA" ]; then
    echo -e "${RED}❌ ERROR: Downloaded field metadata is tracked in Git:${NC}"
    echo "$TRACKED_FIELD_METADATA"
    echo "   These files name the exact fields under research. Keep them local:"
    echo "   git rm --cached data/fields_*.json && add 'data/fields_*.json' to .gitignore"
    exit 1
fi

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

if [ ${#ALPHA_IDS[@]} -eq 0 ]; then
    echo -e "${YELLOW}⚠️  NOTE: No local Alpha-ID source found (submittable_alphas.csv / portfolio_os.json).${NC}"
    echo "   The Alpha-ID scan is skipped for this run. Do not treat this as clearance."
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

# 4b. Scan commit history for Alpha IDs.
#
# The range check above covers only the outgoing diff, which on a repository without
# an upstream collapses to the last commit. An ID committed earlier therefore slips
# through, so every commit's added lines are scanned here as well.
if [ ${#ALPHA_IDS[@]} -gt 0 ]; then
    HISTORY_ADDED=$(git log -p --all --format="%H" 2>/dev/null | grep -E "^\+[^+]" || true)
    HISTORY_HITS=0
    if [ -n "$HISTORY_ADDED" ]; then
        for aid in "${ALPHA_IDS[@]}"; do
            if echo "$HISTORY_ADDED" | grep -q "$aid"; then
                HISTORY_HITS=$((HISTORY_HITS + 1))
            fi
        done
    fi

    if [ "$HISTORY_HITS" -gt 0 ]; then
        if [ "${ALPHAFIND_STRICT_HISTORY:-0}" = "1" ]; then
            echo -e "${RED}❌ ERROR: $HISTORY_HITS Alpha ID(s) found in commit-history added lines.${NC}"
            echo "   A new commit cannot remove them: rewrite history or rotate the exposed alphas."
            echo "   See scratch/purge_history.sh for the purge workflow."
            exit 1
        fi
        echo -e "${YELLOW}⚠️  WARNING: $HISTORY_HITS Alpha ID(s) found in commit-history added lines.${NC}"
        echo "   Published history keeps them; rewrite history or rotate the exposed alphas."
        echo "   Set ALPHAFIND_STRICT_HISTORY=1 to make this a hard failure."
    else
        echo -e "${GREEN}   ✓ Commit-history scan: no Alpha IDs in added lines.${NC}"
    fi
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

# 5. Check for concrete proprietary dataset fields across tracked source,
#    tests, docs, and local metadata.
#
#    The pattern list lives in an ignored local store so this script does not
#    publish the very field names it protects.
FIELD_PATTERN_STORE="data/sanitize_field_patterns.txt"
FIELD_PATTERNS=""
if [ -f "$FIELD_PATTERN_STORE" ]; then
    FIELD_PATTERNS=$(grep -v '^[[:space:]]*$' "$FIELD_PATTERN_STORE" | paste -sd'|' - || true)
else
    echo -e "${YELLOW}⚠️  NOTE: $FIELD_PATTERN_STORE not found; the explicit dataset-field scan was skipped.${NC}"
    echo "   The candidate artifact scan (section 6) remains the primary check."
fi

if [ -n "$FIELD_PATTERNS" ]; then
    # Scan every tracked file (root docs included) except this script, which now
    # only holds the loader rather than the pattern list itself.
    MATCHES=$(git grep -En "$FIELD_PATTERNS" -- . ':!scripts/sanitize_check.sh' 2>/dev/null || true)
    if [ -n "$MATCHES" ]; then
        echo -e "${RED}❌ ERROR: Concrete dataset fields found in tracked files!${NC}"
        echo "$MATCHES"
        echo "   Use abstract tokens like <SHORT_FIELD>, <NEWS_SENTIMENT>, <IDIO_RISK_TERM>, <IMPLIED_VOL>."
        exit 1
    fi
    echo -e "${GREEN}   ✓ Dataset-field scan: all tracked files are clear of configured field patterns.${NC}"
fi

# 6. Check that no candidate from the local private store appears in a tracked file.
PRIVATE_STORE="data/private_candidates.json"
if [ -f "$PRIVATE_STORE" ]; then
    PRIVATE_LITERALS=$(grep -oE '"(name|expression)": *"[^"]{12,}"' "$PRIVATE_STORE" | awk -F'"' 'NF >= 4 { print $4 }' || true)
    PRIVATE_LITERAL_COUNT=$(printf '%s\n' "$PRIVATE_LITERALS" | grep -c . || true)
    while IFS= read -r candidate_literal; do
        [ -z "$candidate_literal" ] && continue
        if git grep -Fq -- "$candidate_literal" 2>/dev/null; then
            echo -e "${RED}❌ ERROR: A private candidate artifact from $PRIVATE_STORE is present in tracked files!${NC}"
            echo "   Offending literal (truncated): ${candidate_literal:0:40}..."
            echo "   Keep candidate names and formulas in the local store only."
            exit 1
        fi
    done <<< "$PRIVATE_LITERALS"
    echo -e "${GREEN}   ✓ Candidate artifact scan: ${PRIVATE_LITERAL_COUNT} private literals absent from tracked files.${NC}"
else
    echo -e "${YELLOW}⚠️  NOTE: $PRIVATE_STORE not found; the candidate artifact scan was skipped.${NC}"
fi

echo -e "${GREEN}✅ [SANITIZATION GATEKEEPER] Pre-Push Audit PASSED. Safe to push.${NC}"
exit 0
