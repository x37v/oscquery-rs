use std::sync::Arc;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Access {
    NoValue = 0,
    ReadOnly = 1,
    WriteOnly = 2,
    ReadWrite = 3,
}

pub trait Get<T>: Send + Sync {
    fn get(&self) -> T;
}

pub trait Set<T>: Send + Sync {
    fn set(&self, value: T);
}

pub enum ValueAccess<T> {
    Get(Arc<dyn Get<T>>),
    Set(Arc<dyn Set<T>>),
    GetSet(Arc<dyn Get<T>>, Arc<dyn Set<T>>),
}

pub trait Node {
    fn address(&self) -> &str;
    fn access(&self) -> Access;

    //XXX
    //fn values(&self) -> Iterator?
}

//XXX use marker traits?

/*
Int(i32)
Float(f32)
String(String)
Blob(Vec<u8>)
Time(u32, u32)
Long(i64)
Double(f64)
Char(char)
Color(OscColor)
Midi(OscMidiMessage)
Bool(bool)
Array(OscArray)
Nil
Inf
*/

pub enum ValueGet {
    Int(Arc<dyn Get<i32>>),
    Float(Arc<dyn Get<f32>>),
    String(Arc<dyn Get<String>>),
    Blob(Arc<dyn Get<Box<[u8]>>>),
    Time(Arc<dyn Get<(u32, u32)>>),
    Long(Arc<dyn Get<i64>>),
    Double(Arc<dyn Get<f64>>),
    Char(Arc<dyn Get<char>>),
    Midi(Arc<dyn Get<(u8, u8, u8, u8)>>),
    Bool(Arc<dyn Get<bool>>),
    Array(Arc<dyn Get<Box<[ValueGet]>>>),
    Nil,
    Inf,
}

pub enum ValueSet {
    Int(Arc<dyn Set<i32>>),
    Float(Arc<dyn Set<f32>>),
    String(Arc<dyn Set<String>>),
    Blob(Arc<dyn Set<Box<[u8]>>>),
    Time(Arc<dyn Set<(u32, u32)>>),
    Long(Arc<dyn Set<i64>>),
    Double(Arc<dyn Set<f64>>),
    Char(Arc<dyn Set<char>>),
    Midi(Arc<dyn Set<(u8, u8, u8, u8)>>),
    Bool(Arc<dyn Set<bool>>),
    Array(Arc<dyn Set<Box<[ValueSet]>>>),
}

pub enum ValueGetSet {
    Int(Arc<dyn Get<i32>>, Arc<dyn Set<i32>>),
    Float(Arc<dyn Get<f32>>, Arc<dyn Set<f32>>),
    String(Arc<dyn Get<String>>, Arc<dyn Set<String>>),
    Blob(Arc<dyn Get<Box<[u8]>>>, Arc<dyn Set<Box<[u8]>>>),
    Time(Arc<dyn Get<(u32, u32)>>, Arc<dyn Set<(u32, u32)>>),
    Long(Arc<dyn Get<i64>>, Arc<dyn Set<i64>>),
    Double(Arc<dyn Get<f64>>, Arc<dyn Set<f64>>),
    Char(Arc<dyn Get<char>>, Arc<dyn Set<char>>),
    Midi(
        Arc<dyn Get<(u8, u8, u8, u8)>>,
        Arc<dyn Set<(u8, u8, u8, u8)>>,
    ),
    Bool(Arc<dyn Get<bool>>, Arc<dyn Set<bool>>),
    Array(Arc<dyn Set<Box<[ValueGetSet]>>>),
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

//XXX need a node 'renderer' that can take a snapshot of all the aparamters for
//a node and send it somewhere to be sent out... this way we can have precise message sending from another thread

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
