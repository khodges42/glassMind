use serde::Serialize;

use crate::db::sha256_hex;
use crate::markdown::{MarkdownBlock, MarkdownBlockKind};

#[derive(Clone, Debug, Serialize)]
pub struct NoteChunk {
    pub index: usize,
    pub heading_path: Vec<String>,
    pub content: String,
    pub chunk_type: ChunkType,
    pub start_line: usize,
    pub end_line: usize,
    pub token_estimate: usize,
    pub content_hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkType {
    HeadingSection,
    SplitSection,
}

pub fn build_chunks(
    blocks: &[MarkdownBlock],
    target_tokens: usize,
    overlap_tokens: usize,
) -> Vec<NoteChunk> {
    let mut chunks = Vec::new();
    let mut current: Vec<MarkdownBlock> = Vec::new();

    for block in blocks {
        if matches!(block.kind, MarkdownBlockKind::Heading) && !current.is_empty() {
            push_section_chunks(&mut chunks, &current, target_tokens, overlap_tokens);
            current.clear();
        }
        current.push(block.clone());
    }

    if !current.is_empty() {
        push_section_chunks(&mut chunks, &current, target_tokens, overlap_tokens);
    }

    for (index, chunk) in chunks.iter_mut().enumerate() {
        chunk.index = index;
    }

    chunks
}

fn push_section_chunks(
    chunks: &mut Vec<NoteChunk>,
    section: &[MarkdownBlock],
    target_tokens: usize,
    overlap_tokens: usize,
) {
    let text = section_text(section);
    if text.trim().is_empty() {
        return;
    }

    let token_estimate = estimate_tokens(&text);
    let heading_path = section
        .iter()
        .rev()
        .find(|block| !block.heading_path.is_empty())
        .map(|block| block.heading_path.clone())
        .unwrap_or_default();
    let start_line = section.first().map_or(1, |block| block.start_line);
    let end_line = section.last().map_or(start_line, |block| block.end_line);

    if token_estimate <= target_tokens {
        chunks.push(NoteChunk {
            index: 0,
            heading_path,
            content_hash: sha256_hex(&text),
            content: text,
            chunk_type: ChunkType::HeadingSection,
            start_line,
            end_line,
            token_estimate,
        });
        return;
    }

    // Big sections get split by rough words first. Good enough for now, easy to inspect later.
    for part in split_with_overlap(&text, target_tokens, overlap_tokens) {
        let token_estimate = estimate_tokens(&part);
        chunks.push(NoteChunk {
            index: 0,
            heading_path: heading_path.clone(),
            content_hash: sha256_hex(&part),
            content: part,
            chunk_type: ChunkType::SplitSection,
            start_line,
            end_line,
            token_estimate,
        });
    }
}

fn section_text(section: &[MarkdownBlock]) -> String {
    section
        .iter()
        .map(|block| block.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn estimate_tokens(content: &str) -> usize {
    let words = content.split_whitespace().count();
    words.max(1)
}

fn split_with_overlap(content: &str, target_tokens: usize, overlap_tokens: usize) -> Vec<String> {
    let words: Vec<_> = content.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }

    let mut parts = Vec::new();
    let mut start = 0;
    let step = target_tokens.saturating_sub(overlap_tokens).max(1);

    while start < words.len() {
        let end = (start + target_tokens).min(words.len());
        let part = words[start..end].join(" ");
        parts.push(trim_to_sentenceish_boundary(part));
        if end == words.len() {
            break;
        }
        start += step;
    }

    parts
}

fn trim_to_sentenceish_boundary(part: String) -> String {
    if part.ends_with('.') || part.ends_with('!') || part.ends_with('?') || part.len() < 240 {
        return part;
    }

    match part.rfind(['.', '!', '?']) {
        Some(idx) if idx > part.len() / 2 => part[..=idx].to_string(),
        _ => part,
    }
}

pub fn chunk_type_name(kind: &ChunkType) -> &'static str {
    match kind {
        ChunkType::HeadingSection => "heading_section",
        ChunkType::SplitSection => "split_section",
    }
}

#[cfg(test)]
mod tests {
    use crate::markdown::{MarkdownBlock, MarkdownBlockKind};

    use super::{ChunkType, build_chunks};

    #[test]
    fn builds_heading_chunks_in_order() {
        let blocks = vec![
            block(MarkdownBlockKind::Heading, "A", 1, vec!["A"]),
            block(MarkdownBlockKind::Paragraph, "one", 2, vec!["A"]),
            block(MarkdownBlockKind::Heading, "B", 3, vec!["B"]),
            block(MarkdownBlockKind::Paragraph, "two", 4, vec!["B"]),
        ];

        let chunks = build_chunks(&blocks, 100, 10);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].heading_path, vec!["A"]);
        assert_eq!(chunks[1].heading_path, vec!["B"]);
        assert!(matches!(chunks[0].chunk_type, ChunkType::HeadingSection));
    }

    #[test]
    fn splits_large_sections_with_overlap() {
        let text = (0..30)
            .map(|idx| format!("word{idx}"))
            .collect::<Vec<_>>()
            .join(" ");
        let blocks = vec![block(MarkdownBlockKind::Paragraph, &text, 1, vec![])];

        let chunks = build_chunks(&blocks, 10, 2);

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.token_estimate <= 10));
        assert!(matches!(chunks[1].chunk_type, ChunkType::SplitSection));
    }

    fn block(
        kind: MarkdownBlockKind,
        text: &str,
        line: usize,
        heading_path: Vec<&str>,
    ) -> MarkdownBlock {
        MarkdownBlock {
            kind,
            text: text.to_string(),
            start_line: line,
            end_line: line,
            heading_path: heading_path.into_iter().map(String::from).collect(),
        }
    }
}
