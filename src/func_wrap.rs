//! Function wrappers.
use crate::node::OscUpdate;
use crate::root::OscWriteCallback;

use rosc::OscType;
use std::marker::PhantomData;
use std::net::SocketAddr;

/// A new-type wrapper for a function that can get OSC updates and potentially modify the OSCQuery
/// graph.
pub struct OscUpdateFunc<F>(pub F);

impl<F> OscUpdate for OscUpdateFunc<F>
where
    F: Fn(&Vec<OscType>, Option<SocketAddr>, Option<(u32, u32)>) -> Option<OscWriteCallback>,
{
    fn osc_update(
        &self,
        args: &Vec<OscType>,
        addr: Option<SocketAddr>,
        time: Option<(u32, u32)>,
    ) -> Option<OscWriteCallback> {
        (self.0)(args, addr, time)
    }
}

/// A new-type wrapper for a function that can get a value.
pub struct GetFunc<F, T> {
    func: F,
    _phantom: PhantomData<T>,
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
