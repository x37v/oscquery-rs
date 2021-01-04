//! Node Parameters.
use crate::{
    osc::{OscArray, OscColor, OscMidiMessage, OscType},
    value::*,
};
use serde::{ser::SerializeSeq, Serialize, Serializer};

pub(crate) trait OSCTypeStr {
    fn osc_type_str(&self) -> String;
}

/// read-only parameters
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
    Array(ValueGet<OscArray>),
    //TODO Nil,
    //TODO Inf,
}

/// write-only parameters
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
    Array(ValueSet<OscArray>),
    //TODO Blob(ValueSet<Box<[u8]>>), //does clip mode make and range make sense?
}

/// read-write parameters
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
    Array(ValueGetSet<OscArray>),
    //TODO Blob(ValueGetSet<Box<[u8]>>), //does clip mode make and range make sense?
    //TODO Array(Box<[Self]>),
}

pub(crate) struct OscTypeWrapper<'a>(pub(crate) &'a OscType);
impl<'a> Serialize for OscTypeWrapper<'a> {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            OscType::Int(v) => ser.serialize_i32(*v),
            OscType::Float(v) => ser.serialize_f32(*v),
            OscType::String(v) => ser.serialize_str(v),
            OscType::Blob(_v) => ser.serialize_none(),
            OscType::Time(v) => ser.serialize_u64((v.0 as u64) << 32 | (v.1 as u64)),
            OscType::Long(v) => ser.serialize_i64(*v),
            OscType::Double(v) => ser.serialize_f64(*v),
            OscType::Char(v) => ser.serialize_char(*v),
            OscType::Color(OscColor {
                red,
                green,
                blue,
                alpha,
            }) => ser.serialize_str(
                format!("#{:02X}{:02X}{:02X}{:02X}", red, green, blue, alpha).as_str(),
            ),
            OscType::Midi(_v) => ser.serialize_none(),
            OscType::Bool(v) => ser.serialize_bool(*v),
            OscType::Array(v) => {
                let mut seq = ser.serialize_seq(Some(v.content.len()))?;
                for i in &v.content {
                    seq.serialize_element(&OscTypeWrapper(&i))?;
                }
                seq.end()
            }
            OscType::Nil => ser.serialize_none(),
            OscType::Inf => ser.serialize_none(),
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
                let v = match self.0 {
                    $p::Int(v) => OscType::Int(v.value().get()),
                    $p::Float(v) => OscType::Float(v.value().get()),
                    $p::String(v) => OscType::String(v.value().get()),
                    $p::Time(v) => OscType::Time(v.value().get()),
                    $p::Long(v) => OscType::Long(v.value().get()),
                    $p::Double(v) => OscType::Double(v.value().get()),
                    $p::Char(v) => OscType::Char(v.value().get()),
                    $p::Midi(v) => {
                        let v = v.value().get();
                        OscType::Midi(OscMidiMessage {
                            port: v.0,
                            status: v.1,
                            data1: v.2,
                            data2: v.3,
                        })
                    }
                    $p::Bool(v) => OscType::Bool(v.value().get()),
                    $p::Array(v) => OscType::Array(v.value().get()),
                };
                let w = OscTypeWrapper(&v);
                w.serialize(serializer)
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
                    $p::Array(..) => {
                        let mut seq = serializer.serialize_seq(Some(1))?;
                        seq.serialize_element(&Range::<()>::None)?;
                        seq.end()
                    }
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
                    $p::Array(..) => {
                        let mut seq = serializer.serialize_seq(Some(1))?;
                        seq.serialize_element(&ClipMode::None)?;
                        seq.end()
                    }
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
                    $p::Array(..) => {
                        let mut seq = serializer.serialize_seq(Some(1))?;
                        seq.serialize_element(&Option::<()>::None)?;
                        seq.end()
                    }
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

impl OSCTypeStr for OscType {
    fn osc_type_str(&self) -> String {
        match self {
            OscType::Int(_) => "i".to_string(),
            OscType::Float(_) => "f".to_string(),
            OscType::String(_) => "s".to_string(),
            OscType::Blob(_) => "b".to_string(),
            OscType::Time(_) => "t".to_string(),
            OscType::Long(_) => "h".to_string(),
            OscType::Double(_) => "d".to_string(),
            OscType::Char(_) => "c".to_string(),
            OscType::Color(_) => "r".to_string(),
            OscType::Midi(_) => "m".to_string(),
            OscType::Bool(v) => if *v { "T" } else { "F" }.to_string(),
            OscType::Array(v) => {
                let mut s = String::from("[");
                for i in &v.content {
                    s.push_str(&i.osc_type_str());
                }
                s.push(']');
                s
            }
            OscType::Nil => "N".to_string(),
            OscType::Inf => "I".to_string(),
        }
    }
}

impl OSCTypeStr for ParamGet {
    fn osc_type_str(&self) -> String {
        match self {
            Self::Int(..) => OscType::Int(Default::default()),
            Self::Float(..) => OscType::Float(Default::default()),
            Self::String(..) => OscType::String(Default::default()),
            Self::Time(..) => OscType::Time(Default::default()),
            Self::Long(..) => OscType::Long(Default::default()),
            Self::Double(..) => OscType::Double(Default::default()),
            Self::Char(..) => OscType::Char(Default::default()),
            Self::Midi(..) => OscType::Midi(OscMidiMessage {
                port: 0,
                status: 0x80,
                data1: 0,
                data2: 0,
            }),
            Self::Bool(v) => OscType::Bool(v.value().get()),
            Self::Array(v) => OscType::Array(v.value().get()),
        }
        .osc_type_str()
    }
}

impl OSCTypeStr for ParamSet {
    fn osc_type_str(&self) -> String {
        match self {
            Self::Int(..) => OscType::Int(Default::default()),
            Self::Float(..) => OscType::Float(Default::default()),
            Self::String(..) => OscType::String(Default::default()),
            Self::Time(..) => OscType::Time(Default::default()),
            Self::Long(..) => OscType::Long(Default::default()),
            Self::Double(..) => OscType::Double(Default::default()),
            Self::Char(..) => OscType::Char(Default::default()),
            Self::Midi(..) => OscType::Midi(OscMidiMessage {
                port: 0,
                status: 0x80,
                data1: 0,
                data2: 0,
            }),
            Self::Bool(_) => OscType::Bool(false),
            Self::Array(_) => OscType::Array(OscArray { content: vec![] }),
        }
        .osc_type_str()
    }
}

impl OSCTypeStr for ParamGetSet {
    fn osc_type_str(&self) -> String {
        match self {
            Self::Int(..) => OscType::Int(Default::default()),
            Self::Float(..) => OscType::Float(Default::default()),
            Self::String(..) => OscType::String(Default::default()),
            Self::Time(..) => OscType::Time(Default::default()),
            Self::Long(..) => OscType::Long(Default::default()),
            Self::Double(..) => OscType::Double(Default::default()),
            Self::Char(..) => OscType::Char(Default::default()),
            Self::Midi(..) => OscType::Midi(OscMidiMessage {
                port: 0,
                status: 0x80,
                data1: 0,
                data2: 0,
            }),
            Self::Bool(v) => OscType::Bool(v.value().get()),
            Self::Array(v) => OscType::Array(v.value().get()),
        }
        .osc_type_str()
    }
}
