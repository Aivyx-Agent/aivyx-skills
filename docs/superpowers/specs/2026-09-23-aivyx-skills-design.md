# Aivyx-Skills Design

## Context

A new, small shared crate, `aivyx-skills`: a default, system-level
library of `SKILL.md`-format capability packages that both `aivyx-pa`
(the flagship Personal Assistant) and `aivyx-coder` (the standalone
terminal coding agent) can depend on and load a bundled default skill set
from, extending each product's own agent with the same baseline
process/methodology guidance rather than each authoring and maintaining
its own copy. First of a three-part initiative (this crate, then each
product's own consumption work as separate, later spec→plan→
implementation cycles):

1. **This spec** — the shared crate and its default skill content.
2. `aivyx-coder`'s own consumption mechanism (a new `Skill` tool).
3. `aivyx-pa`'s own consumption mechanism (integrating without colliding
   with its existing agent-learned `Skill` system).

## Grounding

Researched directly across this workspace before any design, not
assumed:

- **Three existing, unrelated "skill"-shaped concepts already live in
  this workspace, and this crate must be clearly distinct from all
  three**, confirmed by direct reading:
  1. `aivyx-pa`'s own agent-learned `Skill`/`LearnedSkill` system
     (`docs/PRAXIS.md`, `docs/WHETSTONE.md`, `docs/REPERTOIRE.md` in that
     repo) — a mature, deeply-integrated system where the agent itself
     authors or is taught named procedures (`skills.teach`,
     `AuthorSkill`/`SkillAuthorOp::Teach`), tracked with an effectiveness
     ledger (`SkillEffectivenessLedger`) and invocation auditing
     (`AuditEvent::SkillInvocation`), governed through persona
     proposal/approval. This is learned/authored *behavior*, not a file
     format — `aivyx-skills` must never be confused with or attempt to
     replace it.
  2. `aivyx-hub`'s "vertical packs" (`.aivyxpack` bundles, e.g. the
     "Kitchen" pack) — large, signed, network-downloaded domain
     extensions for `aivyx-pa` specifically, distributed via a hub site.
     Unrelated to `aivyx-coder` and to this crate's compiled-in,
     always-available model.
  3. Claude Code's own Skill system (`SKILL.md`, name+description
     discovery, invoked by name) — the actual mechanism this design is
     modeled on, confirmed via this session's own skill listings (e.g.
     the `using-superpowers` skill's real frontmatter, read directly:
     `---\nname: using-superpowers\ndescription: ...\n---`).
- **Established precedent for this exact "small shared Rust crate,
  consumed by both `aivyx-pa` and `aivyx-coder`" shape already exists
  four times over**: `aivyx-confine` (sandboxing), `aivyx-checkpoint`
  (git-ref checkpoints), `aivyx-kvcache` (KV-cache persistence),
  `aivyx-injection-guard` (prompt-injection tripwire) — each a focused,
  independently-versioned crate pinned by git rev in both consuming
  repos' `Cargo.toml`, extracted specifically to avoid each product
  maintaining its own copy of the same primitive. `aivyx-injection-guard`
  in particular (read directly: its own `README.md`/`CLAUDE.md`) is the
  closest structural analog — small, dependency-light, one clear
  responsibility, no workspace, `cargo build`/`test`/`clippy`/`fmt` at
  the crate root with no `-p` flag needed.
- **`aivyx-pa` is already an MCP *client*** (`[[mcp_server]]` config
  entries, `docs/MCP_RECIPES.md`, capability-scoped tool registration via
  `mcp.call:<server>:<tool>`) — a real, existing extension point that a
  later consumption-phase spec could potentially reuse (package this
  crate's content behind a tiny local MCP server) instead of writing
  bespoke integration code. Not decided here — flagged as a live option
  for Part 3.
- **`aivyx-coder` is an MCP *server* only** (`crates/aivyx-mcp-server`,
  exposes itself to other clients like Zed) — it has no MCP client
  capability at all, and its `ToolRegistry` is fixed at compile time
  (confirmed by reading `crates/aivyx/Cargo.toml`'s dependency list).
  Whatever Part 2 designs, it cannot lean on an MCP-client extension
  point the way Part 3 potentially could.
- **This session's own skill frontmatter format**, confirmed directly
  from repeated system-reminder blocks throughout this session: YAML
  frontmatter with (at minimum) `name` and `description` fields, then a
  markdown body — e.g. `using-superpowers`'s real, unaltered frontmatter.
  This spec's `SKILL.md` format matches it exactly, not a reinterpretation.

## Decisions

**1. `aivyx-skills` is a small Rust crate**, not a content-only repo —
matching the `aivyx-confine`/`aivyx-checkpoint`/`aivyx-kvcache`/
`aivyx-injection-guard` precedent exactly: single crate, no workspace,
consumed as a pinned git dependency by both `aivyx-pa` and `aivyx-coder`
in their later, separate consumption phases.

**2. Repo layout**:
```
aivyx-skills/
  Cargo.toml
  README.md, CLAUDE.md, CONTRIBUTING.md, LICENSE(-APACHE/-MIT)
  skills/
    systematic-debugging/SKILL.md
    brainstorming-and-scoping/SKILL.md
    writing-plans/SKILL.md
    self-review-before-done/SKILL.md
    clear-communication/SKILL.md
  src/
    lib.rs      -- public API
    loader.rs   -- parsing + merge/override resolution
```

**3. `SKILL.md` format** — YAML frontmatter, exactly two required fields,
then a markdown body:
```markdown
---
name: systematic-debugging
description: Use when investigating a bug or unexpected behavior -- a structured reproduce/isolate/hypothesize/verify methodology.
---

# Systematic Debugging

...markdown body, the skill's actual guidance...
```
`name` must match the containing directory name (a load-time invariant,
checked and rejected on mismatch — fail closed on a malformed skill
rather than silently loading it under the wrong key). `description`
is what a consumer surfaces in a name+description listing for discovery,
mirroring exactly how this session's own skill listings work.

**4. Public API** (`src/lib.rs`):
```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub source: SkillSource,
}

pub enum SkillSource { Bundled, User, Project }

pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub source: SkillSource,
}

pub struct SkillLoader { /* project_dir: Option<PathBuf>, user_dir: Option<PathBuf> */ }

impl SkillLoader {
    pub fn new() -> Self;
    pub fn with_project_dir(self, dir: PathBuf) -> Self;
    pub fn with_user_dir(self, dir: PathBuf) -> Self;
    pub fn list(&self) -> Vec<SkillSummary>;
    pub fn get(&self, name: &str) -> Option<Skill>;
}
```
`list()` is cheap (name + description only, no body parsing beyond
frontmatter) — the shape a consumer injects into its own system
prompt/tool-list for skill discovery. `get(name)` returns the full
resolved skill (frontmatter + body) for the given name.

**5. Resolution order: project > user > bundled**, on exact name
collision — a project- or user-level skill with the same `name` as a
bundled default entirely replaces it (not merged/appended). This lets a
consumer's own user or project override a default skill without forking
this crate, matching how Claude Code itself layers project-specific
skills alongside plugin-provided ones. Neither `project_dir` nor
`user_dir` need exist on disk — a missing directory is silently treated
as "no overrides from this source," not an error (consistent with this
being an optional, additive layer).

**6. Bundled content is embedded via the `rust-embed` crate** — a
well-established, widely-audited tool for exactly this (embed a
directory's contents into the compiled binary via a derive macro,
`RustEmbed::get(path)`/`RustEmbed::iter()`), rather than hand-rolling
`include_str!` bookkeeping that has to be kept in sync with the `skills/`
directory's actual contents by hand. `project_dir`/`user_dir` skills are
read from the real filesystem at `SkillLoader` construction/call time —
never embedded, since they're inherently runtime-provided.

**7. Initial default skill set — five skills**, each written to be
genuinely tool-agnostic (no assumption of a specific host agent's tool
surface, e.g. never assuming a `Task`/`Agent`-dispatch tool,
`EnterWorktree`, or any other Claude-Code-harness-specific mechanism,
since neither `aivyx-pa` nor `aivyx-coder` has that same tool surface):
- `systematic-debugging` — reproduce → isolate → hypothesize → verify.
- `brainstorming-and-scoping` — turn a vague request into a validated,
  scoped plan before acting, with explicit checkpoints.
- `writing-plans` — break an approved design into a concrete, verifiable,
  step-by-step implementation plan.
- `self-review-before-done` — critically check completed work against
  the actual stated requirements before declaring it finished.
- `clear-communication` — write clearly and concisely for a task's
  user-facing output.

Each is real, complete, usable guidance in this phase — not a stub or
placeholder — but each is also deliberately general enough to require no
product-specific tailoring; product-specific skills (if ever wanted) are
out of scope for this initial set.

## What this spec does not decide

- Either product's own consumption mechanism (Parts 2 and 3) — including
  whether `aivyx-pa` packages this crate behind its existing MCP-client
  plumbing versus a native tool, and whether `aivyx-coder` gains a new
  native `Skill` tool. Separate, later specs.
- Any change to `aivyx-pa`'s own `Skill`/`LearnedSkill`/Praxis system —
  fully untouched, and this crate is explicitly not a replacement or
  variant of it.
- Any change to `aivyx-hub` or vertical-pack distribution.
- Publishing this repo to GitHub, CI setup, or a real release/versioning
  process — local-only for now, same starting posture as `aivyx-hub` was
  when it was first created.
- Whether more skills get added to the default set beyond the initial
  five — a natural, separate future content pass, not blocked on
  anything in this spec.
- Skill *composition* (one skill referencing/depending on another) —
  the initial five are independent; nothing in the loader API assumes or
  prevents this being added later, but it's not designed for now (YAGNI).
