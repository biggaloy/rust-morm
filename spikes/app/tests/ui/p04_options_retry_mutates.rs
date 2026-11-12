// Proof 4: with every option before the closure, a retried closure that
// mutates a captured row still doesn't compile, whatever the order.
use models_b::Customer;
use morm::Db;
use morm::Isolation::Serializable;
use std::time::Duration;

async fn run(db: &Db) {
    let mut row = Customer { id: 1, name: "a".into() };
    let _ = db
        .isolation(Serializable)
        .statement_timeout(Duration::from_secs(1))
        .retry(3)
        .transaction(async |_tx| {
            row.name.push('!');
            Ok(())
        })
        .await;
}

fn main() {
    let _ = run;
}
