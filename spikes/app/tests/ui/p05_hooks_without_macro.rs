// Proof 5: an `impl Hooks` written without `#[morm::hooks]` doesn't compile,
// so a hook can't be skipped because its override went unrecorded.
#[morm::model(table = "t", hooks)]
pub struct Plain {
    pub id: i64,
}

impl morm::Hooks for Plain {
    async fn pre_insert(_: &mut [Self], _: &impl morm::Conn) -> morm::Result<()> {
        Ok(())
    }
}

fn main() {}
