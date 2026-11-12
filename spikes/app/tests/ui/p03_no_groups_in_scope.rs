// Proof 3: a model naming a global group finds the app's `G` through a plain
// `use`. Without one, the error points at the missing `G`.
#[morm::model(table = "t")]
pub struct Secretive {
    pub id: i64,
    #[col(group = admin)]
    pub secret: String,
}

fn main() {}
