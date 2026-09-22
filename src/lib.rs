//! `aivyx-skills`: a default, system-level library of `SKILL.md`-format
//! capability packages, shared by both `aivyx-pa` and `aivyx-coder`
//! instead of each maintaining its own copy. Five real skills are
//! embedded into the binary at compile time; `SkillLoader` additionally
//! supports project and user overlay directories, with project > user >
//! bundled priority when a name collides. See
//! `docs/superpowers/specs/2026-09-23-aivyx-skills-design.md` for the
//! full design rationale.

mod frontmatter;
mod loader;

pub use loader::{Skill, SkillLoader, SkillSource, SkillSummary};
