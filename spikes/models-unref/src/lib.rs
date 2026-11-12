//! A crate of models that no code references, for proof 10. Only `mgr`
//! names it, in its dependencies and with `use models_unref as _;`.

#[morm::model(table = "spike_widget")]
pub struct Widget {
    pub id: i64,
    pub label: String,
}
