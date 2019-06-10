use libc::{c_void, c_char};
use std::ffi::{OsStr, CString};
use std::ptr::null_mut;
use std::convert::From;
use crate::TecReader;

use std::marker::PhantomData;


pub type Result<T> = std::result::Result<T, TecioError>;


#[derive(Debug, PartialEq, Copy,Clone)]
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
        Err(TecioError{
            message: message.to_string(),
            code: er,
        })
    } else{
        Ok(())
    }

}





#[derive(Debug)]
pub struct Dataset{
    pub num_variables: i32,

    pub num_zones: i32,
    pub title: String,

    pub var_names: Vec<String>,
    //pub zone_names: Vec<String>,
}

impl Dataset{
    pub fn empty()->Self{
        Dataset{
            num_variables: 0,
            num_zones: 0,
            title: "".to_string(),
            var_names: vec![],
            //zone_names: vec![]
        }
    }
}


#[derive(Debug)]
pub enum TecZone{
    Ordered(OrderedZone),
    ClassicFE(ClassicFEZone),
    PolyFE(PolyFE),
}

impl TecZone{
    pub fn name(&self) -> &str{
        match self {
            TecZone::Ordered(oz) => {
               &oz.name
            },
            TecZone::ClassicFE(fe) =>{
                &fe.name
            },
            _ => unimplemented!()
        }
    }

    pub fn zone_type(&self) -> ZoneType{
        match self {
            TecZone::Ordered(_) => {
                ZoneType::Ordered
            },
            TecZone::ClassicFE(fe) =>{
                fe.zone_type
            },
            _ => unimplemented!()
        }
    }



}

#[derive(Debug)]
pub struct OrderedZone{
    pub name: String,
    pub id: i32,



    pub solution_time: f64,
    pub strand: i32,
    pub i_max: i64,
    pub j_max: i64,
    pub k_max: i64,


}



impl OrderedZone{}
impl Zone for OrderedZone{
    fn id(&self) -> i32{
        self.id
    }
    fn time(&self) -> f64{
        self.solution_time
    }
    fn name(&self) -> &str{
        &self.name
    }
}





#[derive(Debug)]
pub struct ClassicFEZone{
    pub name: String,
    pub zone_type: ZoneType,
    pub id: i32,
    pub solution_time: f64,
    pub strand: i32,
}
#[derive(Debug)]
pub struct PolyFE{

}

pub trait Zone{
    fn id(&self) -> i32;
    fn name(&self) -> &str;
    fn time(&self) -> f64;
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



impl From<std::ffi::IntoStringError> for TecioError{
    fn from(t: std::ffi::IntoStringError) -> Self{
        TecioError{
            message: "File name contains some characters, cannot convert to String".to_owned(),
            code: -1,
        }
    }
}



