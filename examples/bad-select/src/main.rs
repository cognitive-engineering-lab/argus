use diesel::prelude::*;

allow_tables_to_appear_in_same_query!(
    users, posts,
);

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

fn query(conn: &mut PgConnection) {
    users::table
        .filter(users::id.eq(posts::id))
        .select((
            users::id,
            users::name,
        ))
        .load::<(i32, String)>(conn);
}

fn main() {}
