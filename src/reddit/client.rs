use log::{error, warn};
use reqwest::Client;
use serde_json::Value;
use tokio::time::{sleep, Duration};
use ua_generator::ua::spoof_ua;

use super::error::RedditError;
use super::post::Post;

pub struct RedditClient {
    base_url: String,
}

impl RedditClient {
    pub fn new() -> Self {
        RedditClient::new_with("https://reddit.com")
    }

    pub fn new_with(base_url: &str) -> Self {
        RedditClient {
            base_url: base_url.to_string(),
        }
    }

    pub async fn fetch_posts(&self, subreddit: &str) -> Result<Vec<Post>, RedditError> {
        let url = format!("{}/r/{}/top.json?limit=10&t=week", self.base_url, subreddit);
        let client = self.get_client();
        let res = client.get(&url).send().await?;

        if let Some(remaining) = res.headers().get("x-ratelimit-remaining") {
            let remaining_request_count: u64 =
                remaining.to_str().unwrap().to_string().parse::<f64>().unwrap().trunc() as u64;
            if remaining_request_count < 20 {
                if let Some(reset) = res.headers().get("x-ratelimit-reset") {
                    let reset: u64 = reset.to_str().unwrap().to_string().parse().unwrap();
                    warn!(
                        "running out of remaining reddit requests, sleeping for: {} seconds",
                        reset
                    );
                    sleep(Duration::from_secs(reset)).await;
                }
            }
        }

        let body = res.text().await?;
        let body: Value = serde_json::from_str(&body)?;

        let data = body.get("data");
        if None == data {
            error!("Missing data in response for subreddit: {}", subreddit);
            return Err(RedditError::Error);
        }

        let children = data.unwrap().get("children");

        if None == children {
            error!("Missing children in response for subreddit: {}", subreddit);
            return Err(RedditError::Error);
        }

        let children = children.unwrap();

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
        let client = self.get_client();

        if let Ok(resp) = client.get(&url).send().await {
            resp.status().is_success()
        } else {
            false
        }
    }

    fn get_client(&self) -> Client {
        Client::builder().user_agent(spoof_ua()).build().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use mockito::{mock, server_url};

    use super::*;
    use crate::reddit::test_helpers::mock_reddit_success;

    #[test]
    fn correct_domain() {
        let reddit_client = RedditClient::new();
        assert_eq!(reddit_client.base_url, "https://reddit.com");
    }

    #[tokio::test]
    async fn fetch_posts_success() {
        let url = &server_url();
        let subreddit = "rust";
        let _m = mock_reddit_success(subreddit);
        let reddit_client = RedditClient::new_with(url);
        let result = reddit_client.fetch_posts(subreddit).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            Post {
                title: "A half-hour to learn Rust".to_string(),
                link: format!("{}/r/rust/comments/fbenua/a_halfhour_to_learn_rust/", url),
            }
        );
        _m.assert();
    }

    #[tokio::test]
    async fn fetch_posts_invalid_children() {
        let url = &server_url();

        let body = r#"{
            "kind": "Listing",
              "data": {
                "modhash": "hiv37z7c0he911a48bb0560150060fd86b7e0af8182dc97e68",
                "dist": 1,
                "children": "xxx",
                "after": "t3_fbenua",
                "before": null
              }
            }
         "#;
        let subreddit = "rust";
        let _m = mock(
            "GET",
            format!("/r/{}/top.json?limit=10&t=week", subreddit).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

        let reddit_client = RedditClient::new_with(url);
        let result = reddit_client.fetch_posts(subreddit).await.unwrap();
        assert_eq!(result.len(), 0);
        _m.assert();
    }

    #[tokio::test]
    async fn fetch_posts_missing_data() {
        let url = &server_url();

        let body = r#"{
            "kind": "Listing"
            }
         "#;
        let subreddit = "rust";
        let _m = mock(
            "GET",
            format!("/r/{}/top.json?limit=10&t=week", subreddit).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

        let reddit_client = RedditClient::new_with(url);
        let result = reddit_client.fetch_posts(subreddit).await;
        assert_eq!(result.is_err(), true);
        _m.assert();
    }

    #[tokio::test]
    async fn validate_subreddit_success() {
        let url = &server_url();

        let subreddit = "rust";
        let _m = mock("GET", format!("/r/{}", subreddit).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .create();

        let reddit_client = RedditClient::new_with(url);
        let result = reddit_client.validate_subreddit(subreddit).await;
        assert_eq!(result, true);
        _m.assert();
    }

    #[tokio::test]
    async fn validate_subreddit_invalid() {
        let url = &server_url();

        let subreddit = "rust";
        let _m = mock("GET", format!("/r/{}", subreddit).as_str())
            .with_status(404)
            .with_header("content-type", "application/json")
            .create();

        let reddit_client = RedditClient::new_with(url);
        let result = reddit_client.validate_subreddit(subreddit).await;
        assert_eq!(result, false);
        _m.assert();
    }
}
