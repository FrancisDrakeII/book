## Working With More Than Two Futures

When we switched from using two futures to three in the previous section, we
also had to switch from using `join` to using `join3`. It would be annoying to
do this every time we changed our code. Happily, we have a macro form of `join`
to which we can pass an arbitrary number of arguments. It also handles awaiting
the futures itself. Thus, we could rewrite the code from Listing 17-12 to use
`join!` instead of `join3`, as in Listing 17-13:

<Listing number="17-13" caption="Using `join!` to wait for multiple futures" file-name="src/main.rs">

```rust
{{#rustdoc_include ../listings/ch17-async-await/listing-17-13/src/main.rs:here}}
```

</Listing>

This is definitely a nice improvement over needing to swap between `join` and
`join3` and `join4` and so on! However, even this macro form only works when we
know the number of futures ahead of time. In real-world Rust, though, pushing
futures into a collection and then waiting on some or all the futures in that
collection to complete is a very common pattern.

To check all the futures in some collection, we will need to iterate over and
join on *all* of them. The `trpl::join_all` function accepts any type which
implements the `Iterator` trait, which we learned about back in Chapter 13, so
it seems like just the ticket. Let’s try putting our futures in a vector, and
replace `join!` with `join_all`.

<Listing  number="17-14" caption="Storing anonymous futures in a vector and calling `join_all`">

```rust,ignore,does_not_compile
{{#rustdoc_include ../listings/ch17-async-await/listing-17-14/src/main.rs:here}}
```

</Listing>

Unfortunately, this does not compile. Instead, we get this error:

<!-- manual-regeneration
cd listings/ch17-async-await/listing-17-14/
cargo build
copy just the compiler error, and *add* the following text (correctly aligned),
to match the nicer version we will have starting in 1.81

```
   = note: no two async blocks, even if identical, have the same type
   = help: consider pinning your async block and and casting it to a trait object
```

Once 1.81 lands, we can remove the last part of that, since it will include the
message correctly automatically; but we  still need to do the rest of the manual
regeneration, unfortunately.

-->


<!--
TODO: delete this note once 1.81 is out and we update the version note at the
front of the book.
-->

> Note: Beta readers, the error version shown here is landing in Rust 1.81.0!
> If you are using an earlier version, you will see a *much* less helpful error
> message here. We fixed it as part of writing this chapter!

```text
error[E0308]: mismatched types
  --> src/main.rs:43:37
   |
8  |           let tx1_fut = async move {
   |  _______________________-
9  | |             let vals = vec![
10 | |                 String::from("hi"),
11 | |                 String::from("from"),
...  |
19 | |             }
20 | |         };
   | |_________- the expected `async` block
21 |
22 |           let rx_fut = async {
   |  ______________________-
23 | |             while let Some(value) = rx.recv().await {
24 | |                 println!("received '{value}'");
25 | |             }
26 | |         };
   | |_________- the found `async` block
...
43 |           let futures = vec![tx1_fut, rx_fut, tx_fut];
   |                                       ^^^^^^ expected `async` block, found a different `async` block
   |
   = note: expected `async` block `{async block@src/main.rs:8:23: 20:10}`
              found `async` block `{async block@src/main.rs:22:22: 26:10}`
   = note: no two async blocks, even if identical, have the same type
   = help: consider pinning your async block and and casting it to a trait object
```

This might be surprising. After all, none of them returns anything, so each
block produces a `Future<Output = ()>`. However, `Future` is a trait, not a
concrete type. The concrete types are the individual data structures generated
by the compiler for async blocks. You cannot put two different hand-written
structs in a `Vec`, and the same thing applies to the different structs
generated by the compiler.

To make this work, we need to use *trait objects*, just as we did in [“Returning
Errors from the run function”][dyn] in Chapter 12. (We will cover trait objects
in detail in Chapter 18.) Using trait objects lets us treat each of the
anonymous futures produced by these types as the same type, since all of them
implement the `Future` trait.

> Note: In Chapter 8, we discussed another way to include multiple types in a
> `Vec`: using an enum to represent each of the different types which can
> appear in the vector. We cannot do that here, though. For one thing, we have
> no way to name the different types, because they are anonymous. For another,
> the reason we reached for a vector and `join_all` in the first place was to be
> able to work with a dynamic collection of futures where we do not know what
> they will all be until runtime.

We start by wrapping each of the futures in the `vec!` in a `Box::new()`, as
shown in Listing 17-15.

<Listing number="17-15" caption="Trying to use `Box::new` to align the types of the futures in a `Vec`" file-name="src/main.rs">

```rust,ignore,does_not_compile
{{#rustdoc_include ../listings/ch17-async-await/listing-17-15/src/main.rs:here}}
```

</Listing>

Unfortunately, this still does not compile. In fact, we have the same basic
error we did before, but we get one for both the second and third `Box::new`
calls, and we also get new errors referring to the `Unpin` trait. We will come
back to the `Unpin` errors in a moment. First, let’s fix the type errors on the
`Box::new` calls, by explicitly providing the type of `futures` as a trait
object (Listing 17-16).

<Listing number="17-16" caption="Fixing the rest of the type mismatch errors by using an explicit type declaration" file-name="src/main.rs">

```rust,ignore,does_not_compile
{{#rustdoc_include ../listings/ch17-async-await/listing-17-16/src/main.rs:here}}
```

</Listing>

The type we had to write here is a little involved, so let’s walk through it:

* The innermost type is the future itself. We note explicitly that it the output
  of the future is the unit type `()` by writing `Future<Output = ()>`.
* Then we annotate the trait with `dyn` to mark it as dynamic.
* The entire trait is wrapped in a `Box`.
* Finally, we state explicitly that `futures` is a `Vec` containing these items.

That already made a big difference. Now when we run the compiler, we only have
the errors mentioning `Unpin`. Although there are three of them, notice that
each is very similar in its contents.

<!-- manual-regeneration
cd listings/ch17-async-await/listing-17-16
cargo build
copy *only* the errors
-->

```text
error[E0277]: `{async block@src/main.rs:8:23: 20:10}` cannot be unpinned
   --> src/main.rs:46:24
    |
46  |         trpl::join_all(futures).await;
    |         -------------- ^^^^^^^ the trait `Unpin` is not implemented for `{async block@src/main.rs:8:23: 20:10}`, which is required by `Box<{async block@src/main.rs:8:23: 20:10}>: std::future::Future`
    |         |
    |         required by a bound introduced by this call
    |
    = note: consider using the `pin!` macro
            consider using `Box::pin` if you need to access the pinned value outside of the current scope
    = note: required for `Box<{async block@src/main.rs:8:23: 20:10}>` to implement `std::future::Future`
note: required by a bound in `join_all`
   --> /Users/chris/.cargo/registry/src/index.crates.io-6f17d22bba15001f/futures-util-0.3.30/src/future/join_all.rs:105:14
    |
102 | pub fn join_all<I>(iter: I) -> JoinAll<I::Item>
    |        -------- required by a bound in this function
...
105 |     I::Item: Future,
    |              ^^^^^^ required by this bound in `join_all`

error[E0277]: `{async block@src/main.rs:8:23: 20:10}` cannot be unpinned
  --> src/main.rs:46:9
   |
46 |         trpl::join_all(futures).await;
   |         ^^^^^^^^^^^^^^^^^^^^^^^ the trait `Unpin` is not implemented for `{async block@src/main.rs:8:23: 20:10}`, which is required by `Box<{async block@src/main.rs:8:23: 20:10}>: std::future::Future`
   |
   = note: consider using the `pin!` macro
           consider using `Box::pin` if you need to access the pinned value outside of the current scope
   = note: required for `Box<{async block@src/main.rs:8:23: 20:10}>` to implement `std::future::Future`
note: required by a bound in `JoinAll`
  --> /Users/chris/.cargo/registry/src/index.crates.io-6f17d22bba15001f/futures-util-0.3.30/src/future/join_all.rs:29:8
   |
27 | pub struct JoinAll<F>
   |            ------- required by a bound in this struct
28 | where
29 |     F: Future,
   |        ^^^^^^ required by this bound in `JoinAll`

error[E0277]: `{async block@src/main.rs:8:23: 20:10}` cannot be unpinned
  --> src/main.rs:46:33
   |
46 |         trpl::join_all(futures).await;
   |                                 ^^^^^ the trait `Unpin` is not implemented for `{async block@src/main.rs:8:23: 20:10}`, which is required by `Box<{async block@src/main.rs:8:23: 20:10}>: std::future::Future`
   |
   = note: consider using the `pin!` macro
           consider using `Box::pin` if you need to access the pinned value outside of the current scope
   = note: required for `Box<{async block@src/main.rs:8:23: 20:10}>` to implement `std::future::Future`
note: required by a bound in `JoinAll`
  --> /Users/chris/.cargo/registry/src/index.crates.io-6f17d22bba15001f/futures-util-0.3.30/src/future/join_all.rs:29:8
   |
27 | pub struct JoinAll<F>
   |            ------- required by a bound in this struct
28 | where
29 |     F: Future,
   |        ^^^^^^ required by this bound in `JoinAll`

Some errors have detailed explanations: E0277, E0308.
For more information about an error, try `rustc --explain E0277`.
```

That is a *lot* to digest, so let’s pull it apart. The first part of the message
tell us that the first async block (`src/main.rs:8:23: 20:10`) does not
implement the `Unpin` trait, and suggests using `pin!` or `Box::pin` to resolve
it. The rest of the message tells us *why* that is required: the `JoinAll`
struct returned by `trpl::join_all` is generic over a type `F` which must
implement the `Future` trait, directly awaiting a Future requires that the
future implement the `Unpin` trait. Understanding this error means we need to
dive into a little more of how the `Future` type actually works, in particular
the idea of *pinning*.

### Pinning and the Pin and Unpin Traits

<!-- TODO: get a *very* careful technical review of this section! -->

Let’s look again at the definition of `Future`, focusing now on its `poll`
method’s `self` type:

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Future {
    type Output;

    // Required method
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

This is the first time we have seen a method where `self` has a type annotation
like this. When we specify the type of `self` like this, we are telling Rust
what type `self` must be to call this method. These kinds of type annotations
for `self` are similar to those for other function parameters, but with the
restriction that the type annotation has to be the type on which the method is
implemented, or a reference or smart pointer to that type. We will see more on
this syntax in Chapter 18. For now, it is enough to know that if we want to poll
a future (to check whether it is `Pending` or `Ready(Output)`), we need a
mutable reference to the type, which is wrapped in a `Pin`.

`Pin` is a smart pointer, much like `Box`, `Rc`, and the others we saw in
Chapter 15. Unlike those, however, `Pin` only works with *other pointer types*
like reference (`&` and `&mut`) and smart pointers (`Box`, `Rc`, and so on). To
be precise, `Pin` works with types which implement the `Deref` or `DerefMut`
traits, which we covered in Chapter 15. You can think of this restriction as
equivalent to only working with pointers, though, since implementing `Deref` or
`DerefMut` means your type behaves like a pointer type.

Recalling that `.await` is implemented in terms of calls to `poll()`, this
starts to explain the error message we saw above—but that was in terms of
`Unpin`, not `Pin`. So what exactly are `Pin` and `Unpin`, how do they relate,
and why does `Future` need `self` to be in a `Pin` type to call `poll`?

In [“What Are Futures”][what-are-futures], we described how a series of await
points in a future get compiled into a state machine—and noted how the compiler
helps make sure that state machine follows all of Rust’s normal rules around
safety, including borrowing and ownership. To make that work, Rust looks at what
data is needed between each await point and the next await point or the end of
the async block. It then creates a corresponding variant in the state machine it
creates. Each variant gets the access it needs to the data that will be used in
that section of the source code, whether by taking ownership of that data or by
getting a mutable or immutable reference to it.

So far so good: if we get anything wrong about the ownership or references in a
given async block, the borrow checker will tell us. When we want to move around
the future that corresponds to that block—like moving it into a `Vec` to pass to
`join_all`—things get trickier.

When we move a future—whether by pushing into a data structure to use as an
iterator with `join_all`, or returning them from a function—that actually means
moving the state machine Rust creates for us. And unlike most other types in
Rust, the futures Rust creates for async blocks can end up with references to
themselves in the fields of any given variant. Any object which has a reference
to itself is unsafe to move, though, because references always point to the
actual memory address of the thing they refer to. If you move the data structure
itself, you *have* to update any references to it, or they will be left pointing
to the old location.

In principle, you could make the Rust compiler try to update every reference to
an object every time it gets moved. That would  potentially be a lot of
performance overhead, especially given there can be a whole web of references
that need updating. On the other hand, if we could make sure the data structure
in question *does not move in memory*, we do not have to update any references.
And this is exactly what Rust’s borrow checker already guarantees: you cannot
move an item which has any active references to it using safe code.

`Pin` builds on that to give us the exact guarantee we need. When we *pin* a
value by wrapping a pointer to it in `Pin`, it can no longer move. Thus, if you
have `Pin<Box<SomeType>>`, you actually pin the `SomeType` value, *not* the
`Box` pointer. In fact, the pinned box pointer can move around freely. Remember:
we care about making sure the data ultimately being referenced stays in its
place. If a pointer moves around, but the data it points to is in the same
place, there is no problem.

However, most types are perfectly safe to move around, even if they happen to be
behind a `Pin` pointer. We only need to think about pinning when items have
internal references. Primitive values like numbers and booleans do not have any
internal structure like that, so they are obviously safe. Neither do most types
you normally work with in Rust. A `Vec`, for example, does not have any internal
references it needs to keep up to date this way, so you can move it around
without worrying. If you have a `Pin<Vec<String>>`, you would have to do
everything via Pin’s safe but restrictive APIs, even though a `Vec<String>` is
always safe to move if there are no other references to it. We need a way to
tell the compiler that it is actually just fine to move items around in cases
like these. For that, we have `Unpin`.

`Unpin` is a marker trait, like `Send` and `Sync`, which we saw in Chapter 16.
Recall that marker traits have no functionality of their own. They exist only to
tell the compiler that it is safe to use the type which implements a given trait
in a particular context. `Unpin` informs the compiler that a given type does
*not* need to uphold any particular guarantees about whether the value in
question can be moved.

Just like `Send` and `Sync`, the compiler implements `Unpin` automatically for
all types where it can prove it is safe. Implementing `Unpin` manually is unsafe
because it requires *you* to uphold all the guarantees which make `Pin` and
`Unpin` safe yourself for a type with internal references. In practice, this is
a very rare thing to implement yourself!

> Note: This combination of `Pin` and `Unpin` allows a whole class of complex
> types to be safe in Rust which are otherwise difficult to implement because
> they are self-referential. Types which require `Pin` show up *most* commonly
> in async Rust today, but you might—very rarely!—see it in other contexts, too.
>
> The specific mechanics for how `Pin` and `Unpin` work under the hood are
> covered extensively in the API documentation for `std::pin`, so if you would
> like to understand them more deeply, that is a great place to start.

Now we know enough to fix the last errors with `join_all`. We tried to move the
futures produced by an async blocks into a `Vec<Box<dyn Future<Output = ()>>>`,
but as we have seen, those futures may have internal references, so they do not
implement `Unpin`. They need to be pinned, and then we can pass the `Pin` type
into the `Vec`, confident that the underlying data in the futures will *not* be
moved.

Listing 17-17 shows how we put this all into practice. First, we update the type
annotation for `futures`, with a `Pin` wrapping each `Box`. Second, we use
`Box::pin` to pin the futures themselves.

<Listing number="17-17" caption="Using `Pin` and `Box::pin` to make the `Vec` type check" file-name="src/main.rs">

```rust
{{#rustdoc_include ../listings/ch17-async-await/listing-17-17/src/main.rs:here}}
```

</Listing>

If we compile and run this, we finally get the output we hoped for:

<!-- Not extracting output because changes to this output aren't significant;
the changes are likely to be due to the threads running differently rather than
changes in the compiler -->

```text
received 'hi'
received 'more'
received 'from'
received 'messages'
received 'the'
received 'for'
received 'future'
received 'you'
```

Phew!

There is a bit more we can explore here. For one thing, using `Pin<Box<T>>`
comes with a small amount of extra overhead from putting these futures on the
heap with `Box`—and we are only doing that to get the types to line up. We don’t
actually *need* the heap allocation, after all: these futures are local to this
particular function. As noted above, `Pin` is itself a smart pointer, so we can
get the benefit of having a single type in the `Vec`—the original reason we
reached for `Box`—without doing a heap allocation. We can use `Pin` directly
with each future, using the `std::pin::pin` macro.

However, we must still be explicit about the type of the pinned reference;
otherwise Rust will still not know to interpret these as dynamic trait objects,
which is what we need them to be in the `Vec`. We therefore `pin!` each future
when we define it, and define `futures` as a `Vec` containing pinned mutable
references to the dynamic `Future` type, as in Listing 17-18.

<Listing number="17-18" caption="Using `Pin` directly with the `pin!` macro to avoid unnecessary heap allocations" file-name="src/main.rs">

```rust
{{#rustdoc_include ../listings/ch17-async-await/listing-17-18/src/main.rs:here}}
```

</Listing>

There is another, more serious, issue as well. We got this far by ignoring the
fact that we might have different `Output` types. For example, in Listing 17-19,
the anonymous future for `a` implements `Future<Output = u32>`, the anonymous
future for `b` implements `Future<Output = &str>`, and the anonymous future for
`c` implements `Future<Output = bool>`.

<Listing number="17-19" caption="Three futures with distinct types" file-name="src/main.rs">

```rust
{{#rustdoc_include ../listings/ch17-async-await/listing-17-19/src/main.rs:here}}
```

</Listing>

We can use `trpl::join!` to await them, because it allows you to pass in
multiple future types and produces a tuple of those types. We *cannot* use
`trpl::join_all`, because it requires the futures passed in all to have the same
type. (Remember, that error is what got us started on this adventure with
`Pin`!)

This is a fundamental tradeoff: we can either deal with a dynamic number of
futures with `join_all`, as long as they all have the same type, or we can deal
with a set number of futures with the `join` functions or the `join!` macro,
even if they have different types. This is the same as working with any other
types in Rust, though. Futures are not special, even though we have some nice
syntax for working with them, and that is a good thing.

In practice, you will usually work directly with `async` and `.await`, and
secondarily with functions and macros like `join` or `join_all`. You will only
need to reach for `pin` now and again to use them with those APIs. `Pin` and
`Unpin` are mostly important for building lower-level libraries, or when you are
building a runtime itself, rather than for day to day Rust code. When you see
them, though, now you will know what to do!

[collections]: ch08-01-vectors.html#using-an-enum-to-store-multiple-types
[dyn]: ch12-03-improving-error-handling-and-modularity.html
[what-are-futures]: ch17-01-futures-and-syntax.html#what-are-futures