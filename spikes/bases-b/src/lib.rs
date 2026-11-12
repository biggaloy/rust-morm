//! Proof 2: a base in a second crate, extending the first crate's base.

/// `Timestamps` plus who made the row, and a unique external id, so proof 8
/// can see a unique key that arrives through two macro expansions.
#[morm::base(extends = bases_a::stamps::Timestamps)]
pub struct Audit {
    pub created_by: i64,
    #[col(unique)]
    pub external_id: String,
}

/// Proof 3: global groups in a crate every model crate depends on, as
/// `models-a` does. Models there find them with `use bases_b::groups::G;`.
pub mod groups {
    morm::groups! { auditor }
}
