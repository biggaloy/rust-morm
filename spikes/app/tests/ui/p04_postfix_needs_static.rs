// Proof 4, the evidence for the call shape: a postfix `.retry(n)` needs a
// boxed future, and that forces captures to be `'static`, so a closure that
// reads a local row by reference doesn't compile.
use models_b::Customer;
use morm::{Conn, Db, Tx};

async fn run(db: &Db) {
    let row = Customer {
        id: 1,
        name: "a".into(),
    };
    let _ = db
        .transaction_postfix(async |tx: &Tx| {
            tx.query("SELECT 1", &[&row.id]).await?;
            Ok(())
        })
        .retry(3)
        .await;
}

fn main() {
    let _ = run;
}
