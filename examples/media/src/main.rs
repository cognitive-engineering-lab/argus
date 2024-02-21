mod media;
use media::*;

fn main() {
  let mut archive =
    WebArchive::new("https://library.example.gov", "best-libarian", "b00k5!");

  archive.register(NewsArticle::new(
    "Penguins win the Stanley Cup Championship!",
    "The Pittsburgh Penguins once again are the best hockey team in the NHL.",
  ));

  archive.register(Book::new(
    "Alice's Adventures in Wonderland",
    "Lewis Carroll",
    "Alice was beginning to get very tired of sitting by her sister on the bank, and of having nothing to do: once or twice she had peeped into the book her sister was reading, but it had no pictures or conversations in it, “and what is the use of a book,” thought Alice “without pictures or conversations?”"
  ));

  archive.register((
    Book::new("Frankenstein", "Mary Shelley", "You will rejoice to hear that no disaster has accompanied the commencement of an enterprise which you have regarded with such evil forebodings. I arrived here yesterday, and my first task is to assure my dear sister of my welfare and increasing confidence in the success of my undertaking."),
    NewsArticle::new(
      "Frankenstein is real!",
      "Find out where he's been living all these years.",
    ),
  ));

  archive.register(vec![
    NewsArticle::new("Queen Elizabeth II", "The British Monarch is a figurehead of the British people."),
    NewsArticle::new("Queen Elizabeth II dies at 96", "The British Monarch has passed away."),
    NewsArticle::new("Thousands pay Tribute as Britain Says Final Farewell to Its Queen", "More than 100 world leaders and dignitaries are expected to attend the funeral of Queen Elizabeth II."),
  ]);

  archive.register(
    ("William Shakespeare", vec![
      Play::new("Romeo and Juliet", "William Shakespeare", "Romeo: But, soft! what light through yonder window breaks?"),
      Play::new("Hamlet", "William Shakespeare", "Hamlet: To be, or not to be, that is the question:"),
      Play::new("Macbeth", "William Shakespeare", "Macbeth: Is this a dagger which I see before me, The handle toward my hand?")
    ]),
  );

  archive.publish();
}
