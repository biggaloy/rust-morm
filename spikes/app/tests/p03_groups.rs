//! Proof 3: masks the macros size, used through generic code on stable.
//! A mask or group for another model is a compile error: see `ui/`.

use app::{Product, Thing, big, groups};
use morm::{Bits, Db, GlobalGroups, Mask, MaskBits, Model};
use std::mem::size_of;

#[test]
fn the_macros_count_the_mask_size() {
    assert_eq!(groups::COUNT, 4);
    assert_eq!(size_of::<groups::Mask>(), 8);
    assert_eq!(big::COUNT, 65);
    assert_eq!(size_of::<big::Mask>(), 16);
    let _: Bits<2> = <big::Mask as MaskBits>::EMPTY.0;
    // Scoped masks are counted per model; a model with none has `()`.
    assert_eq!(size_of::<<Product as Model>::Scoped>(), 8);
    assert_eq!(size_of::<<models_b::Customer as Model>::Scoped>(), 0);
    assert_eq!(size_of::<Mask<Product>>(), 16);
    assert_eq!(size_of::<Mask<models_b::Customer>>(), 0);
    // Bit 64 lands in the second word.
    let mut m = <big::Mask as MaskBits>::EMPTY;
    m.set(big::G.g64.bit());
    assert!(m.has(64) && !m.has(0));
    assert_eq!(m.0.0, [0, 1]);
}

/// One generic function serves every model, whatever its mask sizes.
fn visible<M: Model>(db: &Db, mask: &Mask<M>) -> Vec<&'static str> {
    db.groups(mask).columns().collect()
}

#[tokio::test]
async fn db_groups_is_generic_over_the_mask() {
    // Never connects: building a pool opens nothing.
    let db = Db::connect("host=127.0.0.1 user=nobody", 1).unwrap();
    let none = Mask::<Product>::new();
    assert_eq!(visible(&db, &none), ["id", "name", "photo", "tags"]);
    let admin = none.with(groups::G.admin);
    assert_eq!(
        visible(&db, &admin),
        ["id", "name", "cost", "photo", "tags"]
    );
    let editor = none.with_scoped(Product::G.editor);
    assert_eq!(
        visible(&db, &editor),
        ["id", "name", "supplier", "photo", "tags"]
    );
    let sudo = none.with(groups::G.sudo);
    assert_eq!(visible(&db, &sudo), Product::COLUMNS);
    let owner = Mask::<Thing>::new().with_scoped(Thing::G.owner);
    assert_eq!(visible(&db, &owner), ["id", "secret"]);
    assert_eq!(
        visible(&db, &Mask::<models_b::Customer>::new()),
        ["id", "name"]
    );
}

/// `groups!` emits `G` as a type and a constant, so a model in another crate
/// that only does `use bases_b::groups::G;` gets `type Global` from the type
/// and each group's bit from the constant. A model naming no global group
/// gets `()`.
#[tokio::test]
async fn models_find_global_groups_through_a_use() {
    fn same<T>(_: T, _: T) {}
    same(
        <<models_a::Order as Model>::Global as MaskBits>::EMPTY,
        <bases_b::groups::Mask as MaskBits>::EMPTY,
    );
    same(
        <<Product as Model>::Global as MaskBits>::EMPTY,
        <<groups::G as GlobalGroups>::Mask as MaskBits>::EMPTY,
    );
    let () = <<models_b::Customer as Model>::Global as MaskBits>::EMPTY;
    let db = Db::connect("host=127.0.0.1 user=nobody", 1).unwrap();
    let none = Mask::<models_a::Order>::new();
    assert!(!visible(&db, &none).contains(&"note"));
    let auditor = none.with(bases_b::groups::G.auditor);
    assert!(visible(&db, &auditor).contains(&"note"));
}
