# Trait Methods and Typestate

Every programming language cultivates its own set of patterns. One pattern common in Rust is the *builder pattern.* Some data structures are complicated to construct, they may require a large number of inputs, or have complex configuration; the builder pattern helps construct complex values.

A great example of working with builders is the Diesel [`QueryDsl`](https://docs.rs/diesel/latest/diesel/prelude/trait.QueryDsl.html). The `QueryDsl` trait exposes a number of methods to construct a valid SQL query. Each method consumes the caller, and returns a type that itself implements `QueryDsl`. As an example here's the method signature for `select`

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

As we did in the previous section, we shall demo a short workflow using Argus to gather the same information. Opening the Argus panel works a little differently, as you shall see in the following video. When there isn't a link to the obligation in the error tooltip, you can always open Argus from the Command Palette or the bottom toolbar.

<video controls>
  <source alt="Opening the Argus panel" src="assets/diesel-bad-select/open-argus.mp4" type="video/mp4" />
</video>

Below we see that Argus presents *more* failing bounds than the compiler did. To debug effectively with Argus you should consider all failing founds in the context of your problem and start with the one that is most relevant. You can also compare the failing bound with information provided in the Rust error diagnostic to get you on the right track.

```admonish important
**Argus may present more errors than the Rust compiler,** it is research software after all. Use your judgement to decide which errors are first worth exploring, if there are multiple, look at all of them before diving down into one specific search tree. We're working hard to reduce noise produced by Argus as much as possible.
```

<video controls>
  <source alt="Failing bounds in Argus" src="assets/diesel-bad-select/failing-bounds.mp4" type="video/mp4" />
</video>

Now let's dive into the trait error.

<video controls>
  <source alt="Diesel finding bug" src="assets/diesel-bad-select/find-bug.mp4" type="video/mp4" />
</video>

Here are some key points from above that we'd like to highlight

1. When opening the Argus debugger the hover tooltip said "Expression contains unsatisfied trait bounds," but  there wasn't a link to jump to the error. This is an unfortunate circumstance, but one that does occur. In these cases you can open the Argus panel by clicking the Argus status in the bottom information bar, or run the command 'Argus: Inspect current file' in the command palette.

2. The printed types in Rust can get painfully verbose, the Rust diagnostic even *wrote types to a file* because they were too long. Argus shortens and condenses type information to keep the panel as readable as possible. One example of this is that fully-qualified identifiers, like `users::columns::id` prints shortened as `id`. On hover, the full path is shown at the bottom of the Argus panel in our mini-buffer. Extra information or notes Argus has for you are printed in the mini-buffer, so keep an eye on that if you feel Argus isn't giving you enough information.

3. Clicking the tree icon next to a node in the Bottom-Up view jumps to that same node in the Top-Down view. This operation is useful if you want to gather contextual information around a node, but don't want to search the Top-Down tree for it. You can get there in one click.

Turns out we forgot to join the `users` and `posts` tables! At this point we understand and have identified the error, now it's time to fix the program. Unfortunately Argus provides no aide to *fix* typestate errors. We're in the wrong state, `posts::id` doesn't appear in the table we're selecting from, we need to get it on the selected-from table. This is a great time to reach for the Diesel documentation for [`QueryDsl`](https://docs.rs/diesel/latest/diesel/prelude/trait.QueryDsl.html).

<video controls>
  <source alt="Diesel fixing typestate error" src="assets/diesel-bad-select/fixed-error.mp4" type="video/mp4" />
</video>

Here we used our domain knowledge of SQL to find the appropriate join methods. We decided to use an `inner_join` to join the tables, and then all was fixed.

```admonish note
Finding the appropriate method to change the typestate won't always be so straightforward. If you lack domain knowledge or are unfamiliar with the terms used in the library, you may have to read more of the documentation and look through examples to find appropriate fixes. When in doubt, try something! And use Argus to continue debugging.
```
