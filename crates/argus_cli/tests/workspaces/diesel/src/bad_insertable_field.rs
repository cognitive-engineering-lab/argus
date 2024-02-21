use diesel::prelude::*;

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
