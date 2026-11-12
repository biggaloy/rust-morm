//! Models in a crate that sees `bases-b` only: `Audit` brings `Timestamps`
//! with it from `bases-a`.

// Proof 3: the global groups live in another crate, found through a plain
// `use`, even by a model whose fields come back through two base expansions.
use bases_b::groups::G;

/// Proof 2 and 8: fields from two bases in two crates, and unique keys of
/// every kind: a base's unique column, an expression, a partial and a composite.
#[morm::model(table = "spike_order", base = bases_b::Audit, borrowed)]
#[unique(name = lower_email, expr = "lower(\"email\")")]
#[unique(name = live_code, cols(code), filter = "\"deleted\" = false")]
#[unique(name = tenant_code, cols(tenant, code))]
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: i64,
    pub tenant: i64,
    pub code: String,
    pub email: String,
    pub deleted: bool,
    #[col(group = auditor)]
    pub note: Option<String>,
}
