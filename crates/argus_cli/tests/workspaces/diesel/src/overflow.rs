use diesel::prelude::*; // diesel 2.0.4, features = ["postgres"]

table! {
    users {
        id -> Integer,
        posts -> Text,
    }
}

fn test(conn: &mut PgConnection) {
  users::table
    .into_boxed()
    .group_by(users::id)
    .load::<(i32, String)>(conn);
}
