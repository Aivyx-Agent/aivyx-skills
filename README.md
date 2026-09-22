# aivyx-skills

A default, system-level library of [Claude-Skills-format](https://www.anthropic.com)
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

Early — design and initial scaffold only, not yet a working crate.

## License

Dual-licensed under Apache-2.0 OR MIT — see `LICENSE`.
