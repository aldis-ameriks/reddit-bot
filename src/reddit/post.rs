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
