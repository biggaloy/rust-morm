// Proof 1: a view's column isn't the model's column.
use app::{Product, ProductPublic};
use morm::Column;

fn model_only<T, C>(_: Column<Product, T, C>) {}

fn main() {
    model_only(ProductPublic::F.name);
}
