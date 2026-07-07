"""Minimal YAML-subset loader (this environment has no PyYAML / pip).

Supports exactly what the benchmark registry needs:

- nested mappings via 2-space indentation (``key:`` / ``key: value``)
- lists of scalars (``- item``) and lists of mappings (``- key: value``)
- scalars: int, float, bool, null, single/double-quoted and bare strings
- ``#`` comments and blank lines

If full YAML is ever needed, install PyYAML and swap ``load`` — the registry
file itself is valid standard YAML.
"""

from __future__ import annotations


def _scalar(s: str):
    s = s.strip()
    if not s or s in ("null", "~"):
        return None
    if s in ("true", "True"):
        return True
    if s in ("false", "False"):
        return False
    if (s[0] == s[-1] == '"') or (s[0] == s[-1] == "'"):
        return s[1:-1]
    try:
        return int(s)
    except ValueError:
        pass
    try:
        return float(s)
    except ValueError:
        pass
    return s


def _strip_comment(line: str) -> str:
    # A # starts a comment unless inside quotes.
    out = []
    quote = None
    for ch in line:
        if quote:
            out.append(ch)
            if ch == quote:
                quote = None
        elif ch in ("'", '"'):
            quote = ch
            out.append(ch)
        elif ch == "#":
            break
        else:
            out.append(ch)
    return "".join(out).rstrip()


def _lines(text: str):
    out = []
    for raw in text.splitlines():
        line = _strip_comment(raw)
        if not line.strip():
            continue
        indent = len(line) - len(line.lstrip(" "))
        out.append((indent, line.strip()))
    return out


def _parse_block(lines, pos, indent):
    """Parse the block starting at ``pos`` whose items sit at ``indent``."""
    if pos >= len(lines):
        return None, pos
    if lines[pos][1].startswith("- "):
        return _parse_list(lines, pos, indent)
    return _parse_map(lines, pos, indent)


def _parse_map(lines, pos, indent):
    obj = {}
    while pos < len(lines):
        ind, content = lines[pos]
        if ind < indent:
            break
        if ind > indent:
            raise ValueError(f"bad indentation at: {content!r}")
        if ":" not in content:
            raise ValueError(f"expected 'key:' at: {content!r}")
        key, _, rest = content.partition(":")
        key = _scalar(key)
        rest = rest.strip()
        pos += 1
        if rest:
            obj[key] = _scalar(rest)
        else:
            # Nested block (or empty value).
            if pos < len(lines) and lines[pos][0] > indent:
                obj[key], pos = _parse_block(lines, pos, lines[pos][0])
            else:
                obj[key] = None
    return obj, pos


def _parse_list(lines, pos, indent):
    items = []
    while pos < len(lines):
        ind, content = lines[pos]
        if ind < indent or not content.startswith("- "):
            break
        if ind > indent:
            raise ValueError(f"bad list indentation at: {content!r}")
        body = content[2:].strip()
        pos += 1
        if ":" in body and not (body[0] in "'\"" and body[-1] == body[0]):
            # List of mappings: re-inject the inline first pair, then any
            # continuation keys indented past the dash.
            inline = [(indent + 2, body)]
            while pos < len(lines) and lines[pos][0] >= indent + 2:
                inline.append(lines[pos])
                pos += 1
            item, _ = _parse_map(inline, 0, indent + 2)
            items.append(item)
        else:
            items.append(_scalar(body))
    return items, pos


def load(text: str):
    """Parse a YAML-subset document into dicts/lists/scalars."""
    lines = _lines(text)
    if not lines:
        return {}
    root, pos = _parse_block(lines, 0, lines[0][0])
    if pos != len(lines):
        raise ValueError(f"trailing content at: {lines[pos][1]!r}")
    return root


def load_file(path):
    with open(path, encoding="utf-8") as fh:
        return load(fh.read())
