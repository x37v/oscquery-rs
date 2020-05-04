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

macro_rules! impl_value_ser {
    ($t:ident, $p:ident) => {
        //for serialize just the value
        impl<'a> Serialize for $t<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                match self.0 {
                    $p::Int(v) => serializer.serialize_i32(v.value().get()),
                    $p::Float(v) => serializer.serialize_f32(v.value().get()),
                    $p::String(v) => serializer.serialize_str(&v.value().get()),
                    $p::Time(v) => {
                        let v = v.value().get();
                        let v = (v.0 as u64) << 32 | (v.1 as u64);
                        serializer.serialize_u64(v)
                    }
                    $p::Long(v) => serializer.serialize_i64(v.value().get()),
                    $p::Double(v) => serializer.serialize_f64(v.value().get()),
                    $p::Char(v) => serializer.serialize_char(v.value().get()),
                    $p::Midi(..) => serializer.serialize_none(),
                    $p::Bool(v) => serializer.serialize_bool(v.value().get()),
                }
            }
        }
    };
}

macro_rules! impl_range_ser {
    ($t:ident, $p:ident) => {
        impl<'a> Serialize for $t<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                match self.0 {
                    $p::Int(v) => serializer.serialize_some(v.range()),
                    $p::Float(v) => serializer.serialize_some(v.range()),
                    $p::String(v) => serializer.serialize_some(v.range()),
                    $p::Time(v) => serializer.serialize_some(v.range()),
                    $p::Long(v) => serializer.serialize_some(v.range()),
                    $p::Double(v) => serializer.serialize_some(v.range()),
                    $p::Char(v) => serializer.serialize_some(v.range()),
                    $p::Midi(..) => serializer.serialize_none(),
                    $p::Bool(v) => serializer.serialize_some(v.range()),
                }
            }
        }
    };
}

macro_rules! impl_clip_mode_ser {
    ($t:ident, $p:ident) => {
        impl<'a> Serialize for $t<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                match self.0 {
                    $p::Int(v) => serializer.serialize_some(v.clip_mode()),
                    $p::Float(v) => serializer.serialize_some(v.clip_mode()),
                    $p::String(v) => serializer.serialize_some(v.clip_mode()),
                    $p::Time(v) => serializer.serialize_some(v.clip_mode()),
                    $p::Long(v) => serializer.serialize_some(v.clip_mode()),
                    $p::Double(v) => serializer.serialize_some(v.clip_mode()),
                    $p::Char(v) => serializer.serialize_some(v.clip_mode()),
                    $p::Midi(..) => serializer.serialize_none(),
                    $p::Bool(v) => serializer.serialize_some(v.clip_mode()),
                }
            }
        }
    };
}

macro_rules! impl_unit_ser {
    ($t:ident, $p:ident) => {
        impl<'a> Serialize for $t<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                match self.0 {
                    $p::Int(v) => serializer.serialize_some(v.unit()),
                    $p::Float(v) => serializer.serialize_some(v.unit()),
                    $p::String(v) => serializer.serialize_some(v.unit()),
                    $p::Time(v) => serializer.serialize_some(v.unit()),
                    $p::Long(v) => serializer.serialize_some(v.unit()),
                    $p::Double(v) => serializer.serialize_some(v.unit()),
                    $p::Char(v) => serializer.serialize_some(v.unit()),
                    $p::Midi(..) => serializer.serialize_none(),
                    $p::Bool(v) => serializer.serialize_some(v.unit()),
                }
            }
        }
    };
}

pub(crate) struct ParamGetValueWrapper<'a>(pub(crate) &'a ParamGet);
pub(crate) struct ParamGetSetValueWrapper<'a>(pub(crate) &'a ParamGetSet);

impl_value_ser!(ParamGetValueWrapper, ParamGet);
impl_value_ser!(ParamGetSetValueWrapper, ParamGetSet);

pub(crate) struct ParamGetRangeWrapper<'a>(pub(crate) &'a ParamGet);
pub(crate) struct ParamSetRangeWrapper<'a>(pub(crate) &'a ParamSet);
pub(crate) struct ParamGetSetRangeWrapper<'a>(pub(crate) &'a ParamGetSet);

impl_range_ser!(ParamGetRangeWrapper, ParamGet);
impl_range_ser!(ParamSetRangeWrapper, ParamSet);
impl_range_ser!(ParamGetSetRangeWrapper, ParamGetSet);

pub(crate) struct ParamGetClipModeWrapper<'a>(pub(crate) &'a ParamGet);
pub(crate) struct ParamSetClipModeWrapper<'a>(pub(crate) &'a ParamSet);
pub(crate) struct ParamGetSetClipModeWrapper<'a>(pub(crate) &'a ParamGetSet);

impl_clip_mode_ser!(ParamGetClipModeWrapper, ParamGet);
impl_clip_mode_ser!(ParamSetClipModeWrapper, ParamSet);
impl_clip_mode_ser!(ParamGetSetClipModeWrapper, ParamGetSet);

pub(crate) struct ParamGetUnitWrapper<'a>(pub(crate) &'a ParamGet);
pub(crate) struct ParamSetUnitWrapper<'a>(pub(crate) &'a ParamSet);
pub(crate) struct ParamGetSetUnitWrapper<'a>(pub(crate) &'a ParamGetSet);

impl_unit_ser!(ParamGetUnitWrapper, ParamGet);
impl_unit_ser!(ParamSetUnitWrapper, ParamSet);
impl_unit_ser!(ParamGetSetUnitWrapper, ParamGetSet);

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
