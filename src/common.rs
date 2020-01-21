use crate::{TecReader, PltParseError};
use libc::c_char;
use std::convert::From;
use std::ffi::{c_void, CString, OsStr};
use std::ptr::null_mut;

use std::borrow::Cow;
use std::marker::PhantomData;

pub type Result<T> = std::result::Result<T, TecioError>;

#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(i32)]
pub enum ZoneType {
    Ordered = 0,
    FELine = 1,
    FETriangle = 2,
    FEQuad = 3,
    FETetra = 4,
    FEBrick = 5,
    FEPolygon = 6,
    FEPolyhedron = 7,
}
impl ZoneType{
    pub fn is_fe(&self) -> bool {
        use ZoneType::*;
        match self{
            FELine | FETriangle | FEQuad | FETetra | FEBrick | FEPolygon | FEPolyhedron  => true,
            _ => false
        }
    }
}

impl From<i32> for ZoneType{
    fn from(value: i32) -> Self {
        if value <= 7 && value >= 0{
            unsafe { std::mem::transmute(value) }
        }else{
            panic!("Wrong integer for zone type!")
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum TecDataType {
    F32 = 1,
    F64 = 2,
    I32 = 3,
    I16 = 4,
    I8 = 5,
    I1 = 6,
}
#[derive(Debug, Clone)]
pub enum TecData<'a> {
    F64(Cow<'a, [f64]>),
    F32(Cow<'a, [f32]>),
    I64(Cow<'a, [i64]>),
    I32(Cow<'a, [i32]>),
    I16(Cow<'a, [i16]>),
    I8(Cow<'a, [i8]>),
}

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum FileFormat {
    Binary = 0,
    Subzone = 1,
}

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum FileType {
    Full,
    GridOnly,
    SolutionOnly(*mut c_void),
}

impl From<i32> for FileType{
    fn from(i: i32) -> Self {
        match i{
            0 => Self::Full,
            1 => Self::GridOnly,
            2 => Self::SolutionOnly(null_mut()),
            _ => panic!("Wrong file type"),
        }
    }
}

impl FileType {
    pub fn as_i32(&self) -> i32 {
        match self {
            FileType::Full => 0,
            FileType::GridOnly => 1,
            FileType::SolutionOnly(_) => 2,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum ValueLocation {
    CellCentered = 0,
    Nodal,
}

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum FaceNeighborMode {
    LocalOneToOne = 0,
    LocalOneToMany,
    GlobalOneToOne,
    GlobalOneToMany,
}

impl From<i32> for FaceNeighborMode{
    fn from(value: i32) -> Self {
        if value <= 3 && value >= 0{
            unsafe { std::mem::transmute(value) }
        }else{
            panic!("Wrong integer for FaceNeighborMode!")
        }
    }
}
/*pub struct TecZone{
    pub name: String,
    pub zone_type: ZoneType,
    pub time: f64,
    pub id: i32,
    pub i_max: i64,
    pub j_max: i64,
    pub k_max: i64,
}*/

pub fn try_err<S: ToString>(er: i32, message: S) -> Result<()> {
    if er != 0 {
        Err(TecioError::Other {
            message: message.to_string(),
            code: er,
        })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Dataset {
    pub num_variables: i32,

    pub num_zones: i32,
    pub title: String,

    pub var_names: Vec<String>,
    //pub zone_names: Vec<String>,
}

impl Dataset {
    pub fn empty() -> Self {
        Dataset {
            num_variables: 0,
            num_zones: 0,
            title: "".to_string(),
            var_names: vec![],
            //zone_names: vec![]
        }
    }
}

#[derive(Debug)]
pub enum TecZone {
    Ordered(OrderedZone),
    ClassicFE(ClassicFEZone),
    PolyFE(PolyFE),
}

impl TecZone {
    pub fn name(&self) -> &str {
        match self {
            TecZone::Ordered(oz) => &oz.name,
            TecZone::ClassicFE(fe) => &fe.name,
            _ => unimplemented!(),
        }
    }

    pub fn zone_type(&self) -> ZoneType {
        match self {
            TecZone::Ordered(_) => ZoneType::Ordered,
            TecZone::ClassicFE(fe) => fe.zone_type,
            _ => unimplemented!(),
        }
    }

    pub fn is_fe(&self) -> bool {
        match self {
            TecZone::ClassicFE(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct OrderedZone {
    pub name: String,
    pub id: i32,

    pub solution_time: f64,
    pub strand: i32,
    pub i_max: i64,
    pub j_max: i64,
    pub k_max: i64,
    pub var_location: Vec<ValueLocation>,
    pub var_types: Option<Vec<TecDataType>>,
}

impl OrderedZone {}
impl Zone for OrderedZone {
    fn id(&self) -> i32 {
        self.id
    }
    fn time(&self) -> f64 {
        self.solution_time
    }
    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug)]
pub struct ClassicFEZone {
    pub name: String,
    pub zone_type: ZoneType,
    pub id: i32,
    pub solution_time: f64,
    pub strand: i32,

    pub nodes: i64,
    pub cells: i64,

    pub var_location: Vec<ValueLocation>,
    pub var_types: Option<Vec<TecDataType>>,
}
#[derive(Debug)]
pub struct PolyFE {}

pub trait Zone {
    fn id(&self) -> i32;
    fn name(&self) -> &str;
    fn time(&self) -> f64;
}

#[derive(Debug)]
pub enum TecioError{
    Other {
        message: String,
        code: i32,
    },
    FFIError {

    },
    NulError(std::ffi::NulError),
    StringError(std::ffi::IntoStringError),
    WrongFileExtension,
    IOError(std::io::Error),
    ParseError(PltParseError),
    NomErr(nom::Err<PltParseError>),
}

impl From<PltParseError> for TecioError {
    fn from(t: PltParseError) -> Self {
        TecioError::ParseError(t)
    }
}
impl From<nom::Err<PltParseError>> for TecioError{
    fn from(e: nom::Err<PltParseError>) -> Self {
        Self::NomErr(e)
    }
}

impl From<std::io::Error> for TecioError {
    fn from(t: std::io::Error) -> Self {
        TecioError::IOError(t)
    }
}

impl From<std::ffi::NulError> for TecioError {
    fn from(t: std::ffi::NulError) -> Self {
        TecioError::NulError(t)
    }
}

impl From<std::ffi::IntoStringError> for TecioError {
    fn from(t: std::ffi::IntoStringError) -> Self {
        TecioError::StringError(t)
    }
}
