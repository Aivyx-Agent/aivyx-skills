# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## What this is

`aivyx-skills` is a small, default, system-level library of
`SKILL.md`-format capability packages, shared by both `aivyx-pa` (the
flagship Personal Assistant) and `aivyx-coder` (the standalone terminal
coding agent). It exists so both products bundle the same default
process/methodology skills (e.g. systematic debugging, brainstorming and
scoping) rather than each maintaining — and potentially drifting on — its
own copy. Same rationale, same pattern, as `aivyx-confine`/
`aivyx-checkpoint`/`aivyx-kvcache`/`aivyx-injection-guard`.

**Not** `aivyx-pa`'s own agent-learned `Skill`/`LearnedSkill` system
(Praxis/Whetstone/Repertoire, in `aivyx-pa`'s own repo) — that's
procedures the agent authors or is taught at runtime, tracked with an
effectiveness ledger. This crate is a static, compiled-in default
library, not a learning system. Also not `aivyx-hub`'s "vertical packs"
— those are large, signed, downloadable domain extensions distributed
over a network; this crate's content is bundled into the binary at
compile time, no download step.

See `README.md` and `docs/superpowers/specs/2026-09-23-aivyx-skills-design.md`
for the full design rationale.

## Build, test, lint

```sh
cargo build
cargo test
cargo clippy --all-targets
cargo fmt
```

Single crate, no workspace — no `-p` flag needed.

## Architecture

See the design spec for the full picture; in brief:

- `skills/<name>/SKILL.md` — one directory per skill, each a YAML-
  frontmatter (`name`, `description`) + markdown-body file, matching the
  same format Claude Code's own Skill system uses.
- `src/lib.rs` — the public API (`SkillLoader`, `Skill`, `SkillSummary`).
- `src/loader.rs` — parsing plus the project/user/bundled merge-and-
  override resolution logic.
- The `skills/` directory is embedded into the compiled binary via
  `rust-embed`; a `SkillLoader` configured with a project- and/or user-
  level directory additionally reads real skill files from the
  filesystem at runtime, taking priority over the bundled defaults on a
  name collision.

## Contributing a new skill

New or edited skills must stay genuinely tool-agnostic — no skill in this
library should assume a specific host agent's tool surface (see
`CONTRIBUTING.md`).
