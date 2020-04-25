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

impl<T> Default for ClipMode<T> {
    fn default() -> Self {
        ClipMode::<T>::None
    }
}

impl<T> Default for Range<T> {
    fn default() -> Self {
        Range::<T>::None
    }
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

pub struct ValueBuilder<V, T> {
    value: Value<V, T>,
}

impl<V, T> ValueBuilder<V, T> {
    pub fn new(value: V) -> Self {
        let value = Value {
            value,
            clip_mode: Default::default(),
            range: Default::default(),
            unit: Default::default(),
        };
        Self { value }
    }

    pub fn with_clip_mode(mut self, clip_mode: ClipMode<T>) -> Self {
        self.value.clip_mode = clip_mode;
        self
    }

    pub fn with_range(mut self, range: Range<T>) -> Self {
        self.value.range = range;
        self
    }

    pub fn with_unit(mut self, unit: String) -> Self {
        self.value.unit = Some(unit);
        self
    }

    pub fn build(self) -> Value<V, T> {
        self.value
    }
}

impl<V, T> Value<V, T> {
    pub fn value(&self) -> &V {
        &self.value
    }

    pub fn clip_mode(&self) -> &ClipMode<T> {
        &self.clip_mode
    }

    pub fn range(&self) -> &Range<T> {
        &self.range
    }

    pub fn unit(&self) -> &Option<String> {
        &self.unit
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct A(i32);
    struct B(AtomicUsize);

    impl Get<i32> for A {
        fn get(&self) -> i32 {
            self.0
        }
    }

    impl Default for B {
        fn default() -> Self {
            Self(AtomicUsize::new(0))
        }
    }

    impl Set<u32> for B {
        fn set(&self, v: u32) {
            self.0.store(v as usize, Ordering::Relaxed);
        }
    }

    #[test]
    fn can_build() {
        let b: ValueGet<i32> = ValueBuilder::new(Arc::new(A(23i32)) as _).build();
        assert_eq!(b.value().get(), 23i32);
        assert_eq!(b.clip_mode(), &ClipMode::None);
        assert_eq!(b.range(), &Range::None);

        let b: ValueGet<i32> = ValueBuilder::new(Arc::new(A(23i32)) as _)
            .with_range(Range::MinMax(-1, 24))
            .with_unit("horses".into())
            .build();
        assert_eq!(b.clip_mode(), &ClipMode::None);
        assert_eq!(b.range(), &Range::MinMax(-1i32, 24i32));
        assert_eq!(b.unit(), &Some("horses".to_string()));

        let a: Arc<B> = Arc::new(Default::default());
        assert_eq!(a.0.load(Ordering::Relaxed), 0usize);
        let b: ValueSet<u32> = ValueBuilder::new(a.clone() as _).build();
        b.value().set(5u32);
        assert_eq!(a.0.load(Ordering::Relaxed), 5usize);
    }
}
