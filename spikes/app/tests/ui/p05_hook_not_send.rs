// Proof 5: a hook whose future isn't `Send` fails at the hook itself, not
// deep inside the web framework that spawns it.
use std::rc::Rc;

#[morm::model(table = "t", hooks)]
pub struct Counted {
    pub id: i64,
}

#[morm::hooks]
impl morm::Hooks for Counted {
    async fn pre_insert(_: &mut [Self], db: &impl morm::Conn) -> morm::Result<()> {
        let local = Rc::new(1);
        db.query("SELECT 1", &[]).await?;
        drop(local);
        Ok(())
    }
}

fn main() {}
