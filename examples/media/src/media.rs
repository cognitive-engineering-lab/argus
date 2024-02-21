pub struct WebArchive {}

impl WebArchive {
  pub fn new(
    _domain: impl ToString,
    _username: impl ToString,
    _password: impl ToString,
  ) -> Self {
    WebArchive {}
  }

  pub fn register<T: Summary>(&mut self, item: T) {
    println!("Registered: {}", item.summarize());
  }

  pub fn publish(self) {
    println!("Published!");
  }
}

pub trait Summary {
  fn summarize(&self) -> String;
}

pub struct NewsArticle {
  pub headline: String,
  pub content: String,
}

impl NewsArticle {
  pub fn new(headline: impl ToString, content: impl ToString) -> Self {
    NewsArticle {
      headline: headline.to_string(),
      content: content.to_string(),
    }
  }
}

pub struct Play {
  pub title: String,
  pub author: String,
  pub content: String,
}

impl Play {
  pub fn new(
    title: impl ToString,
    author: impl ToString,
    content: impl ToString,
  ) -> Self {
    Play {
      title: title.to_string(),
      author: author.to_string(),
      content: content.to_string(),
    }
  }
}

pub struct Book {
  pub title: String,
  pub author: String,
  pub content: String,
}

impl Book {
  pub fn new(
    title: impl ToString,
    author: impl ToString,
    content: impl ToString,
  ) -> Self {
    Self {
      title: title.to_string(),
      author: author.to_string(),
      content: content.to_string(),
    }
  }
}

impl Summary for String {
  fn summarize(&self) -> String {
    self.clone()
  }
}

impl Summary for Play {
  fn summarize(&self) -> String {
    format!("\"{}\" by {}", self.title, self.author)
  }
}

impl Summary for Book {
  fn summarize(&self) -> String {
    format!("{}, by {}", self.title, self.author)
  }
}

impl Summary for NewsArticle {
  fn summarize(&self) -> String {
    format!("{}", self.headline)
  }
}

impl<T: Summary, U: Summary> Summary for (T, U) {
  fn summarize(&self) -> String {
    format!("({}, {})", self.0.summarize(), self.1.summarize())
  }
}

impl<U: Summary> Summary for (usize, U) {
  fn summarize(&self) -> String {
    format!("(DOI: {}, {})", self.0, self.1.summarize())
  }
}

impl<T: Summary> Summary for Vec<T> {
  fn summarize(&self) -> String {
    format!(
      "[{}]",
      self
        .iter()
        .map(|item| item.summarize())
        .collect::<Vec<_>>()
        .join(", ")
    )
  }
}
