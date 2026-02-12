use serde::Deserialize;

use crate::error::{DicecutError, Result};

#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub source: String,
    pub tags: Vec<String>,
    pub author: String,
    pub updated: Option<String>,
}

#[derive(Deserialize)]
struct GithubSearchResponse {
    items: Vec<GithubRepo>,
}

#[derive(Deserialize)]
struct GithubRepo {
    name: String,
    description: Option<String>,
    clone_url: String,
    topics: Option<Vec<String>>,
    owner: GithubOwner,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct GithubOwner {
    login: String,
}

pub fn parse_github_response(json: &str) -> Result<Vec<RegistryEntry>> {
    let response: GithubSearchResponse =
        serde_json::from_str(json).map_err(|e| DicecutError::RegistrySearchError {
            message: format!("Failed to parse GitHub response: {e}"),
        })?;

    Ok(response
        .items
        .into_iter()
        .map(|repo| RegistryEntry {
            name: repo.name,
            description: repo.description.unwrap_or_default(),
            source: repo.clone_url,
            tags: repo.topics.unwrap_or_default(),
            author: repo.owner.login,
            updated: repo.updated_at,
        })
        .collect())
}

pub fn search_github(query: &str) -> Result<Vec<RegistryEntry>> {
    let url = format!(
        "https://api.github.com/search/repositories?q={}+topic:diecut-template",
        query
    );

    let response = ureq::get(&url)
        .header("User-Agent", "diecut-cli")
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| {
            if let ureq::Error::StatusCode(403) = &e {
                return DicecutError::RateLimited;
            }
            if let ureq::Error::StatusCode(422) = &e {
                return DicecutError::RegistrySearchError {
                    message: "Invalid search query".to_string(),
                };
            }
            DicecutError::RegistrySearchError {
                message: format!("HTTP request failed: {e}"),
            }
        })?;

    let body =
        response
            .into_body()
            .read_to_string()
            .map_err(|e| DicecutError::RegistrySearchError {
                message: format!("Failed to read response body: {e}"),
            })?;

    parse_github_response(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK_GITHUB_RESPONSE: &str = r#"{
        "total_count": 2,
        "incomplete_results": false,
        "items": [
            {
                "name": "rust-cli-template",
                "full_name": "alice/rust-cli-template",
                "description": "Production-ready Rust CLI template",
                "clone_url": "https://github.com/alice/rust-cli-template.git",
                "topics": ["rust", "cli", "diecut-template"],
                "owner": { "login": "alice" },
                "updated_at": "2026-01-15T10:30:00Z"
            },
            {
                "name": "python-api",
                "full_name": "bob/python-api",
                "description": "FastAPI template with Docker",
                "clone_url": "https://github.com/bob/python-api.git",
                "topics": ["python", "api", "docker", "diecut-template"],
                "owner": { "login": "bob" },
                "updated_at": "2026-02-01T08:00:00Z"
            }
        ]
    }"#;

    const MOCK_EMPTY_RESPONSE: &str = r#"{
        "total_count": 0,
        "incomplete_results": false,
        "items": []
    }"#;

    const MOCK_MINIMAL_RESPONSE: &str = r#"{
        "total_count": 1,
        "incomplete_results": false,
        "items": [
            {
                "name": "bare-template",
                "full_name": "user/bare-template",
                "description": null,
                "clone_url": "https://github.com/user/bare-template.git",
                "topics": null,
                "owner": { "login": "user" },
                "updated_at": null
            }
        ]
    }"#;

    #[test]
    fn test_parse_github_response_with_results() {
        let entries = parse_github_response(MOCK_GITHUB_RESPONSE).unwrap();
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].name, "rust-cli-template");
        assert_eq!(entries[0].author, "alice");
        assert_eq!(entries[0].description, "Production-ready Rust CLI template");
        assert_eq!(
            entries[0].source,
            "https://github.com/alice/rust-cli-template.git"
        );
        assert_eq!(entries[0].tags, vec!["rust", "cli", "diecut-template"]);
        assert_eq!(entries[0].updated, Some("2026-01-15T10:30:00Z".to_string()));

        assert_eq!(entries[1].name, "python-api");
        assert_eq!(entries[1].author, "bob");
        assert_eq!(entries[1].tags.len(), 4);
    }

    #[test]
    fn test_parse_github_response_empty() {
        let entries = parse_github_response(MOCK_EMPTY_RESPONSE).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_github_response_minimal_fields() {
        let entries = parse_github_response(MOCK_MINIMAL_RESPONSE).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "bare-template");
        assert_eq!(entries[0].description, "");
        assert!(entries[0].tags.is_empty());
        assert!(entries[0].updated.is_none());
    }

    #[test]
    fn test_parse_github_response_invalid_json() {
        let result = parse_github_response("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_entry_fields() {
        let entry = RegistryEntry {
            name: "my-template".to_string(),
            description: "A test template".to_string(),
            source: "https://github.com/user/my-template.git".to_string(),
            tags: vec!["rust".to_string(), "cli".to_string()],
            author: "user".to_string(),
            updated: Some("2026-01-01T00:00:00Z".to_string()),
        };
        assert_eq!(entry.name, "my-template");
        assert_eq!(entry.author, "user");
        assert_eq!(entry.tags.len(), 2);
    }
}
