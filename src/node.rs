use crate::param::*;
use serde::{Deserialize, Serialize, Serializer};
use std::convert::From;

pub fn address_valid(address: String) -> Result<String, &'static str> {
    //TODO test others
    if address.contains('/') {
        Err("invalid address")
    } else {
        Ok(address)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Access {
    NoValue = 0,
    ReadOnly = 1,
    WriteOnly = 2,
    ReadWrite = 3,
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

#[derive(Debug)]
pub struct Set {
    address: String,
    description: Option<String>,
    params: Box<[ParamSet]>,
}

#[derive(Debug)]
pub struct GetSet {
    address: String,
    description: Option<String>,
    params: Box<[ParamGetSet]>,
}

#[derive(Debug)]
pub enum Node {
    Container(Container),
    Get(Get),
    Set(Set),
    GetSet(GetSet),
}

impl Container {
    pub fn new(address: String, description: Option<String>) -> Result<Self, &'static str> {
        Ok(Self {
            address: address_valid(address)?,
            description,
        })
    }
}

impl Get {
    pub fn new<I>(
        address: String,
        description: Option<String>,
        params: I,
    ) -> Result<Self, &'static str>
    where
        I: Iterator<Item = ParamGet>,
    {
        Ok(Self {
            address: address_valid(address)?,
            description,
            params: params.collect::<Vec<_>>().into(),
        })
    }
}

impl Set {
    pub fn new<I>(
        address: String,
        description: Option<String>,
        params: I,
    ) -> Result<Self, &'static str>
    where
        I: Iterator<Item = ParamSet>,
    {
        Ok(Self {
            address: address_valid(address)?,
            description,
            params: params.collect::<Vec<_>>().into(),
        })
    }
}

impl GetSet {
    pub fn new<I>(
        address: String,
        description: Option<String>,
        params: I,
    ) -> Result<Self, &'static str>
    where
        I: Iterator<Item = ParamGetSet>,
    {
        Ok(Self {
            address: address_valid(address)?,
            description,
            params: params.collect::<Vec<_>>().into(),
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
}

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
