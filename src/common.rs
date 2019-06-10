use libc::{c_void, c_char};
use std::ffi::{OsStr, CString};
use std::ptr::null_mut;
use std::convert::From;



#[derive(Debug, PartialEq)]
pub enum ZoneType{
    Ordered = 0,
    FELine = 1,
    FETriangle = 2,
    FEQuad = 3,
    FETetra = 4,
    FEBrick = 5,
    FEPolygon = 6,
    FEPolyhedron = 7,
}



pub struct TecZone{
    pub name: String,
    pub zone_type: ZoneType,
    pub time: f64,
    pub id: i32,
    pub i_max: i64,
    pub j_max: i64,
    pub k_max: i64,
}



#[derive(Debug)]
pub struct TecioError {
    pub message: String,
    pub code: i32,
}

impl From<std::ffi::NulError> for TecioError{
    fn from(t: std::ffi::NulError) -> Self{
        TecioError{
            message: "File name contains null characters, cannot convert to CString".to_owned(),
            code: -1,
        }
    }
}
pub type Result<T> = std::result::Result<T, TecioError>;







