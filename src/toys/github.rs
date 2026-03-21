use crate::plugin::internal::GrantedCapabilities;

#[derive(Debug, Clone, Default)]
pub struct ListParams {
    pub since: Option<String>,
    pub state: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub items: String,
    pub has_next: bool,
    pub next_page: Option<u32>,
    pub rate_remaining: u32,
}

pub fn ensure_granted(github_granted: bool, plugin_name: &str) -> Result<(), String> {
    if github_granted {
        Ok(())
    } else {
        Err(format!(
            "github toy not granted for plugin '{}'",
            plugin_name
        ))
    }
}

pub fn list_issues(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    params: &ListParams,
) -> Result<Page, String> {
    list(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        "issues",
        params,
    )
}

pub fn list_pulls(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    params: &ListParams,
) -> Result<Page, String> {
    list(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        "pulls",
        params,
    )
}

pub fn list_issue_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    issue_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("issues/{}/comments", issue_number),
        &ListParams::default(),
    )
}

pub fn list_issue_events(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    issue_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("issues/{}/events", issue_number),
        &ListParams::default(),
    )
}

pub fn list_pull_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("pulls/{}/comments", pull_number),
        &ListParams::default(),
    )
}

pub fn list_reviews(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("pulls/{}/reviews", pull_number),
        &ListParams::default(),
    )
}

pub fn list_review_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
    review_id: u64,
) -> Result<Page, String> {
    list_raw_path(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        &format!("pulls/{}/reviews/{}/comments", pull_number, review_id),
        &ListParams::default(),
    )
}

fn list(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    path: &str,
    params: &ListParams,
) -> Result<Page, String> {
    list_raw_path(http_client, grants, plugin_name, owner, repo, path, params)
}

fn list_raw_path(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    path: &str,
    params: &ListParams,
) -> Result<Page, String> {
    if !grants.http_domains.contains("api.github.com") {
        return Err(format!(
            "domain 'api.github.com' not granted for plugin '{}'",
            plugin_name
        ));
    }

    let mut url = reqwest::Url::parse(&format!(
        "https://api.github.com/repos/{owner}/{repo}/{path}"
    ))
    .map_err(|e| e.to_string())?;
    {
        let mut q = url.query_pairs_mut();
        if let Some(since) = params.since.as_deref() {
            if !since.trim().is_empty() {
                q.append_pair("since", since);
            }
        }
        if let Some(state) = params.state.as_deref() {
            if !state.trim().is_empty() {
                q.append_pair("state", state);
            }
        }
        if let Some(page) = params.page {
            q.append_pair("page", &page.to_string());
        }
        if let Some(per_page) = params.per_page {
            q.append_pair("per_page", &per_page.to_string());
        }
    }

    let mut request = http_client
        .get(url)
        .header("User-Agent", "patina-toy-github")
        .header("Accept", "application/vnd.github+json");

    if let Some(token) = token_from_grants(grants) {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    if status >= 400 {
        return Err(format!("github api request failed with status {}", status));
    }

    let rate_remaining = response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let (has_next, next_page) = parse_next_page(response.headers().get("link"));

    let body = response.text().map_err(|e| e.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("github response was not valid json: {}", e))?;
    if !parsed.is_array() {
        return Err("github list endpoint expected array response".into());
    }

    Ok(Page {
        items: body,
        has_next,
        next_page,
        rate_remaining,
    })
}

fn token_from_grants(grants: &GrantedCapabilities) -> Option<String> {
    if let Some(mapping) = grants.credential_mappings.get("api.github.com") {
        if let Ok(Some(secret)) = crate::secrets::get_global_secret(&mapping.secret_name) {
            if !secret.trim().is_empty() {
                return Some(secret);
            }
        }
    }
    std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

fn parse_next_page(link_header: Option<&reqwest::header::HeaderValue>) -> (bool, Option<u32>) {
    let Some(raw) = link_header.and_then(|v| v.to_str().ok()) else {
        return (false, None);
    };

    for part in raw.split(',') {
        let part = part.trim();
        if !part.contains("rel=\"next\"") {
            continue;
        }
        let start = part.find('<');
        let end = part.find('>');
        let Some(start) = start else {
            return (true, None);
        };
        let Some(end) = end else {
            return (true, None);
        };
        if end <= start + 1 {
            return (true, None);
        }
        let url = &part[start + 1..end];
        let next_page = reqwest::Url::parse(url).ok().and_then(|u| {
            u.query_pairs()
                .find(|(k, _)| k == "page")
                .and_then(|(_, v)| v.parse::<u32>().ok())
        });
        return (true, next_page);
    }

    (false, None)
}
