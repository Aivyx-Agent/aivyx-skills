//! `SkillLoader`: merges the bundled default skills (embedded into the
//! binary from this repo's own `skills/` directory) with an optional
//! caller-provided project and/or user overlay directory. See
//! `docs/superpowers/specs/2026-09-23-aivyx-skills-design.md` for the
//! full design.

use rust_embed::RustEmbed;

use crate::frontmatter::parse_skill_md;

#[derive(RustEmbed)]
#[folder = "skills/"]
struct BundledSkills;

/// Who a resolved `Skill`/`SkillSummary` came from -- `Project` and
/// `User` only ever appear once `SkillLoader::with_project_dir`/
/// `with_user_dir` are used (Task 3); this task only ever produces
/// `Bundled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Bundled,
    User,
    Project,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub source: SkillSource,
}

#[derive(Debug, Clone)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub source: SkillSource,
}

#[derive(Default)]
pub struct SkillLoader {}

impl SkillLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<SkillSummary> {
        bundled_skills()
            .into_iter()
            .map(|s| SkillSummary {
                name: s.name,
                description: s.description,
                source: s.source,
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Skill> {
        bundled_skills().into_iter().find(|s| s.name == name)
    }
}

/// Parses every bundled `SKILL.md`, panicking on a parse failure or a
/// name/directory mismatch -- both are only reachable via a bug in this
/// crate's own committed content, never from runtime/user data, and
/// `all_bundled_skills_parse_and_match_their_directory_name` below is
/// specifically designed to catch either before any release.
fn bundled_skills() -> Vec<Skill> {
    let mut skills = Vec::new();
    for path in BundledSkills::iter() {
        let Some(dir_name) = path.strip_suffix("/SKILL.md") else {
            continue;
        };
        let file = BundledSkills::get(&path).unwrap_or_else(|| {
            panic!("bundled skill {path} listed by iter() but get() returned None")
        });
        let raw = std::str::from_utf8(&file.data)
            .unwrap_or_else(|e| panic!("bundled skill {path} is not valid UTF-8: {e}"));
        let (name, description, body) = parse_skill_md(raw)
            .unwrap_or_else(|e| panic!("bundled skill {path} failed to parse: {e}"));
        assert_eq!(
            name, dir_name,
            "bundled skill {path}'s frontmatter name {name:?} must match its directory name {dir_name:?}"
        );
        skills.push(Skill {
            name,
            description,
            body,
            source: SkillSource::Bundled,
        });
    }
    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The single most important test in this crate: proves every
    /// bundled skill committed to `skills/` genuinely parses under
    /// Task 1's frontmatter parser and that its `name` field matches its
    /// own directory -- if this ever fails, `bundled_skills()` would
    /// otherwise panic at runtime for every single consumer of this
    /// crate, so catching it here, in CI, is the whole point.
    #[test]
    fn all_bundled_skills_parse_and_match_their_directory_name() {
        let skills = bundled_skills();
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names.len(),
            5,
            "expected exactly 5 bundled skills, got {names:?}"
        );
        for expected in [
            "systematic-debugging",
            "brainstorming-and-scoping",
            "writing-plans",
            "self-review-before-done",
            "clear-communication",
        ] {
            assert!(
                names.contains(&expected),
                "expected bundled skill {expected:?} to be present, got {names:?}"
            );
        }
        for skill in &skills {
            assert_eq!(skill.source, SkillSource::Bundled);
            assert!(!skill.description.is_empty());
            assert!(!skill.body.is_empty());
        }
    }

    #[test]
    fn list_returns_a_summary_for_every_bundled_skill() {
        let loader = SkillLoader::new();
        let summaries = loader.list();
        assert_eq!(summaries.len(), 5);
        assert!(
            summaries
                .iter()
                .any(|s| s.name == "systematic-debugging" && s.source == SkillSource::Bundled)
        );
    }

    #[test]
    fn get_returns_the_full_skill_body_for_a_known_name() {
        let loader = SkillLoader::new();
        let skill = loader.get("systematic-debugging").unwrap();
        assert_eq!(skill.name, "systematic-debugging");
        assert_eq!(skill.source, SkillSource::Bundled);
        assert!(skill.body.contains("Reproduce"));
    }

    #[test]
    fn get_returns_none_for_an_unknown_name() {
        let loader = SkillLoader::new();
        assert!(loader.get("does-not-exist").is_none());
    }
}
