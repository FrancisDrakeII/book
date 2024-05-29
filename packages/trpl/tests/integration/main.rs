//! Integration tests for the crate.
//!
//! These all live in a *single* integration test crate, `tests/integration`,
//! because each integration test is a dedicated binary crate which has to be
//! compiled separately. While that is not really a problem for a crate this
//! small, we have chosen to follow this “best practice” here as a good example.
//!
//! For more details on why you might prefer this pattern see [this post][post].
//!
//! [post]: https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html

use std::{pin::Pin, time::Duration};

use futures::Future;
use trpl::{Receiver, Sender};

/// This test is foundational for all the others, as they depend on `block_on`.
///
/// If we mess this up, *all* the tests below will fail -- so by the same token,
/// if all the tests below are failing, this one probably is too; fix it and the
/// others will likely start working again.
#[test]
fn re_exported_block_on_works() {
    let val = trpl::block_on(async { "Hello" });
    assert_eq!(val, "Hello");
}

#[test]
fn re_exported_spawn_works() {
    let result = trpl::block_on(async {
        let handle_a = trpl::spawn_task(async { "Hello" });
        let handle_b = trpl::spawn_task(async { "Goodbye" });
        vec![handle_a.await.unwrap(), handle_b.await.unwrap()]
    });

    assert_eq!(result, vec!["Hello", "Goodbye"]);
}

#[test]
fn re_exported_sleep_works() {
    let val = trpl::block_on(async {
        trpl::sleep(Duration::from_micros(1)).await;
        "Done!"
    });
    assert_eq!(val, "Done!");
}

#[test]
fn re_exported_channel_apis_work() {
    trpl::block_on(async {
        // Explicitly naming the type to confirm the re-exports are aligned.
        let (tx, mut rx): (Sender<&str>, Receiver<&str>) = trpl::channel();

        tx.send("Hello").unwrap();
        trpl::sleep(Duration::from_millis(1)).await;
        tx.send("Goodbye").unwrap();
        drop(tx);

        assert_eq!(rx.recv().await, Some("Hello"));
        assert_eq!(rx.recv().await, Some("Goodbye"));
        assert_eq!(rx.recv().await, None);
    });
}

mod re_exported_join_apis_work {
    use super::*;

    #[test]
    fn join_fn() {
        let result = trpl::block_on(async {
            let a = async { 1 };
            let b = async { 2 };
            trpl::join(a, b).await
        });

        assert_eq!(result, (1, 2));
    }

    #[test]
    fn join3_fn() {
        let result = trpl::block_on(async {
            let a = async { 1 };
            let b = async { 2 };
            let c = async { 3 };

            trpl::join3(a, b, c).await
        });

        assert_eq!(result, (1, 2, 3));
    }

    #[test]
    fn join_all_fn() {
        let result = trpl::block_on(async {
            let a = async { format!("{}", 1) };

            let b = async { format!("Hello") };

            let outer = String::from("World");
            let c = async move { format!("{outer}") };

            let futures: Vec<Pin<Box<dyn Future<Output = String>>>> =
                vec![Box::pin(a), Box::pin(b), Box::pin(c)];

            trpl::join_all(futures).await
        });

        assert_eq!(
            result,
            vec![
                String::from("1"),
                String::from("Hello"),
                String::from("World")
            ]
        );
    }

    #[test]
    fn join_macro() {
        let result = trpl::block_on(async {
            let a = async { 1 };
            let b = async { "Hello" };

            let outer = vec![String::from("World")];
            let c = async move { outer };

            trpl::join!(a, b, c)
        });

        assert_eq!(result, (1, "Hello", vec![String::from("World")]));
    }
}