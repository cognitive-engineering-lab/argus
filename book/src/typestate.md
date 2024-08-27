# Trait Methods and Typestate

Every programming language cultivates its own set of patterns. One pattern common in Rust is the *builder pattern.* Some data structures are complicated to construct, they may require a large number of inputs, or have complex configuration.

A great example of working with builders is the Diesel [`QueryDsl`](https://docs.rs/diesel/latest/diesel/prelude/trait.QueryDsl.html#). The `QueryDsl` trait exposes a number of methods to construct a valid SQL query. Each method consumes the caller, and returns a type that itself implements `QueryDsl`. As an example here's the method signature for `select`

```rust,ignore
fn select<Selection>(self, selection: Selection) -> Select<Self, Selection>
  where 
    Selection: Expression,
    Self: SelectDsl<Selection> { /* ... */ }
```

The `QueryDsl` demonstrates the complexity allowed by the builder pattern, it ensures valid SQL queries by encoding query semantics in Rust traits. One drawback of this pattern is that error diagnostics become difficult to understand as your types get larger and the traits involved more complex. In this chapter we will walk through how to debug and understand a trait error involving the builder pattern, or as some people will call it, *typestate.* We refer to this pattern as typestate because each method returns a type in a particular state, the methods available to the resulting type depend on its state. Calling methods in the wrong order, or forgetting a method, can result in the wrong state for the next method you'd like to call. Let's walk through an example.

```rust,ignore
{{#include ../../examples/bad-select/src/main.rs:7:30}}
```

Running `cargo check` produces the following verbose diagnostic.

```text
error[E0271]: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
    --> src/main.rs:29:32     
     |
29   | ...  .load::<(i32, String)>(con...
     |       ----                  ^^^^ expected `Once`, found `Never`
     |       |
     |       required by a bound introduced by this call
     |
note: required for `posts::columns::id` to implement `AppearsOnTable<users::table>`
    --> src/main.rs:16:9      
     |
16   | ...   id -> ...
     |       ^^
     = note: associated types for the current `impl` cannot be restricted in `where` clauses
     = note: 2 redundant requirements hidden
     = note: required for `Grouped<Eq<..., ...>>` to implement `AppearsOnTable<users::table>`
     = note: required for `WhereClause<Grouped<...>>` to implement `diesel::query_builder::where_clause::ValidWhereClause<FromClause<users::table>>`
     = note: required for `SelectStatement<FromClause<...>, ..., ..., ...>` to implement `Query`
     = note: required for `SelectStatement<FromClause<...>, ..., ..., ...>` to implement `LoadQuery<'_, _, (i32, std::string::String)>`
note: required by a bound in `diesel::RunQueryDsl::load`
    --> diesel-2.1.6/src/query_dsl/mod.rs:1542:15
     |
1540 | ...fn load<'query, U>(self, conn: &mut Con...
     |       ---- required by a bound in this associated function
1541 | ...where
1542 | ...    Self: LoadQuery<'query, Conn, 
...                  
     |              ^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `RunQueryDsl::load`
     = note: the full name for the type has been written to 'bad_select-fa50bb6fe8eee519.long-type-16986433487391717729.txt'
```

As we did in the previous section, we shall demo a short workflow using Argus to gather the same information.


