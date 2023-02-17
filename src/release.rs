use crate::{error::MyError, platform};
use regex::Regex;
use reqwest::{self, header, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs::OpenOptions;

#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
struct ReleasesAssets {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ReleasesData {
    tag_name: String,
    name: String,
    draft: bool,
    prerelease: bool,
    published_at: String,
    assets: Vec<ReleasesAssets>,
}

impl ReleasesData {
    fn available(&self) -> bool {
        !self.draft && !self.prerelease && self.assets.len() != 0
    }

    async fn platforms(&self) -> serde_json::Value {
        let mut value = json!({});
        let map = hashmap! {
            "x64_zh-TW.msi.zip" => "windows-x86_64",
            "x86_zh-TW.msi.zip" => "windows-i686",
            "amd64.AppImage.tar.gz" => "linux-x86_64",
            "app.tar.gz" => "darwin-x86_64"
        };

        for (k, v) in map {
            let re = Regex::new(&format!("{k}$")).unwrap();
            let sig_re = Regex::new(&format!("{k}.sig$")).unwrap();
            if let Some(asset) = self.assets.iter().find(|a| re.is_match(&a.name)) {
                let url = &asset.browser_download_url;
                if let Some(asset) = self.assets.iter().find(|a| sig_re.is_match(&a.name)) {
                    let sig_url = &asset.browser_download_url;
                    match reqwest::get(sig_url).await {
                        Ok(response) => match response.text().await {
                            Ok(signature) => value[v] = platform::new(&signature, url),
                            Err(_) => value[v] = platform::new("", url),
                        },
                        Err(e) => eprintln!("Error while download sig file: {e}"),
                    };
                }
            }
        }

        value
    }

    pub async fn summon(&self) -> std::io::Result<()> {
        let note = env::var("NOTES").unwrap_or(self.name.clone());
        let path = env::var("SAVE_PATH").unwrap_or("versions.json".to_string());
        let value = json!({
            "version": self.tag_name,
            "notes": note,
            "pub_date": self.published_at,
            "platforms": self.platforms().await
        });

        let file = OpenOptions::new().write(true).create(true).open(path)?;

        Ok(serde_json::to_writer_pretty(file, &value)?)
    }
}

pub async fn get_release_latest() -> Result<ReleasesData, Box<dyn std::error::Error>> {
    let owner = env::var("OWNER").expect("OWNER variavles is must");
    let repo = env::var("REPO").expect("REPO variavles is must");
    let cargo_version = env!("CARGO_PKG_VERSION");
    let user_agent = format!("RustRuntime/{cargo_version}");
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases");
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "User-Agent",
        header::HeaderValue::from_str(&user_agent).unwrap(),
    );
    if let Ok(token) = env::var("TOKEN") {
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
    };
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();
    let resp = client.get(&url).send().await?;
    if resp.status() == StatusCode::NOT_FOUND {
        return Err(Box::new(MyError::new(&format!("Not Found: {url}"))));
    }

    let res = client
        .get(url)
        .send()
        .await?
        .json::<Vec<ReleasesData>>()
        .await?;

    let release_latest = res
        .into_iter()
        .filter(|r| r.available())
        .collect::<Vec<ReleasesData>>()[0]
        .clone();

    Ok(release_latest)
}
