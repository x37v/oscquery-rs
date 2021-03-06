//! OSCQuery tree items.
use crate::{
    osc::{OscMidiMessage, OscType},
    param::*,
    root::{NodeHandle, OscWriteCallback},
};
use std::fmt;
use std::net::SocketAddr;

use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::convert::From;

pub type UpdateHandler = Box<dyn OscUpdate + Send + Sync>;

pub trait OscUpdate {
    fn osc_update(
        &self,
        args: &Vec<OscType>,
        addr: Option<SocketAddr>,
        time: Option<(u32, u32)>,
        handle: &NodeHandle,
    ) -> Option<OscWriteCallback>;
}

pub trait OscRender {
    fn osc_render(&self, args: &mut Vec<OscType>);
}

pub fn address_valid(address: String) -> Result<String, &'static str> {
    //TODO test others
    if address.contains('/') {
        Err("invalid address")
    } else {
        Ok(address)
    }
}

/// Data access modes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Access {
    NoValue = 0,
    ReadOnly = 1,
    WriteOnly = 2,
    ReadWrite = 3,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NodeQueryParam {
    Value,
    Type,
    Range,
    ClipMode,
    Access,
    Description,
    Unit,
}

//types:
//container
//read
//write
//read/write

#[derive(Debug)]
pub struct Container {
    pub(crate) address: String,
    pub(crate) description: Option<String>,
}

#[derive(Debug)]
pub struct Get {
    address: String,
    description: Option<String>,
    params: Box<[ParamGet]>,
}

pub struct Set {
    address: String,
    description: Option<String>,
    params: Box<[ParamSet]>,
    handler: Option<UpdateHandler>,
}

pub struct GetSet {
    address: String,
    description: Option<String>,
    params: Box<[ParamGetSet]>,
    handler: Option<UpdateHandler>,
}

#[derive(Debug)]
pub enum Node {
    Container(Container),
    Get(Get),
    Set(Set),
    GetSet(GetSet),
}

impl fmt::Debug for Set {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "address={:?} description={:?}, params={:?}, handler={:?}",
            self.address,
            self.description,
            self.params,
            self.handler.is_some()
        )
    }
}

impl std::fmt::Debug for GetSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "address={:?} description={:?}, params={:?}, handler={:?}",
            self.address,
            self.description,
            self.params,
            self.handler.is_some()
        )
    }
}

impl Container {
    pub fn new<A>(address: A, description: Option<&str>) -> Result<Self, &'static str>
    where
        A: ToString,
    {
        Ok(Self {
            address: address_valid(address.to_string())?,
            description: description.map(|d| d.into()),
        })
    }
}

impl Get {
    pub fn new<I, A>(address: A, description: Option<&str>, params: I) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = ParamGet>,
        A: ToString,
    {
        Ok(Self {
            address: address_valid(address.to_string())?,
            description: description.map(|d| d.into()),
            params: params.into_iter().collect::<Vec<_>>().into(),
        })
    }
}

impl Set {
    pub fn new<I, A>(
        address: A,
        description: Option<&str>,
        params: I,
        handler: Option<UpdateHandler>,
    ) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = ParamSet>,
        A: ToString,
    {
        Ok(Self {
            address: address_valid(address.to_string())?,
            description: description.map(|d| d.into()),
            params: params.into_iter().collect::<Vec<_>>().into(),
            handler,
        })
    }
}

impl GetSet {
    pub fn new<I, A>(
        address: A,
        description: Option<&str>,
        params: I,
        handler: Option<UpdateHandler>,
    ) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = ParamGetSet>,
        A: ToString,
    {
        Ok(Self {
            address: address_valid(address.to_string())?,
            description: description.map(|d| d.into()),
            params: params.into_iter().collect::<Vec<_>>().into(),
            handler,
        })
    }
}

impl Serialize for Access {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(match self {
            Self::NoValue => 0,
            Self::ReadOnly => 1,
            Self::WriteOnly => 2,
            Self::ReadWrite => 3,
        })
    }
}

impl Node {
    pub fn access(&self) -> Access {
        match self {
            Node::Container(_) => Access::NoValue,
            Node::Get(_) => Access::ReadOnly,
            Node::Set(_) => Access::WriteOnly,
            Node::GetSet(_) => Access::ReadWrite,
        }
    }
    pub fn description(&self) -> &Option<String> {
        match self {
            Node::Container(n) => &n.description,
            Node::Get(n) => &n.description,
            Node::Set(n) => &n.description,
            Node::GetSet(n) => &n.description,
        }
    }
    pub fn address(&self) -> &String {
        match self {
            Node::Container(n) => &n.address,
            Node::Get(n) => &n.address,
            Node::Set(n) => &n.address,
            Node::GetSet(n) => &n.address,
        }
    }
    pub fn type_string(&self) -> Option<String> {
        match self {
            Node::Container(..) => None,
            Node::Get(n) => Some(
                n.params
                    .iter()
                    .fold(String::new(), |acc, x| acc + x.osc_type_str().as_str()),
            ),
            Node::Set(n) => Some(
                n.params
                    .iter()
                    .fold(String::new(), |acc, x| acc + x.osc_type_str().as_str()),
            ),
            Node::GetSet(n) => Some(
                n.params
                    .iter()
                    .fold(String::new(), |acc, x| acc + x.osc_type_str().as_str()),
            ),
        }
    }
}

pub(crate) struct NodeValueWrapper<'a>(pub(crate) &'a Node);
impl<'a> Serialize for NodeValueWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Node::Set(..) | Node::Container(..) => serializer.serialize_none(),
            Node::Get(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetValueWrapper(v))?;
                }
                seq.end()
            }
            Node::GetSet(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetSetValueWrapper(v))?;
                }
                seq.end()
            }
        }
    }
}

pub(crate) struct NodeUnitWrapper<'a>(pub(crate) &'a Node);
impl<'a> Serialize for NodeUnitWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Node::Container(..) => serializer.serialize_none(),
            Node::Get(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetUnitWrapper(v))?;
                }
                seq.end()
            }
            Node::Set(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamSetUnitWrapper(v))?;
                }
                seq.end()
            }
            Node::GetSet(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetSetUnitWrapper(v))?;
                }
                seq.end()
            }
        }
    }
}

pub(crate) struct NodeRangeWrapper<'a>(pub(crate) &'a Node);
impl<'a> Serialize for NodeRangeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Node::Container(..) => serializer.serialize_none(),
            Node::Get(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetRangeWrapper(v))?;
                }
                seq.end()
            }
            Node::Set(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamSetRangeWrapper(v))?;
                }
                seq.end()
            }
            Node::GetSet(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetSetRangeWrapper(v))?;
                }
                seq.end()
            }
        }
    }
}

pub(crate) struct NodeClipModeWrapper<'a>(pub(crate) &'a Node);
impl<'a> Serialize for NodeClipModeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Node::Container(..) => serializer.serialize_none(),
            Node::Get(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetClipModeWrapper(v))?;
                }
                seq.end()
            }
            Node::Set(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamSetClipModeWrapper(v))?;
                }
                seq.end()
            }
            Node::GetSet(n) => {
                let mut seq = serializer.serialize_seq(Some(n.params.len()))?;
                for v in n.params.iter() {
                    seq.serialize_element(&ParamGetSetClipModeWrapper(v))?;
                }
                seq.end()
            }
        }
    }
}

impl OscUpdate for Node {
    fn osc_update(
        &self,
        args: &Vec<OscType>,
        addr: Option<SocketAddr>,
        time: Option<(u32, u32)>,
        handle: &NodeHandle,
    ) -> Option<OscWriteCallback> {
        match self {
            Self::Container(..) | Self::Get(..) => None,
            Self::Set(n) => n.osc_update(args, addr, time, handle),
            Self::GetSet(n) => n.osc_update(args, addr, time, handle),
        }
    }
}

impl OscRender for Node {
    fn osc_render(&self, args: &mut Vec<OscType>) {
        match self {
            Self::Container(..) | Self::Set(..) => (),
            Self::Get(n) => n.osc_render(args),
            Self::GetSet(n) => n.osc_render(args),
        };
    }
}

macro_rules! impl_osc_update {
    ($t:ty, $p:ident) => {
        impl OscUpdate for $t {
            fn osc_update(
                &self,
                args: &Vec<OscType>,
                addr: Option<SocketAddr>,
                time: Option<(u32, u32)>,
                handle: &NodeHandle,
            ) -> Option<OscWriteCallback> {
                //XXX for GetSet, should we trigger if we actually did do a set?

                let mut cb = None;
                //if we have a handler, exec and see if we should continue
                if let Some(handler) = &self.handler {
                    cb = handler.osc_update(args, addr, time, handle);
                }
                for (p, a) in self.params.iter().zip(args) {
                    match a {
                        OscType::Int(v) => {
                            if let $p::Int(s) = p {
                                s.value().set(*v);
                            }
                        }
                        OscType::Float(v) => {
                            if let $p::Float(s) = p {
                                s.value().set(*v);
                            }
                        }
                        OscType::String(v) => {
                            if let $p::String(s) = p {
                                s.value().set(v.to_owned());
                            }
                        }
                        OscType::Time(v) => {
                            if let $p::Time(s) = p {
                                s.value().set(*v);
                            }
                        }
                        OscType::Long(v) => {
                            if let $p::Long(s) = p {
                                s.value().set(*v);
                            }
                        }
                        OscType::Double(v) => {
                            if let $p::Double(s) = p {
                                s.value().set(*v);
                            }
                        }
                        OscType::Char(v) => {
                            if let $p::Char(s) = p {
                                s.value().set(*v);
                            }
                        }
                        OscType::Midi(v) => {
                            if let $p::Midi(s) = p {
                                s.value().set((v.port, v.status, v.data1, v.data2));
                            }
                        }
                        OscType::Bool(v) => {
                            if let $p::Bool(s) = p {
                                s.value().set(*v);
                            }
                        }
                        //TODO
                        OscType::Blob(..)
                        | OscType::Color(..)
                        | OscType::Array(..)
                        | OscType::Nil
                        | OscType::Inf => unimplemented!(),
                    }
                }
                cb
            }
        }
    };
}

macro_rules! impl_osc_render {
    ($t:ty, $p:ident) => {
        impl OscRender for $t {
            fn osc_render(&self, args: &mut Vec<OscType>) {
                for p in self.params.iter() {
                    match p {
                        $p::Int(v) => args.push(OscType::Int(v.value().get())),
                        $p::Float(v) => args.push(OscType::Float(v.value().get())),
                        $p::String(v) => args.push(OscType::String(v.value().get().clone())),
                        $p::Time(v) => args.push(OscType::Time(v.value.get())),
                        $p::Long(v) => args.push(OscType::Long(v.value().get())),
                        $p::Double(v) => args.push(OscType::Double(v.value().get())),
                        $p::Char(v) => args.push(OscType::Char(v.value().get())),
                        $p::Midi(v) => {
                            let v = v.value().get();
                            args.push(OscType::Midi(OscMidiMessage {
                                port: v.0,
                                status: v.1,
                                data1: v.2,
                                data2: v.3,
                            }))
                        }
                        $p::Bool(v) => args.push(OscType::Bool(v.value().get())),
                        $p::Array(v) => args.push(OscType::Array(v.value().get())),
                    }
                }
            }
        }
    };
}

impl_osc_update!(Set, ParamSet);
impl_osc_update!(GetSet, ParamGetSet);

impl_osc_render!(Get, ParamGet);
impl_osc_render!(GetSet, ParamGetSet);

impl From<Container> for Node {
    fn from(n: Container) -> Self {
        Self::Container(n)
    }
}

impl From<Get> for Node {
    fn from(n: Get) -> Self {
        Self::Get(n)
    }
}

impl From<Set> for Node {
    fn from(n: Set) -> Self {
        Self::Set(n)
    }
}

impl From<GetSet> for Node {
    fn from(n: GetSet) -> Self {
        Self::GetSet(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn access() {
        for (a, t) in &[
            (Access::NoValue, json!(0)),
            (Access::ReadOnly, json!(1)),
            (Access::WriteOnly, json!(2)),
            (Access::ReadWrite, json!(3)),
        ] {
            let v = serde_json::to_value(*a);
            assert!(v.is_ok());
            assert_eq!(v.unwrap(), t.clone());
        }
    }

    #[test]
    fn can_build() {
        let c = Container::new("soda".to_string(), None);
        assert_matches!(c, Ok(Container { .. }));
        let c = Container::new("/soda".to_string(), None);
        assert_matches!(c, Err(..));
    }
}
