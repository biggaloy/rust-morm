// Proof 3: a mask for another model, or another model's group, doesn't compile.
use app::{Product, Thing, big};
use morm::{Db, Grouped, Mask};

fn product_columns(_: Grouped<'_, Product>) {}

fn main() {
    let db = Db::connect("host=127.0.0.1", 1).unwrap();
    let thing = Mask::<Thing>::new();
    product_columns(db.groups(&thing));
    let _ = Mask::<Product>::new().with_scoped(Thing::G.owner);
    let _ = Mask::<Product>::new().with(big::G.g01);
}
