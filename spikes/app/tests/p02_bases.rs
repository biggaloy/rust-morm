//! Proof 2: `#[morm::base]` through a generated `macro_rules!`, across crates,
//! a base extending a base. The fields land as plain fields.

use app::Doc;
use models_a::Order;
use morm::Model;

/// `Order` takes `bases_b::Audit`, which extends `bases_a::Timestamps`, from a
/// crate that doesn't depend on `bases-a`. Building it with a struct literal
/// proves every field is a plain field of `Order`, with no nesting.
#[test]
fn fields_from_two_bases_in_two_crates_are_plain() {
    let o = Order {
        created_at: 1,
        updated_at: 2,
        created_by: 3,
        external_id: "x".into(),
        id: 4,
        tenant: 5,
        code: "c".into(),
        email: "e".into(),
        deleted: false,
        note: None,
    };
    assert_eq!((o.created_at, o.updated_at, o.created_by), (1, 2, 3));
    assert_eq!(
        Order::COLUMNS,
        [
            "created_at",
            "updated_at",
            "created_by",
            "external_id",
            "id",
            "tenant",
            "code",
            "email",
            "deleted",
            "note"
        ]
    );
}

/// A base used in its own crate, through a module path, extending another
/// base of that crate.
#[test]
fn bases_work_inside_their_own_crate() {
    let d = Doc {
        owner: 1,
        flag: true,
        id: 2,
    };
    assert!(d.flag && d.owner == 1 && d.id == 2);
    assert_eq!(Doc::COLUMNS, ["owner", "flag", "id"]);
}

#[test]
fn two_bases_may_share_a_name() {
    assert_eq!(app::LeftTagged::COLUMNS, ["left", "id"]);
    assert_eq!(app::RightTagged::COLUMNS, ["right", "id"]);
}
