use anyhow::{Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::state::ChildRegistrySourceRecord;

use super::{ChildRegistryProvider, DiscoveredChildRelease};

#[derive(Debug, Clone)]
pub struct GitHubChildRegistryProvider {
    client: Client,
}

impl GitHubChildRegistryProvider {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .context("building GitHub registry client")?;
        Ok(Self { client })
    }
}

impl Default for GitHubChildRegistryProvider {
    fn default() -> Self {
        Self::new().expect("failed to build GitHub registry client")
    }
}

impl ChildRegistryProvider for GitHubChildRegistryProvider {
    fn kind(&self) -> &'static str {
        "github"
    }

    fn sync(&self, source: &ChildRegistrySourceRecord) -> Result<Vec<DiscoveredChildRelease>> {
        let config: GitHubSourceConfig = serde_json::from_str(&source.provider_config_json)
            .with_context(|| {
                format!(
                    "parsing provider_config_json for source '{}' as GitHub config",
                    source.source_id
                )
            })?;

        let releases = self.fetch_releases(&config)?;
        let mut discovered = Vec::new();

        for release in releases {
            if release.draft {
                continue;
            }
            if release.prerelease && !config.include_prerelease.unwrap_or(false) {
                continue;
            }

            let Some(version) =
                release_version_from_tag(&release.tag_name, config.tag_prefix.as_deref())
            else {
                continue;
            };

            let wasm_asset =
                match select_asset(&release.assets, config.asset_name_wasm.as_deref(), |name| {
                    name.ends_with(".wasm")
                }) {
                    Some(asset) => asset,
                    None => continue,
                };

            let manifest_asset = match select_asset(
                &release.assets,
                config.asset_name_manifest.as_deref(),
                |name| {
                    name == "child.toml" || name.ends_with(".child.toml") || name.ends_with(".toml")
                },
            ) {
                Some(asset) => asset,
                None => continue,
            };

            let checksums_asset = select_asset(
                &release.assets,
                config.asset_name_checksums.as_deref(),
                |name| name == "checksums.txt" || name.ends_with("checksums.txt"),
            );

            let hashes =
                self.resolve_hashes(&release.assets, wasm_asset, manifest_asset, checksums_asset)?;

            let Some((artifact_sha256, manifest_sha256, checksums_url)) = hashes else {
                continue;
            };

            let child_name = config
                .child_name
                .clone()
                .or_else(|| wasm_asset.name.strip_suffix(".wasm").map(|s| s.to_string()))
                .unwrap_or_else(|| config.repo.clone());
            discovered.push(DiscoveredChildRelease {
                child_name,
                version,
                source_release_ref: release.tag_name,
                artifact_url: wasm_asset.browser_download_url.clone(),
                manifest_url: manifest_asset.browser_download_url.clone(),
                checksums_url,
                artifact_sha256,
                manifest_sha256,
                signature_ref: None,
                patina_min: config.patina_min.clone(),
                operations_json: None,
                needs_toys_json: None,
                needs_scopes_json: None,
            });
        }

        Ok(discovered)
    }
}

impl GitHubChildRegistryProvider {
    fn fetch_releases(&self, config: &GitHubSourceConfig) -> Result<Vec<GitHubRelease>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=100",
            config.owner, config.repo
        );

        let mut request = self
            .client
            .get(url)
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(USER_AGENT, "patina-child-registry");

        if let Some(token) = github_token() {
            request = request.header(AUTHORIZATION, format!("Bearer {}", token));
        }

        let response = request.send().context("requesting GitHub releases")?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .unwrap_or_else(|_| "<unavailable>".to_string());
            anyhow::bail!("GitHub releases API returned {}: {}", status, body);
        }

        response
            .json::<Vec<GitHubRelease>>()
            .context("parsing GitHub releases response")
    }

    fn resolve_hashes(
        &self,
        assets: &[GitHubReleaseAsset],
        wasm_asset: &GitHubReleaseAsset,
        manifest_asset: &GitHubReleaseAsset,
        checksums_asset: Option<&GitHubReleaseAsset>,
    ) -> Result<Option<(String, String, Option<String>)>> {
        let asset_by_name: HashMap<&str, &GitHubReleaseAsset> = assets
            .iter()
            .map(|asset| (asset.name.as_str(), asset))
            .collect();

        if let Some(checksums_asset) = checksums_asset {
            let checksums_body = self
                .fetch_text(&checksums_asset.browser_download_url)
                .with_context(|| {
                    format!("downloading checksums asset '{}'", checksums_asset.name)
                })?;
            let checksums = parse_checksums(&checksums_body);

            if let (Some(artifact), Some(manifest)) = (
                checksums.get(wasm_asset.name.as_str()),
                checksums.get(manifest_asset.name.as_str()),
            ) {
                return Ok(Some((
                    artifact.to_string(),
                    manifest.to_string(),
                    Some(checksums_asset.browser_download_url.clone()),
                )));
            }
        }

        let artifact_sidecar = format!("{}.sha256", wasm_asset.name);
        let manifest_sidecar = format!("{}.sha256", manifest_asset.name);

        let artifact_hash = if let Some(sidecar) = asset_by_name.get(artifact_sidecar.as_str()) {
            let text = self
                .fetch_text(&sidecar.browser_download_url)
                .with_context(|| format!("downloading sidecar '{}'", sidecar.name))?;
            extract_first_sha256(&text)
        } else {
            None
        };

        let manifest_hash = if let Some(sidecar) = asset_by_name.get(manifest_sidecar.as_str()) {
            let text = self
                .fetch_text(&sidecar.browser_download_url)
                .with_context(|| format!("downloading sidecar '{}'", sidecar.name))?;
            extract_first_sha256(&text)
        } else {
            None
        };

        Ok(match (artifact_hash, manifest_hash) {
            (Some(artifact_hash), Some(manifest_hash)) => {
                Some((artifact_hash, manifest_hash, None))
            }
            _ => None,
        })
    }

    fn fetch_text(&self, url: &str) -> Result<String> {
        let mut request = self
            .client
            .get(url)
            .header(ACCEPT, "application/octet-stream")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(USER_AGENT, "patina-child-registry");

        if let Some(token) = github_token() {
            request = request.header(AUTHORIZATION, format!("Bearer {}", token));
        }

        let response = request
            .send()
            .with_context(|| format!("requesting asset {}", url))?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("asset request returned HTTP {} for {}", status, url);
        }

        response.text().context("reading asset response body")
    }
}

#[derive(Debug, Deserialize)]
struct GitHubSourceConfig {
    owner: String,
    repo: String,
    #[serde(default)]
    child_name: Option<String>,
    #[serde(default)]
    tag_prefix: Option<String>,
    #[serde(default)]
    asset_name_wasm: Option<String>,
    #[serde(default)]
    asset_name_manifest: Option<String>,
    #[serde(default)]
    asset_name_checksums: Option<String>,
    #[serde(default)]
    include_prerelease: Option<bool>,
    #[serde(default)]
    patina_min: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

fn release_version_from_tag(tag_name: &str, tag_prefix: Option<&str>) -> Option<String> {
    let version = match tag_prefix {
        Some(prefix) => tag_name.strip_prefix(prefix)?,
        None => tag_name,
    };
    let version = version.strip_prefix('v').unwrap_or(version);
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn select_asset<'a>(
    assets: &'a [GitHubReleaseAsset],
    configured_name: Option<&str>,
    fallback: impl Fn(&str) -> bool,
) -> Option<&'a GitHubReleaseAsset> {
    if let Some(name) = configured_name {
        return assets.iter().find(|asset| asset.name == name);
    }
    assets.iter().find(|asset| fallback(&asset.name))
}

fn parse_checksums(content: &str) -> HashMap<String, String> {
    let mut checksums = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((name, hash)) = parse_bsd_checksum_line(line) {
            checksums.insert(name, hash);
            continue;
        }

        if let Some((name, hash)) = parse_gnu_checksum_line(line) {
            checksums.insert(name, hash);
        }
    }

    checksums
}

fn parse_bsd_checksum_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("SHA256 (")?;
    let (name, hash) = line.split_once(") = ")?;
    if !looks_like_sha256(hash) {
        return None;
    }
    Some((name.to_string(), hash.to_string()))
}

fn parse_gnu_checksum_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split_whitespace();
    let hash = parts.next()?;
    if !looks_like_sha256(hash) {
        return None;
    }
    let mut name = parts.next()?.to_string();
    if let Some(stripped) = name.strip_prefix('*') {
        name = stripped.to_string();
    }
    Some((name, hash.to_string()))
}

fn extract_first_sha256(content: &str) -> Option<String> {
    for token in content.split(|c: char| c.is_whitespace()) {
        if looks_like_sha256(token) {
            return Some(token.to_string());
        }
    }
    None
}

fn looks_like_sha256(input: &str) -> bool {
    input.len() == 64 && input.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn github_token() -> Option<String> {
    std::env::var("PATINA_GITHUB_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_version_from_tag_preserves_single_child_v_tags() {
        assert_eq!(
            release_version_from_tag("v0.2.0", None).as_deref(),
            Some("0.2.0")
        );
        assert_eq!(
            release_version_from_tag("0.2.0", None).as_deref(),
            Some("0.2.0")
        );
    }

    #[test]
    fn release_version_from_tag_filters_and_strips_child_prefixes() {
        assert_eq!(
            release_version_from_tag("folder-watch-actor-v0.1.0", Some("folder-watch-actor-v"))
                .as_deref(),
            Some("0.1.0")
        );
        assert_eq!(
            release_version_from_tag("folder-watch-actor-v0.1.0", Some("watch-null-sink-v")),
            None
        );
    }

    #[test]
    fn parse_checksums_supports_common_formats() {
        let input = r#"
8f6f1eb65f53f8f4f8d9f77a0f8c8e2ba3b72f8f2f07c4866b78f72300a4ad20  slate-manager.wasm
SHA256 (child.toml) = c26bcdf6529d8adf4ceac76714566491582f59d0bc889ef9e4d8ce96aa95f4c4
"#;
        let map = parse_checksums(input);
        assert_eq!(
            map.get("slate-manager.wasm").map(String::as_str),
            Some("8f6f1eb65f53f8f4f8d9f77a0f8c8e2ba3b72f8f2f07c4866b78f72300a4ad20")
        );
        assert_eq!(
            map.get("child.toml").map(String::as_str),
            Some("c26bcdf6529d8adf4ceac76714566491582f59d0bc889ef9e4d8ce96aa95f4c4")
        );
    }

    #[test]
    fn extract_first_sha256_reads_sidecar_text() {
        let content = "sha256: a3d24c4036f88fe4ca64f70556f0eae2e4ef6f878b6c51481e4a4e5c4b2b8f66\n";
        let hash = extract_first_sha256(content).expect("hash");
        assert_eq!(
            hash,
            "a3d24c4036f88fe4ca64f70556f0eae2e4ef6f878b6c51481e4a4e5c4b2b8f66"
        );
    }

    #[test]
    fn looks_like_sha256_rejects_invalid_values() {
        assert!(looks_like_sha256(
            "a3d24c4036f88fe4ca64f70556f0eae2e4ef6f878b6c51481e4a4e5c4b2b8f66"
        ));
        assert!(!looks_like_sha256("deadbeef"));
        assert!(!looks_like_sha256(
            "zzzz4c4036f88fe4ca64f70556f0eae2e4ef6f878b6c51481e4a4e5c4b2b8f66"
        ));
    }
}
