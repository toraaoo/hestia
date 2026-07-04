//! Eclipse Temurin release catalogue via the Adoptium API. Fetches metadata
//! only; downloading, extraction, and registration live in `Java`.

use anyhow::{bail, Context, Result};
use proto::download::{Checksum, HashAlgorithm};
use proto::java::JavaRelease;
use serde_json::Value;

use super::platform::JavaTarget;

const API_BASE: &str = "https://api.adoptium.net";

pub struct JavaPackage {
    pub vendor: String,
    pub major: i32,
    pub release_name: String,
    pub url: String,
    pub archive_name: String,
    pub checksum: Checksum,
}

async fn fetch_json(url: &str, query: &[(&str, &str)]) -> Result<Value> {
    tracing::debug!(url, "adoptium GET");
    let response = reqwest::Client::new()
        .get(url)
        .query(query)
        .send()
        .await
        .with_context(|| "adoptium request failed")?;
    if !response.status().is_success() {
        bail!(
            "adoptium request failed: HTTP {}",
            response.status().as_u16()
        );
    }
    response
        .json()
        .await
        .context("adoptium returned malformed JSON")
}

pub async fn releases() -> Result<Vec<JavaRelease>> {
    let j = fetch_json(&format!("{API_BASE}/v3/info/available_releases"), &[]).await?;
    releases_from_json(&j)
}

pub async fn resolve(major: i32, target: &JavaTarget) -> Result<JavaPackage> {
    let url = format!("{API_BASE}/v3/assets/latest/{major}/hotspot");
    let assets = fetch_json(
        &url,
        &[
            ("os", &target.os),
            ("architecture", &target.arch),
            ("image_type", "jdk"),
            ("vendor", "eclipse"),
        ],
    )
    .await?;
    package_from_json(&assets, major, target)
}

fn releases_from_json(j: &Value) -> Result<Vec<JavaRelease>> {
    let available = j
        .get("available_releases")
        .and_then(Value::as_array)
        .context("adoptium response is missing available_releases")?;
    let lts = j
        .get("available_lts_releases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut releases: Vec<JavaRelease> = available
        .iter()
        .filter_map(|m| m.as_i64())
        .map(|major| JavaRelease {
            major: major as i32,
            lts: lts.iter().any(|l| l.as_i64() == Some(major)),
        })
        .collect();
    releases.sort_by_key(|r| r.major);
    Ok(releases)
}

fn package_from_json(assets: &Value, major: i32, target: &JavaTarget) -> Result<JavaPackage> {
    let assets = assets
        .as_array()
        .context("adoptium assets response is not an array")?;
    for asset in assets {
        let binary = asset.get("binary").cloned().unwrap_or(Value::Null);
        let os = binary.get("os").and_then(Value::as_str).unwrap_or_default();
        let arch = binary
            .get("architecture")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let image = binary
            .get("image_type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if os != target.os || arch != target.arch || image != "jdk" {
            continue;
        }
        let package = binary.get("package").cloned().unwrap_or(Value::Null);
        let resolved = JavaPackage {
            vendor: "temurin".to_string(),
            major,
            release_name: asset
                .get("release_name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            url: package
                .get("link")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            archive_name: package
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            checksum: Checksum {
                algorithm: HashAlgorithm::Sha256,
                hex: package
                    .get("checksum")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            },
        };
        if resolved.url.is_empty()
            || resolved.archive_name.is_empty()
            || !resolved.checksum.is_valid()
        {
            bail!("adoptium build for temurin {major} is missing its download link or checksum");
        }
        return Ok(resolved);
    }
    bail!(
        "no temurin {major} jdk build is published for {}/{}",
        target.os,
        target.arch
    )
}
