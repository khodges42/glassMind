use regex::Regex;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct MarkdownDocument {
    pub headings: Vec<String>,
    pub blocks: Vec<MarkdownBlock>,
    pub wikilinks: Vec<Wikilink>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MarkdownBlock {
    pub kind: MarkdownBlockKind,
    pub text: String,
    pub start_line: usize,
    pub end_line: usize,
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

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if in_code {
                code.push(line.to_string());
                blocks.push(MarkdownBlock {
                    kind: MarkdownBlockKind::CodeBlock,
                    text: code.join("\n"),
                    start_line: code_start,
                    end_line: line_no,
                });
                code.clear();
                in_code = false;
            } else {
                flush_paragraph(
                    &mut blocks,
                    &mut paragraph,
                    paragraph_start,
                    line_no.saturating_sub(1),
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

        if let Some(heading) = parse_heading(trimmed) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph,
                paragraph_start,
                line_no.saturating_sub(1),
            );
            headings.push(heading.clone());
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Heading,
                text: heading,
                start_line: line_no,
                end_line: line_no,
            });
            continue;
        }

        if is_list_item(trimmed) {
            flush_paragraph(
                &mut blocks,
                &mut paragraph,
                paragraph_start,
                line_no.saturating_sub(1),
            );
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::List,
                text: trimmed.to_string(),
                start_line: line_no,
                end_line: line_no,
            });
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(
                &mut blocks,
                &mut paragraph,
                paragraph_start,
                line_no.saturating_sub(1),
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
        });
    }
    flush_paragraph(&mut blocks, &mut paragraph, paragraph_start, final_line);

    MarkdownDocument {
        headings,
        blocks,
        wikilinks: extract_wikilinks(source_path, content),
    }
}

fn flush_paragraph(
    blocks: &mut Vec<MarkdownBlock>,
    paragraph: &mut Vec<String>,
    start_line: usize,
    end_line: usize,
) {
    if paragraph.is_empty() {
        return;
    }

    blocks.push(MarkdownBlock {
        kind: MarkdownBlockKind::Paragraph,
        text: paragraph.join(" "),
        start_line,
        end_line,
    });
    paragraph.clear();
}

fn parse_heading(trimmed: &str) -> Option<String> {
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes) == Some(' ') {
        Some(trimmed[hashes + 1..].trim().to_string())
    } else {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::{MarkdownBlockKind, extract_wikilinks, parse_markdown};

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
}
