use log::{error, warn};
use serde_json::Value;

use super::error::RedditError;
use super::post::Post;

pub struct RedditClient {
    base_url: String,
}

impl RedditClient {
    pub fn new() -> Self {
        RedditClient {
            base_url: String::from("https://reddit.com"),
        }
    }

    #[allow(dead_code)]
    pub fn new_with(base_url: &str) -> Self {
        RedditClient {
            base_url: base_url.to_string(),
        }
    }

    pub async fn fetch_posts(&self, subreddit: &str) -> Result<Vec<Post>, RedditError> {
        let url = format!("{}/r/{}/top.json?limit=10&t=week", self.base_url, subreddit);
        let res = reqwest::get(&url).await?;
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

        if let Ok(resp) = reqwest::get(&url).await {
            resp.status().is_success()
        } else {
            false
        }
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
