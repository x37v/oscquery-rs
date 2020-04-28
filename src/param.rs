use crate::value::*;
use serde::{Serialize, Serializer};

pub trait OSCTypeStr {
    fn osc_type_str(&self) -> &'static str;
}

#[derive(Debug)]
pub enum ParamGet {
    Int(ValueGet<i32>),
    Float(ValueGet<f32>),
    String(ValueGet<String>),
    Time(ValueGet<(u32, u32)>),
    Long(ValueGet<i64>),
    Double(ValueGet<f64>),
    Char(ValueGet<char>),
    Midi(ValueGet<(u8, u8, u8, u8)>),
    Bool(ValueGet<bool>),
    //TODO Blob(ValueGet<Box<[u8]>>), //does clip mode make and range make sense?
    //TODO Array(Box<[Self]>),
    //TODO Nil,
    //TODO Inf,
}

#[derive(Debug)]
pub enum ParamSet {
    Int(ValueSet<i32>),
    Float(ValueSet<f32>),
    String(ValueSet<String>),
    Time(ValueSet<(u32, u32)>),
    Long(ValueSet<i64>),
    Double(ValueSet<f64>),
    Char(ValueSet<char>),
    Midi(ValueSet<(u8, u8, u8, u8)>),
    Bool(ValueSet<bool>),
    //TODO Blob(ValueSet<Box<[u8]>>), //does clip mode make and range make sense?
    //TODO Array(Box<[Self]>),
}

#[derive(Debug)]
pub enum ParamGetSet {
    Int(ValueGetSet<i32>),
    Float(ValueGetSet<f32>),
    String(ValueGetSet<String>),
    Time(ValueGetSet<(u32, u32)>),
    Long(ValueGetSet<i64>),
    Double(ValueGetSet<f64>),
    Char(ValueGetSet<char>),
    Midi(ValueGetSet<(u8, u8, u8, u8)>),
    Bool(ValueGetSet<bool>),
    //TODO Blob(ValueGetSet<Box<[u8]>>), //does clip mode make and range make sense?
    //TODO Array(Box<[Self]>),
}

impl OSCTypeStr for ParamGet {
    fn osc_type_str(&self) -> &'static str {
        match self {
            Self::Int(..) => &"i",
            Self::Float(..) => &"f",
            Self::String(..) => &"s",
            Self::Time(..) => &"t",
            Self::Long(..) => &"h",
            Self::Double(..) => &"d",
            Self::Char(..) => &"c",
            Self::Midi(..) => &"m",
            Self::Bool(v) => {
                if v.value().get() {
                    &"T"
                } else {
                    &"F"
                }
            }
        }
    }
}

//for serialize just the value
pub(crate) struct ParamGetValueWrapper<'a>(pub(crate) &'a ParamGet);
impl<'a> Serialize for ParamGetValueWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamGet::Int(v) => serializer.serialize_i32(v.value().get()),
            ParamGet::Float(v) => serializer.serialize_f32(v.value().get()),
            ParamGet::String(v) => serializer.serialize_str(&v.value().get()),
            ParamGet::Time(v) => {
                let v = v.value().get();
                let v = (v.0 as u64) << 32 | (v.1 as u64);
                serializer.serialize_u64(v)
            }
            ParamGet::Long(v) => serializer.serialize_i64(v.value().get()),
            ParamGet::Double(v) => serializer.serialize_f64(v.value().get()),
            ParamGet::Char(v) => serializer.serialize_char(v.value().get()),
            ParamGet::Midi(..) => serializer.serialize_none(),
            ParamGet::Bool(v) => serializer.serialize_bool(v.value().get()),
        }
    }
}

pub(crate) struct ParamGetSetValueWrapper<'a>(pub(crate) &'a ParamGetSet);
impl<'a> Serialize for ParamGetSetValueWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamGetSet::Int(v) => serializer.serialize_i32(v.value().get()),
            ParamGetSet::Float(v) => serializer.serialize_f32(v.value().get()),
            ParamGetSet::String(v) => serializer.serialize_str(&v.value().get()),
            ParamGetSet::Time(v) => {
                let v = v.value().get();
                let v = (v.0 as u64) << 32 | (v.1 as u64);
                serializer.serialize_u64(v)
            }
            ParamGetSet::Long(v) => serializer.serialize_i64(v.value().get()),
            ParamGetSet::Double(v) => serializer.serialize_f64(v.value().get()),
            ParamGetSet::Char(v) => serializer.serialize_char(v.value().get()),
            ParamGetSet::Midi(..) => serializer.serialize_none(),
            ParamGetSet::Bool(v) => serializer.serialize_bool(v.value().get()),
        }
    }
}

pub(crate) struct ParamGetRangeWrapper<'a>(pub(crate) &'a ParamGet);
impl<'a> Serialize for ParamGetRangeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamGet::Int(v) => serializer.serialize_some(v.range()),
            ParamGet::Float(v) => serializer.serialize_some(v.range()),
            ParamGet::String(v) => serializer.serialize_some(v.range()),
            ParamGet::Time(v) => serializer.serialize_some(v.range()),
            ParamGet::Long(v) => serializer.serialize_some(v.range()),
            ParamGet::Double(v) => serializer.serialize_some(v.range()),
            ParamGet::Char(v) => serializer.serialize_some(v.range()),
            ParamGet::Midi(..) => serializer.serialize_none(),
            ParamGet::Bool(v) => serializer.serialize_some(v.range()),
        }
    }
}

pub(crate) struct ParamSetRangeWrapper<'a>(pub(crate) &'a ParamSet);
impl<'a> Serialize for ParamSetRangeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamSet::Int(v) => serializer.serialize_some(v.range()),
            ParamSet::Float(v) => serializer.serialize_some(v.range()),
            ParamSet::String(v) => serializer.serialize_some(v.range()),
            ParamSet::Time(v) => serializer.serialize_some(v.range()),
            ParamSet::Long(v) => serializer.serialize_some(v.range()),
            ParamSet::Double(v) => serializer.serialize_some(v.range()),
            ParamSet::Char(v) => serializer.serialize_some(v.range()),
            ParamSet::Midi(..) => serializer.serialize_none(),
            ParamSet::Bool(v) => serializer.serialize_some(v.range()),
        }
    }
}

pub(crate) struct ParamGetSetRangeWrapper<'a>(pub(crate) &'a ParamGetSet);
impl<'a> Serialize for ParamGetSetRangeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamGetSet::Int(v) => serializer.serialize_some(v.range()),
            ParamGetSet::Float(v) => serializer.serialize_some(v.range()),
            ParamGetSet::String(v) => serializer.serialize_some(v.range()),
            ParamGetSet::Time(v) => serializer.serialize_some(v.range()),
            ParamGetSet::Long(v) => serializer.serialize_some(v.range()),
            ParamGetSet::Double(v) => serializer.serialize_some(v.range()),
            ParamGetSet::Char(v) => serializer.serialize_some(v.range()),
            ParamGetSet::Midi(..) => serializer.serialize_none(),
            ParamGetSet::Bool(v) => serializer.serialize_some(v.range()),
        }
    }
}

pub(crate) struct ParamGetClipModeWrapper<'a>(pub(crate) &'a ParamGet);
impl<'a> Serialize for ParamGetClipModeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamGet::Int(v) => serializer.serialize_some(v.clip_mode()),
            ParamGet::Float(v) => serializer.serialize_some(v.clip_mode()),
            ParamGet::String(v) => serializer.serialize_some(v.clip_mode()),
            ParamGet::Time(v) => serializer.serialize_some(v.clip_mode()),
            ParamGet::Long(v) => serializer.serialize_some(v.clip_mode()),
            ParamGet::Double(v) => serializer.serialize_some(v.clip_mode()),
            ParamGet::Char(v) => serializer.serialize_some(v.clip_mode()),
            ParamGet::Midi(..) => serializer.serialize_none(),
            ParamGet::Bool(v) => serializer.serialize_some(v.clip_mode()),
        }
    }
}

pub(crate) struct ParamSetClipModeWrapper<'a>(pub(crate) &'a ParamSet);
impl<'a> Serialize for ParamSetClipModeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamSet::Int(v) => serializer.serialize_some(v.clip_mode()),
            ParamSet::Float(v) => serializer.serialize_some(v.clip_mode()),
            ParamSet::String(v) => serializer.serialize_some(v.clip_mode()),
            ParamSet::Time(v) => serializer.serialize_some(v.clip_mode()),
            ParamSet::Long(v) => serializer.serialize_some(v.clip_mode()),
            ParamSet::Double(v) => serializer.serialize_some(v.clip_mode()),
            ParamSet::Char(v) => serializer.serialize_some(v.clip_mode()),
            ParamSet::Midi(..) => serializer.serialize_none(),
            ParamSet::Bool(v) => serializer.serialize_some(v.clip_mode()),
        }
    }
}

pub(crate) struct ParamGetSetClipModeWrapper<'a>(pub(crate) &'a ParamGetSet);
impl<'a> Serialize for ParamGetSetClipModeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ParamGetSet::Int(v) => serializer.serialize_some(v.clip_mode()),
            ParamGetSet::Float(v) => serializer.serialize_some(v.clip_mode()),
            ParamGetSet::String(v) => serializer.serialize_some(v.clip_mode()),
            ParamGetSet::Time(v) => serializer.serialize_some(v.clip_mode()),
            ParamGetSet::Long(v) => serializer.serialize_some(v.clip_mode()),
            ParamGetSet::Double(v) => serializer.serialize_some(v.clip_mode()),
            ParamGetSet::Char(v) => serializer.serialize_some(v.clip_mode()),
            ParamGetSet::Midi(..) => serializer.serialize_none(),
            ParamGetSet::Bool(v) => serializer.serialize_some(v.clip_mode()),
        }
    }
}

impl OSCTypeStr for ParamSet {
    fn osc_type_str(&self) -> &'static str {
        match self {
            Self::Int(..) => &"i",
            Self::Float(..) => &"f",
            Self::String(..) => &"s",
            Self::Time(..) => &"t",
            Self::Long(..) => &"h",
            Self::Double(..) => &"d",
            Self::Char(..) => &"c",
            Self::Midi(..) => &"m",
            Self::Bool(..) => &"T",
        }
    }
}

impl OSCTypeStr for ParamGetSet {
    fn osc_type_str(&self) -> &'static str {
        match self {
            Self::Int(..) => &"i",
            Self::Float(..) => &"f",
            Self::String(..) => &"s",
            Self::Time(..) => &"t",
            Self::Long(..) => &"h",
            Self::Double(..) => &"d",
            Self::Char(..) => &"c",
            Self::Midi(..) => &"m",
            Self::Bool(v) => {
                if v.value().get() {
                    &"T"
                } else {
                    &"F"
                }
            }
        }
    }
}
