use diesel::{prelude::*, sql_types::Text};

table! {
    users(id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Insertable)]
struct User {
  name: Text,
}
