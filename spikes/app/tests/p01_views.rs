//! Proof 1: `#[view]` makes sibling structs from the model's one field list.
//! The compile errors (hidden field, wrong view's column) are in `ui/`.

use app::{Product, ProductCard, ProductPublic};
use morm::{Column, Model};

#[test]
fn views_hold_only_their_fields() {
    let p = ProductPublic {
        id: 1,
        name: "lamp".into(),
        photo: None,
        tags: Some("home".into()),
    };
    assert_eq!(p.name, "lamp");
    assert_eq!(ProductPublic::COLUMNS, ["id", "name", "photo", "tags"]);
    assert_eq!(ProductCard::COLUMNS, ["id", "name"]);
    assert_eq!(
        Product::COLUMNS,
        ["id", "name", "cost", "supplier", "photo", "tags"]
    );
    assert_eq!(
        ProductPublic::SELECT,
        r#"SELECT "id", "name", "photo", "tags" FROM "spike_product""#
    );
    assert_eq!(
        ProductCard::SELECT,
        r#"SELECT "id", "name" FROM "spike_product""#
    );
}

/// Each view's `F` is its own type, and its columns are typed by the view.
#[test]
fn f_differs_per_view() {
    fn public_col<T, C>(c: Column<ProductPublic, T, C>) -> &'static str {
        c.name()
    }
    fn model_col<T, C>(c: Column<Product, T, C>) -> &'static str {
        c.name()
    }
    assert_eq!(public_col(ProductPublic::F.name), "name");
    assert_eq!(model_col(Product::F.cost), "cost");
    assert_ne!(
        std::any::type_name_of_val(&ProductPublic::F),
        std::any::type_name_of_val(&Product::F)
    );
    assert_ne!(
        std::any::type_name_of_val(&ProductPublic::F.name),
        std::any::type_name_of_val(&Product::F.name)
    );
}
