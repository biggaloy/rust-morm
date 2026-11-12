//! A second crate of models, for proof 10.

#[morm::model(table = "spike_customer")]
pub struct Customer {
    pub id: i64,
    pub name: String,
}
