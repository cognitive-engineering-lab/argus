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

allow_tables_to_appear_in_same_query!(users, posts,);

// Select all users who have a post with the same id as their user id.
// Return the user's id, name, and the post's name.
fn get_user(conn: &mut PgConnection) {
  users::table
    // .inner_join(posts::table.on(users::id.eq(posts::id)))
    .filter(users::id.eq(posts::id))
    .select((users::id, users::name))
    .load::<(i32, String)>(conn);
}
