use diesel::prelude::*;

table! {
    users(id) {
        id -> Integer,
        name -> Text,
    }
}

// #[derive(QueryableByName)]
struct User {
  id: i32,
  name: String,
}

fn sql_query_test(conn: &mut PgConnection) -> QueryResult<Vec<User>> {
  diesel::sql_query("…").load(conn)
}
