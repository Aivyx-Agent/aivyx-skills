# aivyx-skills

A default, system-level library of Claude-Skills-format
(`SKILL.md`) capability packages, shared by both `aivyx-pa` (the flagship
Personal Assistant) and `aivyx-coder` (the standalone terminal coding
agent) — one place to author, version, and ship the process/methodology
guidance both products bundle by default, instead of each maintaining its
own copy.

Distinct from two other, unrelated "skill"-shaped concepts already in this
workspace: `aivyx-pa`'s own agent-learned `Skill`/`LearnedSkill` system
(Praxis/Whetstone/Repertoire — procedures the agent authors or is taught
at runtime, tracked with an effectiveness ledger) and `aivyx-hub`'s
"vertical packs" (large, signed, downloadable domain extensions). This
crate is neither — it's a small, static, compiled-in default library, the
same shape as `aivyx-confine`/`aivyx-checkpoint`/`aivyx-kvcache`/
`aivyx-injection-guard`: a focused primitive both products depend on
directly rather than duplicate.

See `docs/superpowers/specs/2026-09-23-aivyx-skills-design.md` for the
full design rationale.

## Status

Implemented and tested: frontmatter parsing, bundled-skill embedding (5
real default skills shipped), and project/user overlay directory
resolution (project > user > bundled priority) are all done. Consumption
by `aivyx-pa` and `aivyx-coder` — wiring this crate into each product's
own skill-discovery/system-prompt flow — is separate, not-yet-started
work (Parts 2/3 of the larger initiative this crate is Part 1 of).

## Usage

```rust
use aivyx_skills::SkillLoader;

let loader = SkillLoader::new()
    // Optional: layer in project/user overrides. Each dir must directly
    // contain `<skill-name>/SKILL.md` subdirectories.
    .with_project_dir(std::path::PathBuf::from(".aivyx/skills"));

for summary in loader.list() {
    println!("{}: {}", summary.name, summary.description);
}

if let Some(skill) = loader.get("systematic-debugging") {
    println!("{}", skill.body);
}
```

## SKILL.md frontmatter format

The frontmatter parser is a deliberately minimal, dependency-free
line-based parser for exactly two required scalar fields (`name`,
`description`) — not a general YAML parser. Supported scalar styles:

- Plain, unquoted: `description: A description.`
- Double-quoted: `description: "A description."` — the surrounding
  quotes are stripped, and `\"` / `\\` are unescaped (the only two
  escapes supported).
- Single-quoted: `description: 'A description.'` — the surrounding
  quotes are stripped, with no escape processing.
- Folded block scalar: `description: >` followed by one or more
  more-indented continuation lines, joined into a single line with
  spaces.
- Literal block scalar: `description: |` followed by one or more
  more-indented continuation lines, joined with `\n`, preserving line
  breaks.

The closing `---` delimiter may or may not be followed by a trailing
newline — a file that ends exactly at `---` with no final `\n` is
accepted (with an empty body), not rejected. Unknown frontmatter fields
(anything besides `name`/`description`) are tolerated and ignored.

Each overlay `SKILL.md` is only read if it is a regular file (a
symlink to one is fine; a symlink to a device or a FIFO is skipped, not
opened) and only up to a 256 KiB cap — see `read_bounded_regular_file`
in `src/loader.rs`. A skipped entry logs a warning and is simply absent
from `list()`/`get()`, same as any other malformed overlay skill.

## Known limitations

- Overlay directories read arbitrary local filesystem content. This
  crate does not treat that *content* as untrusted — whichever repo
  eventually builds the Part 2/3 consumption code should decide whether
  overlay skill content needs sanitizing before it's injected into an
  LLM context. (This is distinct from the file-*type*/size robustness
  above, which this crate does handle itself.)
- `SkillLoader` re-scans and re-parses everything (bundled + overlays) on
  every `list()`/`get()` call; there's no caching. Fine at today's scale
  (5 bundled skills plus a small overlay), but worth revisiting if this
  crate is ever used on a hot path.

## License

Source-available under BUSL-1.1 — see `LICENSE`.
