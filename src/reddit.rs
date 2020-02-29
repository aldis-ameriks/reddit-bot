use reqwest::{Client, Response};
use serde_json::Value;

#[derive(Debug)]
pub struct Post {
    pub title: String,
    pub link: String,
}

impl Post {
    pub fn format(&self) -> String {
        format!("{}\n{}", &self.title, &self.link)
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

pub async fn fetch_posts(subreddit: &str) -> Result<Vec<Post>, RedditError> {
    let url = format!(
        "https://www.reddit.com/r/{}/top.json?limit=1&t=week",
        subreddit
    );
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
                Post {
                    title,
                    link: format!("https://www.reddit.com{}", link),
                }
            })
            .collect()
    } else {
        vec![]
    };

    println!("{:#?}", posts);

    Ok(posts)
}

pub async fn validate_subreddit(subreddit: &str) -> bool {
    let url = format!("https://www.reddit.com/r/{}", subreddit);

    if let Ok(resp) = reqwest::get(&url).await {
        resp.status().is_success()
    } else {
        false
    }
}
