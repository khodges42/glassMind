use std::collections::BTreeSet;

use regex::Regex;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct MarkdownDocument {
    pub headings: Vec<String>,
    pub blocks: Vec<MarkdownBlock>,
    pub wikilinks: Vec<Wikilink>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarkdownBlock {
    pub kind: MarkdownBlockKind,
    pub text: String,
    pub start_line: usize,
    pub end_line: usize,
    pub heading_path: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownBlockKind {
    Heading,
    Paragraph,
    CodeBlock,
    List,
}

#[derive(Clone, Debug, Serialize)]
pub struct Wikilink {
    pub source: String,
    pub target: String,
    pub alias: Option<String>,
}

pub fn parse_markdown(source_path: &str, content: &str) -> MarkdownDocument {
    let _ = pulldown_cmark::Parser::new_ext(content, pulldown_cmark::Options::all()).count();

    let mut headings = Vec::new();
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut paragraph_start = 0;
    let mut in_code = false;
    let mut code = Vec::new();
    let mut code_start = 0;
    let mut heading_stack: Vec<(usize, String)> = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();

        // Code fences get kept whole so later chunks stay readable.
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if in_code {
                code.push(line.to_string());
                blocks.push(MarkdownBlock {
                    kind: MarkdownBlockKind::CodeBlock,
                    text: code.join("\n"),
                    start_line: code_start,
                    end_line: line_no,
                    heading_path: current_heading_path(&heading_stack),
                });
                code.clear();
                in_code = false;
            } else {
                flush_paragraph(
                    &mut blocks,
                    &mut paragraph,
                    paragraph_start,
                    line_no.saturating_sub(1),
                    &heading_stack,
                );
                in_code = true;
                code_start = line_no;
                code.push(line.to_string());
            }
            continue;
        }

        if in_code {
            code.push(line.to_string());
            continue;
        }

        if let Some((level, heading)) = parse_heading(trimmed) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph,
                paragraph_start,
                line_no.saturating_sub(1),
                &heading_stack,
            );
            while heading_stack
                .last()
                .is_some_and(|(last_level, _)| *last_level >= level)
            {
                heading_stack.pop();
            }
            heading_stack.push((level, heading.clone()));
            headings.push(heading.clone());
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Heading,
                text: heading,
                start_line: line_no,
                end_line: line_no,
                heading_path: current_heading_path(&heading_stack),
            });
            continue;
        }

        if is_list_item(trimmed) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph,
                paragraph_start,
                line_no.saturating_sub(1),
                &heading_stack,
            );
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::List,
                text: trimmed.to_string(),
                start_line: line_no,
                end_line: line_no,
                heading_path: current_heading_path(&heading_stack),
            });
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(
                &mut blocks,
                &mut paragraph,
                paragraph_start,
                line_no.saturating_sub(1),
                &heading_stack,
            );
            continue;
        }

        if paragraph.is_empty() {
            paragraph_start = line_no;
        }
        paragraph.push(trimmed.to_string());
    }

    let final_line = content.lines().count();
    if in_code {
        blocks.push(MarkdownBlock {
            kind: MarkdownBlockKind::CodeBlock,
            text: code.join("\n"),
            start_line: code_start,
            end_line: final_line,
            heading_path: current_heading_path(&heading_stack),
        });
    }
    flush_paragraph(
        &mut blocks,
        &mut paragraph,
        paragraph_start,
        final_line,
        &heading_stack,
    );

    MarkdownDocument {
        headings,
        blocks,
        wikilinks: extract_wikilinks(source_path, content),
        tags: extract_tags(content),
    }
}

fn flush_paragraph(
    blocks: &mut Vec<MarkdownBlock>,
    paragraph: &mut Vec<String>,
    start_line: usize,
    end_line: usize,
    heading_stack: &[(usize, String)],
) {
    if paragraph.is_empty() {
        return;
    }

    blocks.push(MarkdownBlock {
        kind: MarkdownBlockKind::Paragraph,
        text: paragraph.join(" "),
        start_line,
        end_line,
        heading_path: current_heading_path(heading_stack),
    });
    paragraph.clear();
}

fn parse_heading(trimmed: &str) -> Option<(usize, String)> {
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes) == Some(' ') {
        Some((hashes, trimmed[hashes + 1..].trim().to_string()))
    } else {
        None
    }
}

fn current_heading_path(heading_stack: &[(usize, String)]) -> Vec<String> {
    heading_stack
        .iter()
        .map(|(_, heading)| heading.clone())
        .collect()
}

fn is_list_item(trimmed: &str) -> bool {
    trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || trimmed.split_once(". ").is_some_and(|(prefix, _)| {
            !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit())
        })
}

pub fn extract_wikilinks(source_path: &str, content: &str) -> Vec<Wikilink> {
    let link_re = Regex::new(r"\[\[([^\]\|]+?)(?:\|([^\]]+))?\]\]").expect("valid wikilink regex");
    link_re
        .captures_iter(content)
        .filter_map(|capture| {
            let target = capture.get(1)?.as_str().trim().to_string();
            if target.is_empty() {
                return None;
            }
            let alias = capture
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .filter(|s| !s.is_empty());
            Some(Wikilink {
                source: source_path.to_string(),
                target,
                alias,
            })
        })
        .collect()
}

pub fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = BTreeSet::new();
    // Frontmatter and inline tags meet here, then we normalize once.
    for tag in extract_frontmatter_tags(content)
        .into_iter()
        .chain(extract_inline_tags(content))
    {
        let normalized = normalize_tag(&tag);
        if !normalized.is_empty() {
            tags.insert(normalized);
        }
    }
    tags.into_iter().collect()
}

fn extract_frontmatter_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return tags;
    }

    let mut in_tags_list = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }

        if let Some(value) = trimmed.strip_prefix("tags:") {
            in_tags_list = true;
            tags.extend(split_tag_values(value));
            continue;
        }

        if in_tags_list && trimmed.starts_with('-') {
            tags.push(trimmed.trim_start_matches('-').trim().to_string());
            continue;
        }

        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            in_tags_list = false;
        }
    }

    tags
}

fn extract_inline_tags(content: &str) -> Vec<String> {
    let tag_re = Regex::new(r"(?m)(^|[\s(\[{])#([A-Za-z0-9_/-]+)").expect("valid tag regex");
    tag_re
        .captures_iter(content)
        .filter_map(|capture| capture.get(2).map(|tag| tag.as_str().to_string()))
        .collect()
}

fn split_tag_values(value: &str) -> Vec<String> {
    let value = value.trim().trim_start_matches('[').trim_end_matches(']');
    value
        .split(',')
        .map(|tag| tag.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn normalize_tag(tag: &str) -> String {
    tag.trim().trim_start_matches('#').trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{MarkdownBlockKind, extract_tags, extract_wikilinks, parse_markdown};

    #[test]
    fn extracts_obsidian_wikilink_forms() {
        let links = extract_wikilinks(
            "source.md",
            "[[note]] [[note|alias]] [[folder/note]] [[folder/note#Heading|Alias]]",
        );

        assert_eq!(links.len(), 4);
        assert_eq!(links[0].target, "note");
        assert_eq!(links[0].alias, None);
        assert_eq!(links[1].target, "note");
        assert_eq!(links[1].alias.as_deref(), Some("alias"));
        assert_eq!(links[2].target, "folder/note");
        assert_eq!(links[3].target, "folder/note#Heading");
        assert_eq!(links[3].alias.as_deref(), Some("Alias"));
    }

    #[test]
    fn extracts_markdown_structure_from_malformed_input() {
        let document = parse_markdown(
            "note.md",
            "# Title\n\nParagraph text\n\n- item\n\n```rust\nfn main() {}\n",
        );

        assert_eq!(document.headings, vec!["Title"]);
        assert!(
            document
                .blocks
                .iter()
                .any(|block| matches!(block.kind, MarkdownBlockKind::Paragraph))
        );
        assert!(
            document
                .blocks
                .iter()
                .any(|block| matches!(block.kind, MarkdownBlockKind::List))
        );
        assert!(
            document
                .blocks
                .iter()
                .any(|block| matches!(block.kind, MarkdownBlockKind::CodeBlock))
        );
    }

    #[test]
    fn extracts_and_normalizes_tags() {
        let tags = extract_tags(
            "---\ntags: [Rust, glassmind]\n---\nBody #Rust #local-first\n# Heading is not a tag\n",
        );

        assert_eq!(tags, vec!["glassmind", "local-first", "rust"]);
    }
}
