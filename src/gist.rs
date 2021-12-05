use crate::{store::MemoryTiddlersStore, GITHUB_GIST_ID, GITHUB_GIST_TOKEN};
use actix_web::{client::Client, http::StatusCode, HttpMessage};
use futures::StreamExt;
use log::info;
use serde_json::json;
use serde_json::Value;

pub async fn backup_to_gist(data: String) {
    let client = Client::default();
    let value = json!({"files":{"wiki_data.json":{"content":data}}});

    // Create request builder and send request
    let response = client
        .patch(format!(
            "https://api.github.com/gists/{}",
            GITHUB_GIST_ID.as_str()
        ))
        .header("User-Agent", "actix-web/3.0")
        .header("Accept", "application/vnd.github.v3+json")
        .header(
            "Authorization",
            format!("bearer {}", GITHUB_GIST_TOKEN.as_str()),
        )
        .send_body(value)
        .await; // <- Wait for response
    if let Ok(res) = &response {
        if res.status() != StatusCode::OK {
            info!("Response: {:?}", response);
        }
    } else {
        info!("Response: {:?}", response);
    }
}

pub async fn pull_from_gist() -> Option<MemoryTiddlersStore> {
    let client = Client::default();

    // Create request builder and send request
    let mut response = client
        .get(format!(
            "https://api.github.com/gists/{}",
            GITHUB_GIST_ID.as_str()
        ))
        .header("User-Agent", "curl/7.80.0")
        .header("Accept", "application/vnd.github.v3+json")
        .header(
            "Authorization",
            format!("bearer {}", GITHUB_GIST_TOKEN.as_str()),
        )
        .send()
        .await; // <- Wait for response
    if let Ok(res) = &mut response {
        if res.status() == StatusCode::OK {
            let mut body = actix_web::web::BytesMut::new();
            let mut payload = res.take_payload();
            while let Some(chunk) = payload.next().await {
                let chunk = chunk.ok()?;
                body.extend_from_slice(&chunk);
            }
            let body = serde_json::from_slice::<Value>(&body).unwrap_or(Default::default());
            if let Some(content) = body["files"]["wiki_data.json"]["content"].as_str() {
                if let Ok(d) = serde_json::from_str(content) {
                    return Some(d);
                }
            }
        }
    }
    info!("Response: {:?}", response);
    None
}
