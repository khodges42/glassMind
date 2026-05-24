use serde::Serialize;

use crate::db::SearchHit;

pub trait Summarizer {
    fn summarize(&self, text: &str) -> Option<String>;
}

pub struct DisabledSummarizer;

impl Summarizer for DisabledSummarizer {
    fn summarize(&self, _text: &str) -> Option<String> {
        None
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ContextBundle {
    pub query: String,
    pub token_budget: usize,
    pub used_tokens: usize,
    pub summary: Option<String>,
    pub sources: Vec<SearchHit>,
}

impl ContextBundle {
    pub fn from_hits(query: &str, token_budget: usize, hits: Vec<SearchHit>) -> Self {
        Self::from_hits_with_summarizer(query, token_budget, hits, &DisabledSummarizer)
    }

    pub fn from_hits_with_summarizer(
        query: &str,
        token_budget: usize,
        hits: Vec<SearchHit>,
        summarizer: &dyn Summarizer,
    ) -> Self {
        let mut used_tokens = 0;
        let mut sources = Vec::new();

        // Keep the highest ranked hits first, but stop before the bundle gets too chunky.
        for hit in hits {
            if used_tokens + hit.token_estimate > token_budget && !sources.is_empty() {
                break;
            }
            used_tokens += hit.token_estimate;
            sources.push(hit);
        }

        Self {
            query: query.to_string(),
            token_budget,
            used_tokens,
            summary: summarizer.summarize(query),
            sources,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut out = format!(
            "# Glassmind Context\n\nQuery: `{}`\n\nBudget: {} tokens\nUsed: {} tokens\n\n",
            self.query, self.token_budget, self.used_tokens
        );

        if self.sources.is_empty() {
            out.push_str("No matching chunks found.\n");
            return out;
        }

        out.push_str("## Suggested Context\n\n");
        for (idx, source) in self.sources.iter().enumerate() {
            out.push_str(&format!("{}. `{}`", idx + 1, source.path));
            if !source.heading_path.is_empty() {
                out.push_str(&format!(" > {}", source.heading_path));
            }
            out.push_str(&format!(
                "\n   score: {:.4}, tokens: {}\n   {}\n\n",
                source.score, source.token_estimate, source.snippet
            ));
        }

        out.push_str("## Sources\n\n");
        for source in &self.sources {
            out.push_str(&format!("- `{}`\n", source.path));
        }
        out
    }
}
