use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::{config::SearchEngine, llm};

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
        SearchEngine::Tavily => match llm::env_var("TAVILY_API_KEY") {
            Some(k) => search_tavily(client, &k, query).await,
            None => search_duckduckgo(client, query).await,
        },
        SearchEngine::Ddg => search_duckduckgo(client, query).await,
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

// ── Tavily ──────────────────────────────────────────────────────────────────

async fn search_tavily(client: &Client, api_key: &str, query: &str) -> Result<SearchResults> {
    let resp = client
        .post("https://api.tavily.com/search")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "query": query,
            "max_results": 5,
            "search_depth": "basic"
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<TavilyResponse>()
        .await?;

    let mut snippets: Vec<String> = resp
        .results
        .into_iter()
        .filter_map(|r| {
            let text = clean_whitespace(&r.content);
            if text.is_empty() {
                return None;
            }
            let snippet = if let Some(url) = r.url.filter(|u| !u.is_empty()) {
                format!("{text} ({url})")
            } else {
                text
            };
            Some(snippet)
        })
        .collect();

    dedupe(&mut snippets);
    snippets.truncate(5);

    Ok(SearchResults {
        query: query.to_string(),
        snippets,
    })
}

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    content: String,
    url: Option<String>,
}
