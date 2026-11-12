//! Proof 3: group masks whose size the macros count, generic on stable.

use crate::{Db, Model};
use std::marker::PhantomData;

/// `N` 64-bit words: `groups!` and `#[model(groups(..))]` pick `N`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bits<const N: usize>(pub [u64; N]);

/// What generic code needs from a mask, whatever its size.
pub trait MaskBits: Copy + Send + Sync + 'static {
    const EMPTY: Self;
    const CAPACITY: usize;
    fn set(&mut self, bit: u16);
    fn has(&self, bit: u16) -> bool;
}

impl<const N: usize> MaskBits for Bits<N> {
    const EMPTY: Self = Bits([0; N]);
    const CAPACITY: usize = N * 64;
    #[inline]
    fn set(&mut self, bit: u16) {
        self.0[bit as usize / 64] |= 1 << (bit % 64);
    }
    #[inline]
    fn has(&self, bit: u16) -> bool {
        self.0[bit as usize / 64] & (1 << (bit % 64)) != 0
    }
}

/// A model with no scoped groups, or an app with no global ones.
impl MaskBits for () {
    const EMPTY: Self = ();
    const CAPACITY: usize = 0;
    fn set(&mut self, _bit: u16) {
        unreachable!("no groups")
    }
    fn has(&self, _bit: u16) -> bool {
        false
    }
}

/// What `groups!` implements on its `G` type: the app's global mask.
pub trait GlobalGroups {
    type Mask: MaskBits;
}

/// A global group: `G.<name>`. Typed by the app's mask.
pub struct GlobalGroup<G>(u16, PhantomData<fn() -> G>);
impl<G> GlobalGroup<G> {
    #[doc(hidden)]
    pub const fn new(bit: u16) -> Self {
        Self(bit, PhantomData)
    }
    pub const fn bit(&self) -> u16 {
        self.0
    }
}
impl<G> Clone for GlobalGroup<G> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<G> Copy for GlobalGroup<G> {}

/// A model-scoped group: `Model::G.<name>`. Typed by its model.
pub struct ScopedGroup<M>(u16, PhantomData<fn() -> M>);
impl<M> ScopedGroup<M> {
    #[doc(hidden)]
    pub const fn new(bit: u16) -> Self {
        Self(bit, PhantomData)
    }
}
impl<M> Clone for ScopedGroup<M> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M> Copy for ScopedGroup<M> {}

/// What a column needs to be visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Req {
    Any,
    Global(u16),
    Scoped(u16),
}

/// The groups a caller holds, for one model.
pub struct Mask<M: Model> {
    global: M::Global,
    scoped: M::Scoped,
}

impl<M: Model> Clone for Mask<M> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M: Model> Copy for Mask<M> {}

impl<M: Model> Default for Mask<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Model> Mask<M> {
    pub const fn new() -> Self {
        Self {
            global: <M::Global as MaskBits>::EMPTY,
            scoped: <M::Scoped as MaskBits>::EMPTY,
        }
    }
    pub fn with(mut self, g: GlobalGroup<M::Global>) -> Self {
        self.global.set(g.bit());
        self
    }
    pub fn with_scoped(mut self, g: ScopedGroup<M>) -> Self {
        self.scoped.set(g.0);
        self
    }
    #[inline]
    pub fn allows(&self, req: Req) -> bool {
        let sudo = <M::Global as MaskBits>::CAPACITY > 0 && self.global.has(0);
        match req {
            Req::Any => true,
            Req::Global(b) => sudo || self.global.has(b),
            Req::Scoped(b) => sudo || self.scoped.has(b),
        }
    }
}

/// `db.groups(&mask)`: queries for the mask's model only.
pub struct Grouped<'a, M: Model> {
    _db: &'a Db,
    mask: &'a Mask<M>,
}

impl Db {
    pub fn groups<'a, M: Model>(&'a self, mask: &'a Mask<M>) -> Grouped<'a, M> {
        Grouped { _db: self, mask }
    }
}

impl<M: Model> Grouped<'_, M> {
    /// The columns this mask may read.
    pub fn columns(&self) -> impl Iterator<Item = &'static str> + '_ {
        M::COLUMNS
            .iter()
            .zip(M::GROUPS)
            .filter(|(_, r)| self.mask.allows(**r))
            .map(|(c, _)| *c)
    }
}
