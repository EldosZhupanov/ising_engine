# Obsidian workspace

Obsidian is an optional local interface over this Git repository. It is useful
for navigation and backlinks, but it is not a second memory store and has no
authority of its own.

## Setup

1. Open the repository root as the vault. Do not copy `memory/` into a separate
   vault.
2. Pin [`START_HERE.md`](../START_HERE.md) as the home note.
3. Use [`NOW.md`](NOW.md) for the current task, [`AUTHORITY.md`](AUTHORITY.md)
   for conflict resolution, [`CATALOG.md`](CATALOG.md) for document discovery,
   and [`TIMELINE.md`](TIMELINE.md) for chronology.
4. Keep Obsidian's generated `.obsidian/` directory local. It is ignored by Git
   because layouts, caches, plugin state, and device settings are not project
   evidence.

## Rules

- Edit canonical files in place; never create an Obsidian-only duplicate.
- Do not enable a plugin that automatically commits, rewrites links, renames
  files, or synchronizes a second copy of the vault.
- Do not move or rename any document marked `immutable: true` in the catalogue.
- Graph links, tags, search indexes, canvases, and AI summaries are views. When
  they disagree with a canonical document, the canonical document wins.
- A useful conclusion discovered through a graph or search becomes durable only
  after it is written to the correct Git-tracked document and reviewed normally.

No Obsidian installation is required for agents or CI. Plain Markdown, Git, and
the validation script remain the portable storage and verification layer.
