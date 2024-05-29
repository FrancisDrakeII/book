## Async and Await

In Chapter 16, we saw one of Rust’s approaches to concurrency: using threads.
Since Rust 1.39, there has been another option for concurrency: asynchronous
programming, or *async*.

In the rest of this chapter, we will:

* see how to use Rust’s `async` and `.await` syntax
* explore how to use the async model to solve some of the same challenges we
  looked at in Chapter 16
* look at how multithreading and async provide complementary solutions, which
  you can even use together in many cases

First, though, let’s explore what async gives us.

### Why Async?

Many operations we ask the computer to do can take a while to finish. For
example, if you used a video editor to create a video of a family celebration,
exporting it could take anywhere from minutes to hours. Similarly, when you
download a video shared by someone in your family, that download process might
take a long time. It would be nice if we could do something else while we are
waiting for those long-running processes to complete.

The video export will use as much CPU and GPU power as it can. If you only had
one CPU core, and your operating system never paused that export until it
completed, you could not do anything else on your computer while it was running.
That would be a pretty frustrating experience, though, so instead your computer
can (and does!) invisibly interrupt the export often enough to let you get other
small amounts of work done along the way.

The file download is different. It does not take up very much CPU time. Instead,
you are mostly waiting on data to transfer across the network. You can start
reading from a network socket, but it might take a while for all the data to
arrive and be fed into the socket by the network controller. Moreover, even once
the data has all arrived, videos can be quite large, so it might take some time
to load all the data from the socket. Even if “some time” here is just a second
or two, that is a very long time for a modern processor, which can do billions
of operations every second. You could choose to wait for all of that to finish,
but you might be waiting for a while… with your CPU doing not much! Thus, even
if your specific program cannot do anything until it finishes reading data from
a network socket, your computer will once again invisibly interrupt your
program so other things can happen at the same time as the network operation.

> Note: The video export is the kind of operation which is often described as
> “CPU-bound”. It is limited by the speed of the computer’s CPU (and GPU), and
> how much of that power it can use. The video download is the kind of operation
> which is often described as “IO-bound,” because it is limited by the speed of
> the computer’s *input and output*. It can only go as fast as the data can be
> sent across the network, which means that it can only go as fast as the data
> can be written to the socket by the network controller.

In both of these examples, the concurrency happens at the level of the whole
program. The operating system decides to interrupt the program to let other
programs get work done. In many cases, since we understand our programs at a
much more granular level than the operating system does, we can lots of
opportunities for concurrency that the operating system cannot see. For example,
if we are building a tool to manage file downloads, it is important that the
user interface stay responsive while a download is happening. In fact, we should
even be able to start multiple downloads at the same time.

However, many operating system APIs for interacting with network sockets are
*blocking*. That is, the function calls block further progress in the program
when they are called until they return. This is how *most* function calls work,
if you think about it! However, we normally reserve the term “blocking” for
function calls which interact with files, network sockets, or other resources on
the computer, because those are the places where an individual program would
benefit from the operation being *non*-blocking.

When doing file downloads, we could work around the fact that the call to write to
a network socket is blocking using threads. If we move the data over to a
dedicated thread which handles the write operation, it will *not* block the rest
of the program. But in many ways, it would be nicer if the call were not
blocking in the first place.

One way to accomplish that would be to use an API built around callbacks. For
each blocking operation, we could pass in a function to call once the operation
completes:

```rust,ignore
network_socket.read_non_blocking(|result| {
    // ...
});
```

Or we could register callbacks to run when events happen:

```rust,ignore
network_socket.add_listener(Event::ReadFinished, |data| {
    // ...
});
```

Or we could have our functions return a type with `and_then` method, which in
turn accepts a callback which can do more work of the same sort:

```rust,ignore
network_socket.read_non_blocking().and_then(|result| {
    /* another non_blocking operation */
});
```

Historically, this last choice was the way that Rust did async! Each of these
can make the control flow for the program more complicated, though. You can end
up with many nested callbacks, or long chains of callbacks, and understanding
the flow of data through the program can become more difficult as a result.

With other common types in Rust, we often use pattern-matching in scenarios like
this. When we are using callbacks we do not yet have the data at the time we
call `read_non_blocking`—and we will not have it until the callback gets called.
That means that there is no way to match on the data it will return: it is not
here yet!

There are also no particularly good ways to get data out of those callbacks. We
might try something like this, imagining a `read_non_blocking` which has exactly
the kind of `and_then` method described above. If we were to try to use that,
with code something like this, it would not compile:

```rust,ignore,does_not_compile
let mut data = None;
network_socket.read_non_blocking().and_then(|result| {
    data = Some(result);
});
println!("{data:?}");
```

The callback passed to `and_then` needs a mutable reference to `data`, but the
`load` function tries to return `data` to the caller. Rust would helpfully tell
us that we cannot borrow `data` immutably to print it because it is still
borrowed mutably for the `and_then` callback. This is not just Rust being fussy,
either: the result of this would normally always just print the `None` value and
exit, but if the read *happened* to go fast enough, it is possible it could
sometimes print some string data instead. That is *definitely* not what we
want!

We also cannot cancel `read_non_blocking`: once it has started, it will run till
it finishes unless the whole program stops.

What we really want to be able to write is something much more direct, like we
would write in blocking code, but with the benefits of getting the data when it
is available and *not* blocking the rest of the program while waiting for the
data to arrive—something like this:

```rust,ignore,does_not_compile
let data = network_socket.read(&path).await;
println!("{data}");
```

That is exactly what Rust’s async abstraction gives us. It is designed to help
us solve all of these issues. In the next section, we will see how this works in
practice.