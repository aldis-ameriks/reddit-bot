use log::warn;
use serde_json::Value;
use std::fmt;

#[derive(Debug)]
pub struct RedditPost {
    pub title: String,
    pub link: String,
}

impl fmt::Display for RedditPost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n{}\n", self.title, self.link)
    }
}

pub enum RedditError {
    NetworkError(reqwest::Error),
    MalformedResponse(serde_json::error::Error),
}

impl From<reqwest::Error> for RedditError {
    fn from(error: reqwest::Error) -> Self {
        RedditError::NetworkError(error)
    }
}

impl From<serde_json::error::Error> for RedditError {
    fn from(error: serde_json::error::Error) -> Self {
        RedditError::MalformedResponse(error)
    }
}

pub struct RedditClient {
    base_url: String,
}

impl RedditClient {
    pub fn new() -> Self {
        RedditClient {
            base_url: String::from("https://reddit.com"),
        }
    }

    pub fn new_with(base_url: &str) -> Self {
        RedditClient {
            base_url: base_url.to_string(),
        }
    }

    pub async fn fetch_posts(&self, subreddit: &str) -> Result<Vec<RedditPost>, RedditError> {
        let url = format!("{}/r/{}/top.json?limit=10&t=week", self.base_url, subreddit);
        let res = reqwest::get(&url).await?;
        let body = res.text().await?;
        let body: Value = serde_json::from_str(&body)?;
        let children = body.get("data").unwrap().get("children").unwrap();

        let posts = if let Value::Array(children) = children {
            children
                .iter()
                .map(|child| {
                    let title = child.get("data").unwrap().get("title").unwrap();
                    let link = child.get("data").unwrap().get("permalink").unwrap();
                    let title = if let Value::String(v) = title { v } else { "" }.to_string();
                    let link = if let Value::String(v) = link { v } else { "" }.to_string();
                    RedditPost {
                        title,
                        link: format!("{}{}", self.base_url, link),
                    }
                })
                .collect()
        } else {
            warn!("response did not contain an array");
            vec![]
        };

        Ok(posts)
    }

    pub async fn validate_subreddit(&self, subreddit: &str) -> bool {
        let url = format!("{}/r/{}", self.base_url, subreddit);

        if let Ok(resp) = reqwest::get(&url).await {
            resp.status().is_success()
        } else {
            false
        }
    }
}
