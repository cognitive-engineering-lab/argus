use diesel::prelude::*;

fn sql_query_test(conn: &mut PgConnection) -> QueryResult<Vec<String>> {
  diesel::sql_query("…").load(conn)
}
