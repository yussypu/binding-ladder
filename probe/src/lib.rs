#![recursion_limit = "256"]
use lock_ordering::relation::{LockAfter, LockBefore};
use lock_ordering::impl_transitive_lock_order;

pub enum L0 {}
pub enum L1 {}
pub enum L2 {}
pub enum L3 {}
pub enum L4 {}
pub enum L5 {}
pub enum L6 {}
pub enum L7 {}
pub enum L8 {}
pub enum L9 {}

impl LockAfter<L0> for L1 {}
impl_transitive_lock_order!(L0 => L1);
impl LockAfter<L1> for L2 {}
impl_transitive_lock_order!(L1 => L2);
impl LockAfter<L2> for L3 {}
impl_transitive_lock_order!(L2 => L3);
impl LockAfter<L3> for L4 {}
impl_transitive_lock_order!(L3 => L4);
impl LockAfter<L4> for L5 {}
impl_transitive_lock_order!(L4 => L5);
impl LockAfter<L5> for L6 {}
impl_transitive_lock_order!(L5 => L6);
impl LockAfter<L6> for L7 {}
impl_transitive_lock_order!(L6 => L7);
impl LockAfter<L7> for L8 {}
impl_transitive_lock_order!(L7 => L8);
impl LockAfter<L8> for L9 {}
impl_transitive_lock_order!(L8 => L9);

fn assert_before<A, B>() where A: LockBefore<B> {}
// One root->deepest proof forces full transitive-closure resolution.
pub fn exercise() { assert_before::<L0, L9>(); }
