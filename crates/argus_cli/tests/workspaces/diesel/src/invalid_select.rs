use diesel::prelude::*;

table! {
    users(id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts(id) {
        id -> Integer,
        name -> Text,
        user_id -> Integer,
    }
}

fn bad_select(conn: &mut PgConnection) {
  users::table.select(posts::id).load::<i32>(conn);
}
