use diesel::prelude::*;

table! {
    users(id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Queryable)]
struct User {
  name: String,
  id: i32,
}

fn get_user(conn: &mut PgConnection) -> QueryResult<User> {
  users::table.first(conn)
}
