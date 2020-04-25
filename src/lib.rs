//https://github.com/Vidvox/OSCQueryProposal
use std::fmt;
use std::sync::Arc;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Access {
    NoValue = 0,
    ReadOnly = 1,
    WriteOnly = 2,
    ReadWrite = 3,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ClipMode<T> {
    None,
    Low(T),
    High(T),
    Both(T, T),
}

#[derive(PartialEq, Eq, Debug)]
pub enum Range<T> {
    None,
    Min(T),
    Max(T),
    MinMax(T, T),
    Vals(Box<[T]>),
}

pub trait Get<T>: Send {
    fn get(&self) -> T;
}

pub trait Set<T>: Send {
    fn set(&self, value: T);
}

pub trait GetSet<T>: Get<T> + Set<T> {
    fn as_get(&self) -> &dyn Get<T>;
    fn as_set(&self) -> &dyn Set<T>;
}

impl<T> fmt::Debug for dyn Get<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Get({:?})", self.get())
    }
}

impl<T> fmt::Debug for dyn Set<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Set")
    }
}

impl<T> fmt::Debug for dyn GetSet<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GetSet({:?})", self.as_get().get())
    }
}

//types:
//container
//read
//write
//read/write

#[derive(Debug)]
pub enum Node {
    Container {
        address: String,
        description: Option<String>,
    },
    Get {
        address: String,
        description: Option<String>,
        params: Vec<ParamGet>,
    },
    Set {
        address: String,
        description: Option<String>,
        params: Vec<ParamSet>,
    },
    GetSet {
        address: String,
        description: Option<String>,
        params: Vec<ParamGetSet>,
    },
}

impl Node {
    pub fn address_valid(address: &String) -> Result<(), &'static str> {
        //TODO test others
        if address.contains('/') {
            Err("invalid address")
        } else {
            Ok(())
        }
    }
    pub fn new_container(
        address: String,
        description: Option<String>,
    ) -> Result<Self, &'static str> {
        Self::address_valid(&address)?;
        Ok(Self::Container {
            address,
            description,
        })
    }
    pub fn access(&self) -> Access {
        match self {
            Node::Container { .. } => Access::NoValue,
            Node::Get { .. } => Access::ReadOnly,
            Node::Set { .. } => Access::WriteOnly,
            Node::GetSet { .. } => Access::ReadWrite,
        }
    }
    pub fn description(&self) -> &Option<String> {
        match self {
            Node::Container { description, .. } => description,
            Node::Get { description, .. } => description,
            Node::Set { description, .. } => description,
            Node::GetSet { description, .. } => description,
        }
    }
    pub fn address(&self) -> &String {
        match self {
            Node::Container { address, .. } => address,
            Node::Get { address, .. } => address,
            Node::Set { address, .. } => address,
            Node::GetSet { address, .. } => address,
        }
    }
}

#[derive(Debug)]
pub struct Value<V, T> {
    pub value: V,
    pub clip_mode: ClipMode<T>,
    pub range: Range<T>,
    pub unit: Option<String>,
}

pub type ValueGet<T> = Value<Arc<dyn Get<T>>, T>;
pub type ValueSet<T> = Value<Arc<dyn Set<T>>, T>;
pub type ValueGetSet<T> = Value<Arc<dyn GetSet<T>>, T>;

#[derive(Debug)]
pub enum ParamGet {
    Int(ValueGet<i32>),
    Float(ValueGet<f32>),
    String(ValueGet<String>),
    Blob(ValueGet<Box<[u8]>>), //does clip mode make and range make sense?
    Time(ValueGet<(u32, u32)>),
    Long(ValueGet<i64>),
    Double(ValueGet<f64>),
    Char(ValueGet<char>),
    Midi(ValueGet<(u8, u8, u8, u8)>),
    Bool(ValueGet<bool>),
    Array(Box<[ParamGet]>),
    Nil,
    Inf,
}

#[derive(Debug)]
pub enum ParamSet {
    Int(ValueSet<i32>),
    Float(ValueSet<f32>),
    String(ValueSet<String>),
    Blob(ValueSet<Box<[u8]>>), //does clip mode make and range make sense?
    Time(ValueSet<(u32, u32)>),
    Long(ValueSet<i64>),
    Double(ValueSet<f64>),
    Char(ValueSet<char>),
    Midi(ValueSet<(u8, u8, u8, u8)>),
    Bool(ValueSet<bool>),
    Array(Box<[ParamSet]>),
}

#[derive(Debug)]
pub enum ParamGetSet {
    Int(ValueGetSet<i32>),
    Float(ValueGetSet<f32>),
    String(ValueGetSet<String>),
    Blob(ValueGetSet<Box<[u8]>>), //does clip mode make and range make sense?
    Time(ValueGetSet<(u32, u32)>),
    Long(ValueGetSet<i64>),
    Double(ValueGetSet<f64>),
    Char(ValueGetSet<char>),
    Midi(ValueGetSet<(u8, u8, u8, u8)>),
    Bool(ValueGetSet<bool>),
    Array(Box<[ParamGetSet]>),
}

//XXX need a node 'renderer' that can take a snapshot of all the aparamters for
//a node and send it somewhere to be sent out... this way we can have precise message sending from another thread

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn can_build() {
        let c = Node::new_container("soda".to_string(), None);
        assert_matches!(c, Ok(Node::Container { .. }));
        println!("{:?}", 32usize);
    }
}
