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

## Known limitations

- Overlay directories read arbitrary local filesystem content. This
  crate does not treat that content as untrusted — whichever repo
  eventually builds the Part 2/3 consumption code should decide whether
  overlay skill content needs sanitizing before it's injected into an
  LLM context.
- `SkillLoader` re-scans and re-parses everything (bundled + overlays) on
  every `list()`/`get()` call; there's no caching. Fine at today's scale
  (5 bundled skills plus a small overlay), but worth revisiting if this
  crate is ever used on a hot path.

## License

Source-available under BUSL-1.1 — see `LICENSE`.
