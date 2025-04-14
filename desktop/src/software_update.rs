// Checking for system updates from GitHub

use std::time::Duration;

use anyhow::Result;
use semver::Version;
use serde::Deserialize;
use tokio::{sync::mpsc::UnboundedSender, time::sleep};

use crate::get_reqwest_client;

#[derive(Debug, Clone)]
enum GitHubError {
    UnknownError,
    NotFound,
    FailedToParse,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ReleaseUser {
    pub login: String,
    pub id: u64,
    // pub node_id: String,
    // pub avatar_url: String,
    // pub gravatar_id: String,
    // pub url: String,
    // pub html_url: String,
    // pub followers_url: String,
    // pub following_url: String,
    // pub gists_url: String,
    // pub starred_url: String,
    // pub subscriptions_url: String,
    // pub organizations_url: String,
    // pub repos_url: String,
    // pub events_url: String,
    // pub received_events_url: String,
    // pub r#type: String, // Change to enum?
    // pub user_view_type: String,
    // pub site_admin: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ReleaseAsset {
    // pub url: String,
    pub id: u64,
    // pub node_id: String,
    pub name: String,
    // pub label: Option<String>,
    // pub uploader: ReleaseUser,
    // pub content_type: String,
    // pub state: String, // Change to enum?
    pub size: u64,
    // pub download_count: u64,
    // pub created_at: String, // Change to chrono type?
    // pub updated_at: String, // Change to chrono type?
    pub browser_download_url: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ReleaseInfo {
    // pub url: String,
    // pub assets_url: String,
    // pub upload_url: String,
    // pub html_url: String,
    pub id: u64,
    pub author: ReleaseUser,
    // pub node_id: String,
    pub tag_name: String,
    // pub target_commitish: String,
    pub name: String,
    pub draft: bool,
    pub prerelease: bool,
    // pub created_at: String,   // Change to chrono type?
    // pub published_at: String, // Change to chrono type?
    pub assets: Vec<ReleaseAsset>,
    pub tarball_url: String,
    pub zipball_url: String,
    pub body: String,
}

async fn get_release(
    owner: impl ToString,
    repository: impl ToString,
    release: impl ToString,
) -> Result<ReleaseInfo, GitHubError> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/{}",
        owner.to_string(),
        repository.to_string(),
        release.to_string()
    );
    match get_reqwest_client().get(url).send().await {
        Ok(r) => {
            if r.status().is_success() {
                r.json().await.map_err(|_| GitHubError::FailedToParse)
            } else if r.status().is_client_error() {
                Err(GitHubError::NotFound)
            } else {
                Err(GitHubError::UnknownError)
            }
        }
        Err(_) => Err(GitHubError::UnknownError),
    }
}

pub async fn software_update_task(
    update_available_signal: UnboundedSender<(Version, String)>,
) -> Result<()> {
    let this_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

    loop {
        match get_release("FriendTeamInc", "JukeBox", "latest").await {
            Ok(release) => {
                let new_version = release.tag_name.replace("v", "");
                let new_version = Version::parse(&new_version).unwrap();
                if new_version > this_version {
                    log::info!("new version available! {}", new_version);
                    let _ = update_available_signal.send((new_version, release.body));
                }
            }
            Err(e) => {
                log::warn!("software_update: {:?}", e)
            }
        }

        sleep(Duration::from_secs(86400)).await;
    }

    // Ok(())
}
