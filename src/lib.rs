use std::sync::Arc;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Access {
    NoValue = 0,
    ReadOnly = 1,
    WriteOnly = 2,
    ReadWrite = 3,
}

pub enum ClipMode<T> {
    None,
    Low(T),
    High(T),
    Both(T, T),
}

pub enum Range<T> {
    None,
    Min(T),
    Max(T),
    MinMax(T, T),
    Vals(Vec<T>),
}

pub trait Get<T>: Send {
    fn get(&self) -> T;
}

pub trait Set<T>: Send {
    fn set(&self, value: T);
}

//types:
//container
//read
//write
//read/write

pub enum Node {
    Container {
        description: Option<String>,
        address: String,
    },
    Get {
        description: Option<String>,
        address: String,
        values: Vec<ValueGet>,
    },
    Set {
        description: Option<String>,
        address: String,
        values: Vec<ValueSet>,
    },
    GetSet {
        description: Option<String>,
        address: String,
        values: Vec<ValueGetSet>,
    },
}

impl Node {
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

pub struct Value<V, T> {
    pub value: V,
    pub clip_mode: ClipMode<T>,
    pub range: Range<T>,
    pub unit: Option<String>,
}

pub enum ValueGet {
    Int(Value<Arc<dyn Get<i32>>, i32>),
    Float(Value<Arc<dyn Get<f32>>, f32>),
    String(Value<Arc<dyn Get<String>>, String>),
    Blob(Value<Arc<dyn Get<Box<[u8]>>>, Box<[u8]>>),
    Time(Value<Arc<dyn Get<(u32, u32)>>, (u32, u32)>),
    Long(Value<Arc<dyn Get<i64>>, i64>),
    Double(Value<Arc<dyn Get<f64>>, f64>),
    Char(Value<Arc<dyn Get<char>>, char>),
    Midi(Value<Arc<dyn Get<(u8, u8, u8, u8)>>, (u8, u8, u8, u8)>),
    Bool(Value<Arc<dyn Get<bool>>, bool>),
    Array(Box<[ValueGet]>),
    Nil,
    Inf,
}

pub enum ValueSet {
    Int(Value<Arc<dyn Set<i32>>, i32>),
    Float(Value<Arc<dyn Set<f32>>, f32>),
    String(Value<Arc<dyn Set<String>>, String>),
    Blob(Value<Arc<dyn Set<Box<[u8]>>>, Box<[u8]>>),
    Time(Value<Arc<dyn Set<(u32, u32)>>, (u32, u32)>),
    Long(Value<Arc<dyn Set<i64>>, i64>),
    Double(Value<Arc<dyn Set<f64>>, f64>),
    Char(Value<Arc<dyn Set<char>>, char>),
    Midi(Value<Arc<dyn Set<(u8, u8, u8, u8)>>, (u8, u8, u8, u8)>),
    Bool(Value<Arc<dyn Set<bool>>, bool>),
    Array(Box<[ValueSet]>),
}

pub enum ValueGetSet {
    Int(Value<(Arc<dyn Get<i32>>, Arc<dyn Set<i32>>), i32>),
    Float(Value<(Arc<dyn Get<f32>>, Arc<dyn Set<f32>>), f32>),
    String(Value<(Arc<dyn Get<String>>, Arc<dyn Set<String>>), String>),
    Blob(Value<Arc<dyn Set<Box<[u8]>>>, Box<[u8]>>),
    Time(Value<Arc<dyn Set<(u32, u32)>>, (u32, u32)>),
    Long(Value<(Arc<dyn Get<i64>>, Arc<dyn Set<i64>>), i64>),
    Double(Value<(Arc<dyn Get<f64>>, Arc<dyn Set<f64>>), f64>),
    Char(Value<(Arc<dyn Get<char>>, Arc<dyn Set<char>>), char>),
    Midi(Value<Arc<dyn Set<(u8, u8, u8, u8)>>, (u8, u8, u8, u8)>),
    Bool(Value<(Arc<dyn Get<bool>>, Arc<dyn Set<bool>>), bool>),
    Array(Box<[ValueGetSet]>),
}

//XXX need a node 'renderer' that can take a snapshot of all the aparamters for
//a node and send it somewhere to be sent out... this way we can have precise message sending from another thread

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
