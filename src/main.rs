use serde_json::Value;
use std::env;

mod telegram;

#[derive(Debug)]
struct Post {
    title: String,
    link: String,
}

impl Post {
    fn format(&self) -> String {
        format!("{}\n{}", &self.title, &self.link)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("TG_TOKEN").expect("Missing TG_TOKEN env var");
    let chat_id = env::var("TG_CHAT_ID").expect("Missing TG_CHAT_ID env var");

    let telegram_client = telegram::TelegramClient::new(token, chat_id);
    let subreddits = ["rust", "arduino", "Whatcouldgowrong"];
    for subreddit in subreddits.iter() {
        let posts = fetch_posts(subreddit).await?;
        for post in posts.iter() {
            telegram_client.send_message(&post.format()).await?;
        }
    }

    Ok(())
}

async fn fetch_posts(subreddit: &str) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
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
