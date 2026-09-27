#!/usr/bin/env bash
# Pinned, optional local code navigation. No model calls or agent-config writes.
set -euo pipefail
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
revision=80692e5ad0bc8e8f7e1edea648247d90b9f76820
archive_sha256=15b611f363cea321649de3b8a570d82da891a81dd22f83a2f19b73e28e84dad6
tool_dir="$repo_root/.cache/graft-toolchain/$revision"
export DO_NOT_TRACK=1 CI=1 GRAFT_NO_GITIGNORE=1 GRAFT_NO_IGNORE=1
export GRAFT_REFRESH=hash

if [[ ${1:-} == setup ]]; then
    if [[ ! -d "$tool_dir" ]]; then
        mkdir -p "$(dirname "$tool_dir")"
        task_tmp=$(mktemp -d "$(dirname "$tool_dir")/download.XXXXXX")
        trap 'rm -rf -- "$task_tmp"' EXIT
        curl --fail --location --retry 2 \
            "https://codeload.github.com/trailhq/Graft/tar.gz/$revision" \
            --output "$task_tmp/source.tar.gz"
        printf '%s  %s\n' "$archive_sha256" "$task_tmp/source.tar.gz" | sha256sum --check --strict
        tar -xzf "$task_tmp/source.tar.gz" -C "$task_tmp"
        mv -- "$task_tmp/Graft-$revision" "$tool_dir"
    fi
    (cd "$tool_dir" && npm ci --no-audit --no-fund)
    node "$tool_dir/dist/cli.js" --version
    exit
fi

if [[ ! -f "$tool_dir/dist/cli.js" ]]; then
    printf '%s\n' 'Graft is not installed. Run: bash scripts/graft.sh setup' >&2
    exit 1
fi

# Keep integration narrower than upstream init/deep/upgrade.
if (( $# == 0 )); then set -- --help; fi
case $1 in
    build|ask|skeleton|check|grep|map|callers|--help|--version) ;;
    *) printf '%s\n' 'Supported: setup, build, ask, skeleton, check, grep, map, callers, --help, --version' >&2; exit 2 ;;
esac
for arg in "$@"; do
    case "$arg" in
        --deep|--deep=*|--dir|--dir=*|--provider|--provider=*|--model|--model=*|--api-key|--api-key=*|--base-url|--base-url=*|--only-dir|--only-dir=*|--include-dir|--include-dir=*|--follow-submodules|--no-follow-submodules|--follow-nested-repos|--no-follow-nested-repos)
            printf 'Option outside this local structural adapter: %s\n' "$arg" >&2
            exit 2 ;;
    esac
done

# An alternate Git root is used by the isolated compatibility test.
project_root=$(cd "${ISING_GRAFT_ROOT:-$repo_root}" && pwd)
cd "$project_root"
index_dir="$project_root/.cache/graft-index"
if [[ $1 == --help || $1 == --version ]]; then
    exec node "$tool_dir/dist/cli.js" --dir "$index_dir" "$@"
fi
mkdir -p "$index_dir"
# Serialize this adapter's readers/builders; upstream refresh also writes caches.
exec 9>"$index_dir/adapter.lock"
flock 9
task_list=$(mktemp "$index_dir/files.XXXXXX")
trap 'rm -f -- "$task_list"' EXIT
python3 - > "$task_list" <<'PY'
import pathlib, subprocess, sys
names = subprocess.check_output([
    'git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'
]).decode().split('\0')
extensions = {'.rs', '.py', '.pyi', '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.mts', '.cts'}
for name in sorted(set(names)):
    p = pathlib.Path(name)
    if (name and not name.startswith('benchmarks/qoblib/upstream/')
            and p.suffix in extensions and p.is_file() and not p.is_symlink()
            and not any(part.startswith('.') for part in p.parts)):
        sys.stdout.buffer.write(name.encode() + b'\0')
PY
mapfile -d '' -t source_files < "$task_list"
if (( ${#source_files[@]} == 0 )); then
    printf '%s\n' 'No supported visible source files in this Git root.' >&2
    exit 1
fi
# Upstream --extensions does not filter structural builds at this revision.
# Explicit file prefixes prevent .sol solution artifacts becoming Solidity code.
# Recompute membership on every call so new/untracked/deleted code is reflected.
if [[ $1 == build ]] || ! cmp -s "$task_list" "$index_dir/adapter-files" \
        || [[ ! -f "$index_dir/.graph/wiring.json" ]]; then
    build_args=()
    for path in "${source_files[@]}"; do build_args+=(--only-dir "$path"); done
    if [[ $1 == build ]]; then
        shift
        node "$tool_dir/dist/cli.js" --dir "$index_dir" build "$@" "$project_root" "${build_args[@]}"
        mv -- "$task_list" "$index_dir/adapter-files"
        exit
    fi
    node "$tool_dir/dist/cli.js" --dir "$index_dir" build "$project_root" "${build_args[@]}" >&2
    mv -- "$task_list" "$index_dir/adapter-files"
fi
rm -f -- "$task_list"
trap - EXIT
exec node "$tool_dir/dist/cli.js" --dir "$index_dir" "$@" "$project_root"
