//! OscUpdate function wrappers.
use crate::node::OscUpdate;
use crate::root::OscWriteCallback;

use rosc::OscType;
use std::net::SocketAddr;

pub struct UpdateFunc<F>(pub F);
pub struct UpdateFunc2<F>(pub F);

impl<F> OscUpdate for UpdateFunc<F>
where
    F: Fn(&Vec<OscType>, Option<SocketAddr>, Option<(u32, u32)>) -> (),
{
    fn osc_update(
        &self,
        args: &Vec<OscType>,
        addr: Option<SocketAddr>,
        time: Option<(u32, u32)>,
    ) -> Option<OscWriteCallback> {
        (self.0)(args, addr, time);
        None
    }
}

impl<F> OscUpdate for UpdateFunc2<F>
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
