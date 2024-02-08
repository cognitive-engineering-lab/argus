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

fn get_user(conn: &mut PgConnection) {
  users::table.select(posts::id);
  users::table
    .filter(users::id.eq(posts::id))
    .load::<(i32, String)>(conn);
}
