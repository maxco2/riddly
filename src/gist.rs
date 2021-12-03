use crate::{GITHUB_GIST_ID, GITHUB_GIST_TOKEN};
use actix_web::{client::Client, http::StatusCode};
use log::info;
use serde_json::json;

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
