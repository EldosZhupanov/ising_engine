#!/usr/bin/env python3
"""Query the architecture knowledge graph embedded in ADR frontmatter.

The graph is files-as-database (ADR-0000): every ADR under research/architecture/ADR/
carries `edges:` triples in its YAML frontmatter. This tool extracts and queries them.
Stdlib only — runs on the bare system python3.

Usage:
  query_graph.py list                 # all ADRs: id, status, title
  query_graph.py edges <Node>         # all edges touching a node
  query_graph.py why <Node>           # rationale trace: outgoing chains + defining ADRs
  query_graph.py nodes                # all known nodes
"""

import re
import sys
from pathlib import Path

ADR_DIR = Path(__file__).resolve().parent / "ADR"

EDGE_RE = re.compile(r"^\s*-\s*\[\s*([^,\]]+?)\s*,\s*([^,\]]+?)\s*,\s*([^,\]]+?)\s*\]\s*$")
META_RE = re.compile(r"^(id|title|status):\s*(.+?)\s*$")


def load():
    """Return (adrs: {id: {title,status,file}}, edges: [(subj, pred, obj, adr_id)])."""
    adrs, edges = {}, []
    for path in sorted(ADR_DIR.glob("ADR-*.md")):
        meta, in_fm, fm_seen = {}, False, 0
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.strip() == "---":
                fm_seen += 1
                in_fm = fm_seen == 1
                if fm_seen == 2:
                    break
                continue
            if not in_fm:
                continue
            m = META_RE.match(line)
            if m:
                meta[m.group(1)] = m.group(2).strip('"')
            e = EDGE_RE.match(line)
            if e:
                edges.append((e.group(1), e.group(2), e.group(3), meta.get("id", path.stem)))
        aid = meta.get("id", path.stem)
        adrs[aid] = {"title": meta.get("title", "?"), "status": meta.get("status", "?"), "file": path.name}
    return adrs, edges


def cmd_list(adrs, _edges):
    for aid, m in adrs.items():
        print(f"{aid}  [{m['status']:9s}]  {m['title']}")


def cmd_nodes(_adrs, edges):
    nodes = sorted({n for s, _, o, _ in edges for n in (s, o)})
    for n in nodes:
        print(n)


def match(node, name):
    return node.lower() == name.lower()


def cmd_edges(_adrs, edges, node):
    hits = [(s, p, o, a) for s, p, o, a in edges if match(s, node) or match(o, node)]
    if not hits:
        sys.exit(f"no edges touch '{node}' (try: query_graph.py nodes)")
    for s, p, o, a in hits:
        print(f"{s} --{p}--> {o}   ({a})")


def cmd_why(adrs, edges, node):
    """Trace rationale: follow outgoing edges from node, then edges of what it reaches."""
    canonical = {n.lower(): n for s, _, o, _ in edges for n in (s, o)}
    if node.lower() not in canonical:
        sys.exit(f"unknown node '{node}' (try: query_graph.py nodes)")
    node = canonical[node.lower()]
    print(f"WHY {node}\n{'=' * (4 + len(node))}")
    defined_in = sorted({a for s, _, _, a in edges if match(s, node)})
    if defined_in:
        print("Defined/used by: " + ", ".join(f"{a} ({adrs[a]['file']})" for a in defined_in if a in adrs))
    seen, frontier, depth = {node}, [node], 0
    while frontier and depth < 4:
        depth += 1
        nxt = []
        for cur in frontier:
            for s, p, o, a in edges:
                if match(s, cur) and o not in seen:
                    print(f"{'  ' * depth}{cur} --{p}--> {o}   [{a}]")
                    seen.add(o)
                    nxt.append(o)
        frontier = nxt
    incoming = [(s, p, a) for s, p, o, a in edges if match(o, node)]
    if incoming:
        print("Depends on it / justified by it:")
        for s, p, a in incoming:
            print(f"  {s} --{p}--> {node}   [{a}]")


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in {"list", "edges", "why", "nodes"}:
        sys.exit(__doc__)
    adrs, edges = load()
    cmd = sys.argv[1]
    if cmd == "list":
        cmd_list(adrs, edges)
    elif cmd == "nodes":
        cmd_nodes(adrs, edges)
    else:
        if len(sys.argv) < 3:
            sys.exit(f"usage: query_graph.py {cmd} <Node>")
        (cmd_edges if cmd == "edges" else cmd_why)(adrs, edges, sys.argv[2])


if __name__ == "__main__":
    main()
