//! Proof 2: a base in one crate.

pub mod stamps {
    /// Timestamps every model in other crates can take.
    #[morm::base]
    pub struct Timestamps {
        pub created_at: i64,
        pub updated_at: i64,
    }
}

pub use stamps::Timestamps;
