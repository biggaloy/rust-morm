// Proof 1: a field a view hides is absent from the view and from its `F`.
use app::ProductPublic;

fn main() {
    let _ = ProductPublic::F.cost;
    let p = ProductPublic { id: 1, name: String::new(), photo: None, tags: None };
    let _ = p.supplier;
}
