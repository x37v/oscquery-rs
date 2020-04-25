use std::fmt;
use std::sync::Arc;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ClipMode<T> {
    None,
    Low(T),
    High(T),
    Both(T, T),
}

#[derive(Clone, PartialEq, Eq, Debug)]
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

pub trait GetSet<T>: Get<T> + Set<T> {
    fn as_get(&self) -> &dyn Get<T>;
    fn as_set(&self) -> &dyn Set<T>;
}

#[derive(Clone, Debug)]
pub struct Value<V, T> {
    pub value: V,
    pub clip_mode: ClipMode<T>,
    pub range: Range<T>,
    pub unit: Option<String>,
}

pub type ValueGet<T> = Value<Arc<dyn Get<T>>, T>;
pub type ValueSet<T> = Value<Arc<dyn Set<T>>, T>;
pub type ValueGetSet<T> = Value<Arc<dyn GetSet<T>>, T>;

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
