//! Function wrappers.
use crate::node::OscUpdate;
use crate::root::{NodeHandle, OscWriteCallback};

use crate::osc::OscType;
use std::marker::PhantomData;
use std::net::SocketAddr;

/// A new-type wrapper for a function that can get OSC updates and potentially modify the OSCQuery
/// graph.
pub struct OscUpdateFunc<F>(pub F);

impl<F> OscUpdateFunc<F> {
    pub fn new(func: F) -> Self {
        Self(func)
    }
}

impl<F> OscUpdate for OscUpdateFunc<F>
where
    F: Fn(
        &Vec<OscType>,
        Option<SocketAddr>,
        Option<(u32, u32)>,
        &NodeHandle,
    ) -> Option<OscWriteCallback>,
{
    fn osc_update(
        &self,
        args: &Vec<OscType>,
        addr: Option<SocketAddr>,
        time: Option<(u32, u32)>,
        handle: &NodeHandle,
    ) -> Option<OscWriteCallback> {
        (self.0)(args, addr, time, handle)
    }
}

/// A new-type wrapper for a function that can get a value.
pub struct GetFunc<F, T> {
    func: F,
    _phantom: PhantomData<T>,
}

/// A new-type wrapper for a function that can set a value.
pub struct SetFunc<F, T> {
    func: F,
    _phantom: PhantomData<T>,
}

/// A new-type wrapper for a get and set functions.
pub struct GetSetFuncs<G, S, T> {
    get: G,
    set: S,
    _phantom: PhantomData<T>,
}

impl<F, T> GetFunc<F, T>
where
    F: Fn() -> T + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: PhantomData,
        }
    }
}

impl<F, T> SetFunc<F, T>
where
    F: Fn(T) -> () + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: PhantomData,
        }
    }
}

impl<G, S, T> GetSetFuncs<G, S, T>
where
    G: Fn() -> T + Send + Sync,
    S: Fn(T) -> () + Send + Sync,
{
    pub fn new(get: G, set: S) -> Self {
        Self {
            get,
            set,
            _phantom: PhantomData,
        }
    }
}

impl<F, T> crate::value::Get<T> for GetFunc<F, T>
where
    F: Fn() -> T + Send + Sync,
    T: Send + Sync,
{
    fn get(&self) -> T {
        (self.func)()
    }
}

impl<F, T> crate::value::Set<T> for SetFunc<F, T>
where
    F: Fn(T) -> () + Send + Sync,
    T: Send + Sync,
{
    fn set(&self, value: T) {
        (self.func)(value)
    }
}

impl<G, S, T> crate::value::Get<T> for GetSetFuncs<G, S, T>
where
    G: Fn() -> T + Send + Sync,
    S: Send + Sync,
    T: Send + Sync,
{
    fn get(&self) -> T {
        (self.get)()
    }
}

impl<G, S, T> crate::value::Set<T> for GetSetFuncs<G, S, T>
where
    G: Send + Sync,
    S: Fn(T) -> () + Send + Sync,
    T: Send + Sync,
{
    fn set(&self, value: T) {
        (self.set)(value)
    }
}
