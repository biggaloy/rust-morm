// Proof 4, the evidence for the change: with an `AsyncFn` bound, a closure
// that only reads a captured row by reference isn't `Send` under
// `tokio::spawn`, so `.retry` would fail in every Axum handler that does it.
// `RetryBody` (`AsyncFnOnce + Clone`) accepts the same closure: see p04_tx.rs.
use models_b::Customer;
use morm::{Conn, Db};

fn main() {
    let db = Db::connect("host=127.0.0.1", 1).unwrap();
    let row = Customer { id: 1, name: "a".into() };
    tokio::spawn(async move {
        db.retry(3)
            .transaction_async_fn(async |tx| {
                tx.query("SELECT 1", &[&row.id]).await?;
                Ok(())
            })
            .await
    });
}
