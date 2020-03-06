use std::fmt;

#[derive(Debug)]
pub struct Post {
    pub title: String,
    pub link: String,
}

impl fmt::Display for Post {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n{}\n", self.title, self.link)
    }
}

impl std::cmp::PartialEq for Post {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.link == other.link
    }

    fn ne(&self, other: &Self) -> bool {
        self.title != other.title || self.link != other.link
    }
}
