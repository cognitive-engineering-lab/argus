//TODO `argus bundle` fails for this module,
//we're not "totally" sure why, but there are
//macros and lots of traits involved...
//mod bad_insertable_field;
mod invalid_query;
mod invalid_select;
mod overflow;

// mod bad_sql_query; // Currerently hangs rustc
// mod queryable_order_mismatch; // Currerently hangs rustc

fn main() {}
