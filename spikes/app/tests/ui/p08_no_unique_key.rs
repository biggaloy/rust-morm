// Proof 8: a column with no unique key can't be a conflict target.
use models_a::Order;

fn main() {
    let _ = morm::upsert_sql::<Order, _>(Order::F.code, &["note"]);
}
