use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use std::fmt;
use std::sync::Arc;

pub mod atomic;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipMode {
    None,
    Low,
    High,
    Both,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Range<T> {
    None,
    Min(T),
    Max(T),
    MinMax(T, T),
    Vals(Vec<T>),
}

impl<T> Serialize for Range<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::None => serializer.serialize_map(Some(0))?.end(),
            Self::Min(v) => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("MIN".into(), v)?;
                m.end()
            }
            Self::Max(v) => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("MAX".into(), v)?;
                m.end()
            }
            Self::MinMax(min, max) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("MIN".into(), min)?;
                m.serialize_entry("MAX".into(), max)?;
                m.end()
            }
            Self::Vals(values) => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("VALS".into(), values)?;
                m.end()
            }
        }
    }
}

impl Default for ClipMode {
    fn default() -> Self {
        ClipMode::None
    }
}

impl<T> Default for Range<T> {
    fn default() -> Self {
        Range::<T>::None
    }
}

pub trait Get<T>: Send + Sync {
    fn get(&self) -> T;
}

pub trait Set<T>: Send + Sync {
    fn set(&self, value: T);
}

pub trait GetSet<T>: Get<T> + Set<T> {
    fn as_get(&self) -> &dyn Get<T>;
    fn as_set(&self) -> &dyn Set<T>;
}

impl<X, T> GetSet<T> for X
where
    X: Get<T> + Set<T>,
{
    fn as_get(&self) -> &dyn Get<T> {
        self
    }
    fn as_set(&self) -> &dyn Set<T> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct Value<V, T> {
    pub value: V,
    pub clip_mode: ClipMode,
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

    pub fn with_clip_mode(mut self, clip_mode: ClipMode) -> Self {
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

    pub fn clip_mode(&self) -> &ClipMode {
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
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct A(i32);
    struct B(AtomicUsize);
    struct C(AtomicUsize);

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

    impl Default for C {
        fn default() -> Self {
            Self(AtomicUsize::new(0))
        }
    }

    impl Set<u32> for B {
        fn set(&self, v: u32) {
            self.0.store(v as usize, Ordering::Relaxed);
        }
    }

    impl Get<u32> for C {
        fn get(&self) -> u32 {
            self.0.load(Ordering::Relaxed) as u32
        }
    }

    impl Set<u32> for C {
        fn set(&self, v: u32) {
            self.0.store(v as usize, Ordering::Relaxed);
        }
    }

    #[test]
    fn clip_mode() {
        for (c, s) in &[
            (ClipMode::None, "none"),
            (ClipMode::Low, "low"),
            (ClipMode::High, "high"),
            (ClipMode::Both, "both"),
        ] {
            let v = serde_json::to_value(&c);
            assert!(v.is_ok());
            assert_eq!(v.unwrap(), serde_json::Value::String(s.to_string()));
        }
    }

    #[test]
    fn range() {
        let r: Range<u32> = Range::None;
        let v = serde_json::to_value(&r);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), json!({}));

        let r: Range<u32> = Range::Min(23);
        let v = serde_json::to_value(&r);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), json!({"MIN": 23}));

        let r: Range<f32> = Range::Max(100f32);
        let v = serde_json::to_value(&r);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), json!({"MAX": 100.0}));

        let r: Range<f32> = Range::MinMax(2f32, 100f32);
        let v = serde_json::to_value(&r);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), json!({"MAX": 100.0, "MIN": 2.0}));

        let r: Range<i32> = Range::Vals(vec![-1i32, 2i32]);
        let v = serde_json::to_value(&r);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), json!({"VALS": [-1, 2]}));

        let r: Range<String> = Range::Vals(vec!["x".to_string(), "y".to_string(), "z".to_string()]);
        let v = serde_json::to_value(&r);
        assert!(v.is_ok());
        assert_eq!(v.unwrap(), json!({"VALS": ["x", "y", "z"]}));
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

        let a: Arc<C> = Arc::new(Default::default());
        let b: ValueGetSet<u32> = ValueBuilder::new(a.clone() as _).build();
        assert_eq!(b.value().get(), 0u32);
        b.value().set(20u32);
        assert_eq!(b.value().get(), 20u32);

        //can clone
        let x = b.clone();
        assert_eq!(x.value().get(), 20u32);

        //can also be just a get or set
        let _: ValueGet<u32> = ValueBuilder::new(a.clone() as _).build();
        let _: ValueSet<u32> = ValueBuilder::new(a.clone() as _).build();
    }
}
