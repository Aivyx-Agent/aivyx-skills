//! Parsing for the SKILL.md frontmatter format: a `---`-delimited header
//! with exactly two required scalar fields (`name`, `description`),
//! followed by a markdown body. Deliberately NOT a full YAML parser --
//! the schema is two flat string fields, so a minimal, dependency-free
//! line-based parser is used instead of pulling in a general YAML crate,
//! matching this workspace's established preference for keeping small
//! shared crates dependency-light (see `aivyx-injection-guard`'s own
//! "no dependencies -- pure std").
//!
//! Supported scalar styles for `name`/`description` (see
//! `README.md`'s "SKILL.md frontmatter format" section for the
//! user-facing version of this list):
//! - Plain, unquoted (`description: A description.`).
//! - Double-quoted (`description: "A description."`), with `\"` and
//!   `\\` unescaped.
//! - Single-quoted (`description: 'A description.'`), quotes stripped,
//!   no escape processing.
//! - Folded block scalar (`description: >`, continuation lines indented
//!   on following lines), joined into one line with spaces.
//! - Literal block scalar (`description: |`, continuation lines
//!   indented on following lines), joined with `\n`, preserving line
//!   breaks.
//!
//! The closing `---` delimiter may or may not be followed by a trailing
//! newline -- a file that ends exactly at `---` with no final `\n` is
//! accepted (with an empty body), not rejected.

use std::fmt;

// The shared `Missing` prefix is intentional, not an oversight: all three
// variants are the same underlying condition (something required wasn't
// present), and that shared vocabulary is clearer here than inventing
// three unrelated-sounding names just to satisfy the lint.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FrontmatterError {
    MissingOpeningDelimiter,
    MissingClosingDelimiter,
    MissingField(&'static str),
}

impl fmt::Display for FrontmatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontmatterError::MissingOpeningDelimiter => {
                write!(f, "file does not start with a `---` frontmatter delimiter")
            }
            FrontmatterError::MissingClosingDelimiter => {
                write!(f, "frontmatter has no closing `---` delimiter")
            }
            FrontmatterError::MissingField(field) => {
                write!(f, "frontmatter is missing required field `{field}`")
            }
        }
    }
}

impl std::error::Error for FrontmatterError {}

/// Parses a full `SKILL.md` file's contents into `(name, description,
/// body)`. `body` is the markdown content after the closing `---`, with
/// leading blank lines trimmed. Any `\r\n` line endings (frontmatter or
/// body) are normalized to `\n` first, so `body`'s line endings may
/// differ from the raw input even though its text content is otherwise
/// unchanged.
pub(crate) fn parse_skill_md(raw: &str) -> Result<(String, String, String), FrontmatterError> {
    // Normalize CRLF to LF up front so every delimiter/line check below --
    // in the frontmatter block AND the body -- works identically whether
    // the file was authored/checked-out with `\n` or `\r\n` line endings.
    let raw = raw.replace("\r\n", "\n");

    let after_open = raw
        .strip_prefix("---\n")
        .ok_or(FrontmatterError::MissingOpeningDelimiter)?;

    // The closing `---` is normally followed by a newline (and then the
    // body); but a file that ends exactly at the closing `---`, with no
    // final `\n` at all, is also valid -- it just has an empty body.
    let (frontmatter_block, rest) = if let Some(pos) = after_open.find("\n---\n") {
        (&after_open[..pos], &after_open[pos + "\n---\n".len()..])
    } else if let Some(stripped) = after_open.strip_suffix("\n---") {
        (stripped, "")
    } else {
        return Err(FrontmatterError::MissingClosingDelimiter);
    };

    let mut name = None;
    let mut description = None;
    let mut lines = frontmatter_block.lines().peekable();
    while let Some(raw_line) = lines.next() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, raw_value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let raw_value = raw_value.trim();

        // `>` (folded) and `|` (literal) introduce a YAML block scalar:
        // the actual content is on the following, more-indented lines,
        // not after the colon on this line. Any key's block scalar gets
        // consumed here (not just name/description's) so its
        // continuation lines are never mistaken for separate `key:
        // value` lines on the next loop iteration.
        let value = if raw_value == ">" || raw_value == "|" {
            let mut continuation_lines = Vec::new();
            while let Some(next_line) = lines.peek() {
                let is_indented = next_line.trim_start().len() != next_line.len();
                if !is_indented || next_line.trim().is_empty() {
                    break;
                }
                continuation_lines.push(next_line.trim().to_string());
                lines.next();
            }
            let separator = if raw_value == ">" { " " } else { "\n" };
            continuation_lines.join(separator)
        } else {
            unquote(raw_value)
        };

        match key {
            "name" => name = Some(value),
            "description" => description = Some(value),
            _ => {}
        }
    }

    let name = name.ok_or(FrontmatterError::MissingField("name"))?;
    let description = description.ok_or(FrontmatterError::MissingField("description"))?;
    let body = rest.trim_start_matches('\n').to_string();

    Ok((name, description, body))
}

/// Strips a single- or double-quoted YAML scalar's surrounding quotes.
/// A double-quoted value also has `\"` and `\\` unescaped -- the only
/// two escapes this deliberately-minimal parser supports (see the
/// module doc comment). A value that isn't quoted at all, or is quoted
/// on only one side, is returned unchanged, quote characters and all.
fn unquote(value: &str) -> String {
    if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    // Not a recognized escape -- keep the backslash
                    // literally rather than silently dropping it.
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        }
        out
    } else if let Some(inner) = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')) {
        inner.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_well_formed_skill_md() {
        let raw = "---\nname: systematic-debugging\ndescription: A structured methodology.\n---\n\n# Systematic Debugging\n\nBody text here.\n";
        let (name, description, body) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "systematic-debugging");
        assert_eq!(description, "A structured methodology.");
        assert_eq!(body, "# Systematic Debugging\n\nBody text here.\n");
    }

    #[test]
    fn only_splits_on_the_first_colon_in_a_value() {
        let raw = "---\nname: clear-communication\ndescription: Use when: writing anything user-facing.\n---\n\nBody.\n";
        let (_, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(description, "Use when: writing anything user-facing.");
    }

    #[test]
    fn tolerates_unknown_frontmatter_fields() {
        let raw =
            "---\nname: a-skill\ndescription: A description.\nauthor: someone\n---\n\nBody.\n";
        let (name, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "a-skill");
        assert_eq!(description, "A description.");
    }

    #[test]
    fn tolerates_crlf_line_endings() {
        let raw = "---\r\nname: systematic-debugging\r\ndescription: A structured methodology.\r\n---\r\n\r\n# Systematic Debugging\r\n\r\nBody text here.\r\n";
        let (name, description, body) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "systematic-debugging");
        assert_eq!(description, "A structured methodology.");
        assert_eq!(body, "# Systematic Debugging\n\nBody text here.\n");
    }

    #[test]
    fn rejects_a_file_with_no_opening_delimiter() {
        let raw = "name: a-skill\ndescription: A description.\n---\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingOpeningDelimiter
        );
    }

    #[test]
    fn rejects_a_file_with_no_closing_delimiter() {
        let raw = "---\nname: a-skill\ndescription: A description.\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingClosingDelimiter
        );
    }

    #[test]
    fn rejects_frontmatter_missing_the_name_field() {
        let raw = "---\ndescription: A description.\n---\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingField("name")
        );
    }

    #[test]
    fn rejects_frontmatter_missing_the_description_field() {
        let raw = "---\nname: a-skill\n---\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingField("description")
        );
    }

    #[test]
    fn double_quoted_scalars_are_unquoted_and_unescaped() {
        let raw = r#"---
name: a-skill
description: "A \"quoted\" value with a backslash: \\."
---

Body.
"#;
        let (_, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(description, "A \"quoted\" value with a backslash: \\.");
    }

    #[test]
    fn single_quoted_scalars_are_unquoted() {
        let raw = r#"---
name: a-skill
description: 'A single-quoted value.'
---

Body.
"#;
        let (_, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(description, "A single-quoted value.");
    }

    #[test]
    fn folded_block_scalar_description_joins_lines_with_spaces() {
        let raw = "---\nname: a-skill\ndescription: >\n  This is a folded\n  description split\n  across lines.\n---\n\nBody.\n";
        let (_, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(
            description,
            "This is a folded description split across lines."
        );
    }

    #[test]
    fn literal_block_scalar_description_preserves_newlines() {
        let raw = "---\nname: a-skill\ndescription: |\n  Line one.\n  Line two.\n---\n\nBody.\n";
        let (_, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(description, "Line one.\nLine two.");
    }

    #[test]
    fn a_closing_delimiter_with_no_trailing_newline_is_accepted() {
        let raw = "---\nname: a-skill\ndescription: A description.\n---";
        let (name, description, body) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "a-skill");
        assert_eq!(description, "A description.");
        assert_eq!(body, "");
    }
}
