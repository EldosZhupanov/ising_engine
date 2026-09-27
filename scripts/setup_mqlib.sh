#!/usr/bin/env bash
# Optional isolated dependency. Never installs packages or modifies production.
set -euo pipefail
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cache="$repo/.cache/mqlib-qualification"
sha=585496274af5abb0849d0d47e135496b4688680b
mkdir -p "$cache"
exec 9>"$cache/setup.lock"
flock 9
if [[ ! -d "$cache/upstream" ]]; then
    git clone --no-checkout https://github.com/MQLib/MQLib.git "$cache/upstream"
    git -C "$cache/upstream" checkout --detach "$sha"
fi
[[ $(git -C "$cache/upstream" rev-parse HEAD) == "$sha" ]]
[[ -z $(git -C "$cache/upstream" status --porcelain) ]]
# Rebuild, rather than associate an old executable with a newly recorded source.
make -C "$cache/upstream" -B -j2 CXX=g++ >"$cache/build.log" 2>&1
g++ -std=c++11 -O2 -Wall -Wextra -Werror -I"$cache/upstream/include" \
    "$repo/research/mqlib_qualification/oracle.cpp" \
    "$cache/upstream/bin/MQLib.a" -o "$cache/oracle" >"$cache/oracle-build.log" 2>&1
python3 - "$repo" "$cache" "$sha" <<'PY'
import hashlib, json, pathlib, subprocess, sys
root, cache = map(pathlib.Path, sys.argv[1:3])
paths = [cache/'upstream/bin/MQLib', cache/'upstream/bin/MQLib.a', cache/'oracle',
         cache/'upstream/Makefile', root/'research/mqlib_qualification/oracle.cpp',
         root/'scripts/setup_mqlib.sh']
data = {'upstream_commit': sys.argv[3], 'compiler': subprocess.check_output(['g++','--version'],text=True),
        'make_command': 'make -B -j2 CXX=g++',
        'oracle_flags': '-std=c++11 -O2 -Wall -Wextra -Werror',
        'build_log': (cache/'build.log').read_text(),
        'oracle_build_log': (cache/'oracle-build.log').read_text(),
        'hashes': {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}}
(cache/'build.json').write_text(json.dumps(data,indent=2)+'\n')
print('Built pinned MQLib and exhaustive oracle; provenance: '+str(cache/'build.json'))
PY
