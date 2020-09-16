use crate::{ TecReader};
use libc::c_char;
use std::{
    borrow::Cow,
    convert::From,
    ffi::{c_void, CString, OsStr},
    marker::PhantomData,
    ptr::null_mut,
};
use thiserror::{Error};

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
impl ZoneType {
    pub fn is_fe(&self) -> bool {
        use ZoneType::*;
        match self {
            FELine | FETriangle | FEQuad | FETetra | FEBrick | FEPolygon | FEPolyhedron => true,
            _ => false,
        }
    }
    pub fn num_nodes(&self) -> usize {
        use ZoneType::*;
        match self {
            FELine => 2,
            FETriangle => 3,
            FEQuad => 4,
            FETetra => 4,
            FEBrick => 8,
            _ => 0,
        }
    }
}

impl From<i32> for ZoneType {
    fn from(value: i32) -> Self {
        if value <= 7 && value >= 0 {
            unsafe { std::mem::transmute(value) }
        } else {
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

impl From<i32> for TecDataType {
    fn from(i: i32) -> Self {
        match i {
            1 => Self::F32,
            2 => Self::F64,
            3 => Self::I32,
            4 => Self::I16,
            5 => Self::I8,
            6 => Self::I1,
            _ => panic!("Wrong data type!"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TecData<'a> {
    F64(Cow<'a, [f64]>),
    F32(Cow<'a, [f32]>),
    I64(Cow<'a, [i64]>),
    I32(Cow<'a, [i32]>),
    I16(Cow<'a, [i16]>),
    I8(Cow<'a, [i8]>),
    U64(Cow<'a, [u64]>),
    U32(Cow<'a, [u32]>),
}

impl<'a> TecData<'a> {
    pub(crate) fn get(&self) -> TecData<'a> {
        match self {
            TecData::F64(ref cow) => match cow {
                Cow::Owned(ref owned) => TecData::F64(Cow::Borrowed(unsafe {
                    std::mem::transmute(owned.as_slice())
                })),
                Cow::Borrowed(bor) => TecData::F64(Cow::Borrowed(*bor)),
            },
            TecData::F32(ref cow) => match cow {
                Cow::Owned(ref owned) => TecData::F32(Cow::Borrowed(unsafe {
                    std::mem::transmute(owned.as_slice())
                })),
                Cow::Borrowed(bor) => TecData::F32(Cow::Borrowed(*bor)),
            },
            TecData::I32(ref cow) => match cow {
                Cow::Owned(ref owned) => TecData::I32(Cow::Borrowed(unsafe {
                    std::mem::transmute(owned.as_slice())
                })),
                Cow::Borrowed(bor) => TecData::I32(Cow::Borrowed(*bor)),
            },
            _ => unimplemented!(),
        }
    }
    pub fn len(&self) -> usize {
        use TecData::*;
        match self {
            F64(c) => c.len(),
            F32(c) => c.len(),
            I64(c) => c.len(),
            I32(c) => c.len(),
            I16(c) => c.len(),
            I8(c) => c.len(),
            U64(c) => c.len(),
            U32(c) => c.len(),
        }
    }
    pub fn as_f32(&self) -> Vec<f32>{
        match self{
            TecData::F32(ref cow) => {
                cow.clone().into_owned()
            },
            TecData::F64(ref cow) => {
                cow.iter().map(|v| *v as f32).collect()
            }
            _ => unimplemented!(),
        }
    }

    pub fn as_f64(&self) -> Vec<f64>{
        match self{
            TecData::F32(ref cow) => {
                cow.iter().map(|v| *v as f64).collect()
            },
            TecData::F64(ref cow) => {
                cow.clone().into_owned()
            }
            _ => unimplemented!(),
        }
    }

    pub fn as_i32(&self) -> Vec<i32>{
        match self{
            TecData::I32(ref cow) => {
                cow.clone().into_owned()
            },
            _ => unimplemented!(),
        }
    }
}

macro_rules! borrowed_impl {
    ($ty: tt, $var: tt) => {
        impl<'a> From<&'a [$ty]> for TecData<'a>{
            fn from(s: &'a [$ty]) -> Self {
                Self::$var(Cow::Borrowed(s))
            }
        }
    }
}
macro_rules! owned_impl {
    ($ty: tt, $var: tt) => {
        impl From<Vec<$ty>> for TecData<'static>{
            fn from(s: Vec<$ty>) -> Self {
                Self::$var(Cow::Owned(s))
            }
        }
    }
}

macro_rules! both_impl {
    ($ty: tt, $var: tt) => {
        borrowed_impl!($ty, $var);
        owned_impl!($ty, $var);
    }
}


both_impl!(f64, F64);
both_impl!(f32, F32);
both_impl!(i64, I64);
both_impl!(i32, I32);
both_impl!(i16, I16);
both_impl!(i8 , I8 );
both_impl!(u64, U64);
both_impl!(u32, U32);





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

impl From<i32> for FileType {
    fn from(i: i32) -> Self {
        match i {
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
    Nodal = 1,
}

impl From<i32> for ValueLocation{
    fn from(i: i32) -> Self {
        match i {
            0 => Self::CellCentered,
            1 => Self::Nodal,
            _ => unreachable!()
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum FaceNeighborMode {
    LocalOneToOne = 0,
    LocalOneToMany,
    GlobalOneToOne,
    GlobalOneToMany,
}

impl From<i32> for FaceNeighborMode {
    fn from(value: i32) -> Self {
        if value <= 3 && value >= 0 {
            unsafe { std::mem::transmute(value) }
        } else {
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

#[derive(Debug, Clone)]
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
    pub fn var_locs(&self) -> &[ValueLocation] {
        match self {
            TecZone::Ordered(z) => &z.var_location,
            TecZone::ClassicFE(z) => &z.var_location,
            _ => unimplemented!(),
        }
    }
    pub fn solution_time(&self) -> f64{
        match self{
            TecZone::Ordered(z) => z.solution_time,
            TecZone::ClassicFE(z) => z.solution_time,
            _ => unimplemented!()
        }
    }
    pub fn node_count(&self) -> usize {
        match self {
            TecZone::Ordered(z) => (z.i_max * z.j_max * z.k_max) as _,
            TecZone::ClassicFE(z) => z.nodes as _,
            _ => unimplemented!(),
        }
    }
    pub fn cell_count(&self) -> usize {
        match self {
            TecZone::Ordered(z) => z.cell_count(),
            TecZone::ClassicFE(z) => z.cells as _,
            _ => unimplemented!(),
        }
    }
    pub fn data_types(&self) -> Option<&[TecDataType]> {
        match self {
            TecZone::Ordered(z) => z.var_types.as_ref().map(|v| v.as_slice()),
            TecZone::ClassicFE(z) => z.var_types.as_ref().map(|v| v.as_slice()),
            _ => unimplemented!(),
        }
    }
    pub fn data_types_mut(&mut self) -> &mut Option<Vec<TecDataType>> {
        match self {
            TecZone::Ordered(z) => &mut z.var_types,
            TecZone::ClassicFE(z) => &mut z.var_types,
            _ => unimplemented!(),
        }
    }
}

impl Zone for TecZone {
    fn id(&self) -> i32 {
        match self{
            TecZone::Ordered(z) => z.id(),
            TecZone::ClassicFE(z) => z.id(),
            _ => unimplemented!()
        }
    }
    fn time(&self) -> f64 {
        self.solution_time()
    }
    fn name(&self) -> &str {
        self.name()
    }
}

#[derive(Debug, Clone)]
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
    pub passive_var_list: Vec<i32>,
}

impl OrderedZone {
    fn cell_count(&self) -> usize {
        (if self.i_max != 1 { self.i_max - 1 } else { 1 }
            * if self.j_max != 1 { self.j_max - 1 } else { 1 }
            * if self.k_max != 1 { self.k_max - 1 } else { 1 }) as _
    }
}
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

#[derive(Debug, Clone)]
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

impl ClassicFEZone {
    pub fn num_connections(&self) -> usize {
        self.cells as usize * self.zone_type.num_nodes()
    }
}

impl Zone for ClassicFEZone {
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

#[derive(Debug, Clone)]
pub struct PolyFE {}

pub trait Zone {
    fn id(&self) -> i32;
    fn name(&self) -> &str;
    fn time(&self) -> f64;
}


#[derive(Debug, Copy, Clone, Error)]
pub enum ParseError {
    #[error("Header Version Missing")]
    HeaderVersionMissing,
    #[error("Version mismatch (minimum: {min}, current: {current})")]
    VersionMismatch { min: i32, current: i32 },
    #[error("Utf8 Error")]
    Utf8Error,
    #[error("Unsupported Feature")]
    NotSupportedFeature,
    #[error("Wrong Header Tag")]
    WrongHeaderTag,
    #[error("Wrong data tag")]
    WrongDataTag,
    #[error("Unexpected end of header")]
    EndOfHeader,
    #[error("Nom Error of kind: {}", .0.description())]
    NomError(nom::error::ErrorKind),
}

impl nom::error::ParseError<&[u8]> for ParseError {
    fn from_error_kind(input: &[u8], kind: nom::error::ErrorKind) -> Self {
        println!("{:?} {:?}", input, kind);
        unimplemented!()
    }

    fn append(input: &[u8], kind: nom::error::ErrorKind, other: Self) -> Self {
        unimplemented!()
    }
}

impl nom::error::ParseError<&str> for ParseError {
    fn from_error_kind(input: &str, kind: nom::error::ErrorKind) -> Self {
        //println!("{:?} {:?}", kind, input);
        ParseError::NomError(kind)
    }

    fn append(input: &str, kind: nom::error::ErrorKind, other: Self) -> Self {
        ParseError::NomError(kind)
    }
}


#[derive(Debug, Error)]
pub enum TecioError {
    #[error("{message} (code {code}).")]
    Other { message: String, code: i32 },
    #[error("FFI Error occured")]
    FFIError {},
    #[error("Null Error: {0}")]
    NulError(#[from] std::ffi::NulError),
    #[error("StringError: {0}")]
    StringError(#[from] std::ffi::IntoStringError),
    #[error("Wrong file extension, expected one of: `szplt`, `plt`, `dat`")]
    WrongFileExtension,
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("Error during parsing: {0}")]
    ParseError(#[from] ParseError),
    #[error("Nom Error: {0}")]
    NomErr(#[from] nom::Err<ParseError>),
}
