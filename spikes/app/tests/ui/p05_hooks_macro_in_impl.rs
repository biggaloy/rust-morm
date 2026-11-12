// Proof 5: `#[morm::hooks]` sees the impl before macros inside it expand, so
// a hook written by a macro would go unrecorded and never run. The attribute
// refuses any macro call among the impl's items.
#[morm::model(table = "t", hooks)]
pub struct Generated {
    pub id: i64,
}

#[allow(unused_macros)]
macro_rules! my_hooks {
    () => {
        async fn pre_insert(_: &mut [Self], _: &impl morm::Conn) -> morm::Result<()> {
            Ok(())
        }
    };
}

#[morm::hooks]
impl morm::Hooks for Generated {
    my_hooks!();
}

fn main() {}
