use anyhow::{Result, bail};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::config::SearchEngine;

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub query: String,
    pub snippets: Vec<String>,
}

impl SearchResults {
    pub fn as_prompt_context(&self) -> Option<String> {
        if self.snippets.is_empty() {
            return None;
        }

        let mut lines = vec![format!("Search query: {}", self.query)];
        for snippet in &self.snippets {
            lines.push(format!("- {snippet}"));
        }
        Some(lines.join("\n"))
    }
}

pub async fn search(client: &Client, engine: SearchEngine, query: &str) -> Result<SearchResults> {
    match engine {
        SearchEngine::Ddg => search_duckduckgo(client, query).await,
        SearchEngine::Serper => bail!("serper is not implemented yet"),
    }
}

async fn search_duckduckgo(client: &Client, query: &str) -> Result<SearchResults> {
    let json = client
        .get("https://api.duckduckgo.com/")
        .query(&[
            ("q", query),
            ("format", "json"),
            ("no_html", "1"),
            ("skip_disambig", "1"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<DuckDuckGoResponse>()
        .await?;

    let mut snippets = collect_json_snippets(&json);
    if snippets.len() < 3 {
        snippets.extend(search_duckduckgo_html(client, query).await?);
    }

    dedupe(&mut snippets);
    snippets.truncate(3);

    Ok(SearchResults {
        query: query.to_string(),
        snippets,
    })
}

async fn search_duckduckgo_html(client: &Client, query: &str) -> Result<Vec<String>> {
    let body = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let document = Html::parse_document(&body);
    let selector = Selector::parse(".result__snippet").expect("valid selector");
    let mut snippets = Vec::new();

    for node in document.select(&selector).take(3) {
        let text = node.text().collect::<Vec<_>>().join(" ");
        let text = clean_whitespace(&text);
        if !text.is_empty() {
            snippets.push(text);
        }
    }

    Ok(snippets)
}

fn collect_json_snippets(response: &DuckDuckGoResponse) -> Vec<String> {
    let mut snippets = Vec::new();

    if let Some(text) = response
        .abstract_text
        .as_ref()
        .map(|text| clean_whitespace(text))
        .filter(|text| !text.is_empty())
    {
        snippets.push(text);
    }

    if let Some(text) = response
        .answer
        .as_ref()
        .map(|text| clean_whitespace(text))
        .filter(|text| !text.is_empty())
    {
        snippets.push(text);
    }

    for topic in &response.related_topics {
        flatten_related_topics(topic, &mut snippets);
        if snippets.len() >= 3 {
            break;
        }
    }

    snippets
}

fn flatten_related_topics(topic: &DuckTopic, snippets: &mut Vec<String>) {
    match topic {
        DuckTopic::Topic { text } => {
            let cleaned = clean_whitespace(text);
            if !cleaned.is_empty() {
                snippets.push(cleaned);
            }
        }
        DuckTopic::Group { topics } => {
            for nested in topics {
                flatten_related_topics(nested, snippets);
            }
        }
    }
}

fn clean_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn dedupe(values: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    values.retain(|value| seen.insert(value.to_lowercase()));
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoResponse {
    #[serde(rename = "AbstractText")]
    abstract_text: Option<String>,

    #[serde(rename = "Answer")]
    answer: Option<String>,

    #[serde(rename = "RelatedTopics", default)]
    related_topics: Vec<DuckTopic>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DuckTopic {
    Topic {
        #[serde(rename = "Text")]
        text: String,
    },
    Group {
        #[serde(rename = "Topics")]
        topics: Vec<DuckTopic>,
    },
}
