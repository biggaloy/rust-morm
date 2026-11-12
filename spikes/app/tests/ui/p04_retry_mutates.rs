// Proof 4: a retried closure that mutates a captured row doesn't compile.
use models_b::Customer;
use morm::Db;

async fn run(db: &Db) {
    let mut row = Customer { id: 1, name: "a".into() };
    let _ = db
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
