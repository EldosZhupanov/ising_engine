#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

fail() {
    printf 'memory-doc check failed: %s\n' "$*" >&2
    exit 1
}

require_file() {
    [[ -f "$1" ]] || fail "required file is missing: $1"
}

require_file START_HERE.md
require_file memory/NOW.md
require_file memory/AUTHORITY.md
require_file memory/CATALOG.md
require_file memory/BINDING_SHA256

agents_bytes=$(wc -c < AGENTS.md)
(( agents_bytes <= 20480 )) || fail "AGENTS.md is ${agents_bytes} bytes; limit is 20480"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

git ls-files --cached --others --exclude-standard -- '*.md' | sort > "$tmp_dir/actual-md"
sed -n 's/^| `\([^`]*\)` |.*/\1/p' memory/CATALOG.md | sort > "$tmp_dir/catalog-md"

duplicates=$(uniq -d "$tmp_dir/catalog-md")
[[ -z "$duplicates" ]] || fail "duplicate catalogue paths: $duplicates"

if ! diff -u "$tmp_dir/actual-md" "$tmp_dir/catalog-md" > "$tmp_dir/catalog.diff"; then
    cat "$tmp_dir/catalog.diff" >&2
    fail 'CATALOG.md does not cover every Markdown file exactly once'
fi

for lifecycle in active binding closed historical proposed reference superseded; do
    declared=$(awk -v state="$lifecycle" '$1 == "-" && $2 == "`" state "`:" {print $3}' memory/CATALOG.md)
    actual=$(awk -F'|' -v state="$lifecycle" '{value=$5; gsub(/^ +| +$/, "", value); if (value == state) count++} END {print count + 0}' memory/CATALOG.md)
    [[ "$declared" == "$actual" ]] \
        || fail "catalogue summary for $lifecycle says ${declared:-missing}, registry has $actual"
done

: > "$tmp_dir/live-authority"
while IFS= read -r markdown; do
    if grep -Eq '^authority_scope: current-task$' "$markdown"; then
        printf '%s\n' "$markdown" >> "$tmp_dir/live-authority"
    fi
done < "$tmp_dir/actual-md"
sort -o "$tmp_dir/live-authority" "$tmp_dir/live-authority"
printf '%s\n' memory/NOW.md > "$tmp_dir/expected-live-authority"
if ! diff -u "$tmp_dir/expected-live-authority" "$tmp_dir/live-authority"; then
    fail 'memory/NOW.md must be the sole current-task authority'
fi

catalogue_live=$(awk -F'|' '$9 ~ /current-task/ {gsub(/^ +| +$/, "", $2); gsub(/`/, "", $2); print $2}' memory/CATALOG.md)
[[ "$catalogue_live" == 'memory/NOW.md' ]] \
    || fail 'catalogue must name only memory/NOW.md for current-task'

for pointer in memory/CURRENT_TASK.md memory/CURRENT_HANDOFF.md; do
    grep -Eq 'compatibility pointer' "$pointer" \
        || fail "$pointer is not marked as a compatibility pointer"
    grep -Eq 'NOW\.md`?\]\(NOW\.md\)' "$pointer" \
        || fail "$pointer does not redirect to memory/NOW.md"
done

bootstrap_files=(
    AGENTS.md
    CLAUDE.md
    START_HERE.md
    PROJECT_PLAN.md
    ROADMAP.md
    PRODUCT_SPEC.md
    memory/NOW.md
    memory/AUTHORITY.md
    memory/INDEX.md
)
stale_pattern='PRE-REGISTERED ONLY|No RC-021 instrument exists|14 operators|280 tests|CURRENT_HANDOFF\.md.*mandatory|read memory/CURRENT_HANDOFF'
if grep -Eni -- "$stale_pattern" "${bootstrap_files[@]}"; then
    fail 'an active bootstrap document contains a retired state marker'
fi

awk -F'|' '$8 ~ /true/ {
    gsub(/^ +| +$/, "", $2)
    gsub(/`/, "", $2)
    print $2
}' memory/CATALOG.md | sort > "$tmp_dir/catalogue-immutable"
awk '{print $2}' memory/BINDING_SHA256 | sort > "$tmp_dir/manifest-immutable"
manifest_duplicates=$(uniq -d "$tmp_dir/manifest-immutable")
[[ -z "$manifest_duplicates" ]] \
    || fail "duplicate binding checksum paths: $manifest_duplicates"
if ! diff -u "$tmp_dir/catalogue-immutable" "$tmp_dir/manifest-immutable"; then
    fail 'binding checksum manifest and immutable catalogue rows differ'
fi
sha256sum --check --strict memory/BINDING_SHA256 > /dev/null \
    || fail 'a binding or closed document changed bytes'

comparison_base=${1:-HEAD^}
git rev-parse --verify --quiet "${comparison_base}^{commit}" > /dev/null \
    || fail "cannot resolve comparison base: $comparison_base"
cp "$tmp_dir/catalogue-immutable" "$tmp_dir/protected-candidates"
if git cat-file -e "${comparison_base}:memory/CATALOG.md" 2>/dev/null; then
    git show "${comparison_base}:memory/CATALOG.md" \
        | awk -F'|' '$8 ~ /true/ {
            gsub(/^ +| +$/, "", $2)
            gsub(/`/, "", $2)
            print $2
        }' >> "$tmp_dir/protected-candidates"
fi

# Bootstrap floor: the first catalogue commit has no catalogue at its base.
# Derive already-frozen research paths independently from their stable naming
# contract and, for ADRs, from the status committed at the comparison base.
# This prevents an author from unfreezing evidence by editing CATALOG.md and the
# checksum manifest in the same change.
while IFS= read -r path; do
    case "$path" in
        research/PREREG_*.md|\
        research/RC[0-9][0-9][0-9]_*.md|\
        research/*AMENDMENT*.md|\
        research/*RECORD*.md|\
        research/*RESULTS*.md|\
        research/EXTERNAL_COMPARISON_PROTOCOL.md|\
        research/ISING_ENGINE_CONSTITUTION.md)
            printf '%s\n' "$path" >> "$tmp_dir/protected-candidates"
            ;;
        research/architecture/ADR/ADR-*.md)
            if git show "${comparison_base}:$path" \
                | grep -Eq '^status:[[:space:]]*accepted[[:space:]]*$'; then
                printf '%s\n' "$path" >> "$tmp_dir/protected-candidates"
            fi
            ;;
    esac
done < <(git ls-tree -r --name-only "$comparison_base" -- research)
sort -u -o "$tmp_dir/protected-candidates" "$tmp_dir/protected-candidates"
: > "$tmp_dir/protected-at-base"
while IFS= read -r path; do
    if git cat-file -e "${comparison_base}:$path" 2>/dev/null; then
        printf '%s\n' "$path" >> "$tmp_dir/protected-at-base"
    fi
done < "$tmp_dir/protected-candidates"
git diff --name-only "$comparison_base" -- | sort > "$tmp_dir/changed-since-base"
comm -12 "$tmp_dir/protected-at-base" "$tmp_dir/changed-since-base" \
    > "$tmp_dir/changed-protected"
if [[ -s "$tmp_dir/changed-protected" ]]; then
    cat "$tmp_dir/changed-protected" >&2
    fail "an immutable file differs from comparison base $comparison_base"
fi

: > "$tmp_dir/links"
while IFS= read -r markdown; do
    if grep -En '\[\[[^]]+\]\]' "$markdown"; then
        fail "unsupported or unresolved wiki-link in $markdown; use a standard Markdown link"
    fi
    perl -ne 'while (/\[[^\]]*\]\(([^)]+)\)/g) { print "$ARGV\t$1\n" }' "$markdown" \
        >> "$tmp_dir/links"
done < "$tmp_dir/actual-md"
while IFS=$'\t' read -r source target; do
    case "$target" in
        http://*|https://*|mailto:*|'#'*) continue ;;
    esac
    target=${target%%#*}
    target=${target#<}
    target=${target%>}
    [[ -e "$(dirname "$source")/$target" ]] \
        || fail "broken local Markdown link: $source -> $target"
done < "$tmp_dir/links"

printf 'memory-doc check passed: %s Markdown files, %s immutable files, AGENTS.md %s bytes\n' \
    "$(wc -l < "$tmp_dir/actual-md")" \
    "$(wc -l < "$tmp_dir/catalogue-immutable")" \
    "$agents_bytes"
