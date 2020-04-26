use crate::value::*;

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
