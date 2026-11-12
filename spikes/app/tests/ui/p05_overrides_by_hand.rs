// Proof 5: `OVERRIDES` can't be written by hand. `Overrides` has no public
// constructor or constant, another model's value has another model's type,
// and `#[morm::hooks]` refuses one written inside the impl.
#[morm::model(table = "t", hooks)]
pub struct Silent {
    pub id: i64,
}

impl morm::Hooks for Silent {
    const OVERRIDES: morm::Overrides<Self> = morm::Overrides::NONE;
    async fn pre_insert(_: &mut [Self], _: &impl morm::Conn) -> morm::Result<()> {
        Ok(())
    }
}

#[morm::model(table = "t2", hooks)]
pub struct Borrowed {
    pub id: i64,
}

impl morm::Hooks for Borrowed {
    const OVERRIDES: morm::Overrides<Self> = <app::Note as morm::Hooks>::OVERRIDES;
}

#[morm::model(table = "t3", hooks)]
pub struct Written {
    pub id: i64,
}

#[morm::hooks]
impl morm::Hooks for Written {
    const OVERRIDES: morm::Overrides<Self> = <Self as morm::Hooks>::OVERRIDES;
}

fn main() {}
