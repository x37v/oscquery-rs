use crate::param::*;

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
    address: String,
    description: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_build() {
        let c = Container::new("soda".to_string(), None);
        assert_matches!(c, Ok(Container { .. }));
        let c = Container::new("/soda".to_string(), None);
        assert_matches!(c, Err(..));
    }
}
