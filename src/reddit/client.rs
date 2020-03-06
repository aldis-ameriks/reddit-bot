use log::warn;
use serde_json::Value;

use super::error::Error;
use super::post::Post;

pub struct Client {
    base_url: String,
}

impl Client {
    pub fn new() -> Self {
        Client {
            base_url: String::from("https://reddit.com"),
        }
    }

    pub fn new_with(base_url: &str) -> Self {
        Client {
            base_url: base_url.to_string(),
        }
    }

    pub async fn fetch_posts(&self, subreddit: &str) -> Result<Vec<Post>, Error> {
        let url = format!("{}/r/{}/top.json?limit=10&t=week", self.base_url, subreddit);
        let res = reqwest::get(&url).await?;
        let body = res.text().await?;
        let body: Value = serde_json::from_str(&body)?;
        let children = body
            .get("data")
            .expect("Missing data")
            .get("children")
            .expect("Missing children");

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

    #[test]
    fn correct_domain() {
        let reddit_client = Client::new();
        assert_eq!(reddit_client.base_url, "https://reddit.com");
    }

    #[tokio::test]
    async fn fetch_posts_success() {
        let url = &server_url();

        let body = r#"{"kind": "Listing", "data": {"modhash": "hiv37z7c0he911a48bb0560150060fd86b7e0af8182dc97e68", "dist": 1, "children": [{"kind": "t3", "data": {"approved_at_utc": null, "subreddit": "rust", "selftext": "", "author_fullname": "t2_2stz", "saved": false, "mod_reason_title": null, "gilded": 0, "clicked": false, "title": "A half-hour to learn Rust", "link_flair_richtext": [], "subreddit_name_prefixed": "r/rust", "hidden": false, "pwls": 6, "link_flair_css_class": null, "downs": 0, "hide_score": false, "name": "t3_fbenua", "quarantine": false, "link_flair_text_color": "dark", "author_flair_background_color": null, "subreddit_type": "public", "ups": 567, "total_awards_received": 0, "media_embed": {}, "author_flair_template_id": null, "is_original_content": false, "user_reports": [], "secure_media": null, "is_reddit_media_domain": false, "is_meta": false, "category": null, "secure_media_embed": {}, "link_flair_text": null, "can_mod_post": false, "score": 567, "approved_by": null, "author_premium": true, "thumbnail": "", "edited": false, "author_flair_css_class": null, "author_flair_richtext": [], "gildings": {}, "content_categories": null, "is_self": false, "mod_note": null, "created": 1583021451.0, "link_flair_type": "text", "wls": 6, "removed_by_category": null, "banned_by": null, "author_flair_type": "text", "domain": "fasterthanli.me", "allow_live_comments": false, "selftext_html": null, "likes": null, "suggested_sort": null, "banned_at_utc": null, "view_count": null, "archived": false, "no_follow": false, "is_crosspostable": true, "pinned": false, "over_18": false, "all_awardings": [], "awarders": [], "media_only": false, "can_gild": true, "spoiler": false, "locked": false, "author_flair_text": null, "visited": false, "removed_by": null, "num_reports": null, "distinguished": null, "subreddit_id": "t5_2s7lj", "mod_reason_by": null, "removal_reason": null, "link_flair_background_color": "", "id": "fbenua", "is_robot_indexable": true, "report_reasons": null, "author": "koavf", "discussion_type": null, "num_comments": 80, "send_replies": true, "whitelist_status": "all_ads", "contest_mode": false, "mod_reports": [], "author_patreon_flair": false, "author_flair_text_color": null, "permalink": "/r/rust/comments/fbenua/a_halfhour_to_learn_rust/", "parent_whitelist_status": "all_ads", "stickied": false, "url": "https://fasterthanli.me/blog/2020/a-half-hour-to-learn-rust/", "subreddit_subscribers": 92729, "created_utc": 1582992651.0, "num_crossposts": 1, "media": null, "is_video": false}}], "after": "t3_fbenua", "before": null}}"#;
        let subreddit = "rust";
        let _m = mock(
            "GET",
            format!("/r/{}/top.json?limit=10&t=week", subreddit).as_str(),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

        let reddit_client = Client::new_with(url);
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

        let reddit_client = Client::new_with(url);
        let result = reddit_client.fetch_posts(subreddit).await.unwrap();
        assert_eq!(result.len(), 0);
        _m.assert();
    }

    #[tokio::test]
    #[should_panic(expected = "Missing data")]
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

        let reddit_client = Client::new_with(url);
        reddit_client.fetch_posts(subreddit).await.unwrap();
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

        let reddit_client = Client::new_with(url);
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

        let reddit_client = Client::new_with(url);
        let result = reddit_client.validate_subreddit(subreddit).await;
        assert_eq!(result, false);
        _m.assert();
    }
}
