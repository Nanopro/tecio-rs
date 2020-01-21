use crate::{bindings, ClassicFEZone, ValueLocation, SzpltFormat, PltFormat, TecData};
use libc::c_char;
use std::{
    convert::From,
    ffi::{CString, OsStr, c_void},
    ptr::null_mut,
    path::{Path},
};

use crate::common::{
    try_err, Dataset, OrderedZone, Result, TecDataType, TecZone, TecioError, ZoneType,
};
use std::marker::PhantomData;
use crate::reader::InnerReader::SzpltReader;


pub struct TecReader {
    inner: InnerReader,
}

pub enum InnerReader {
    PltReader(PltFormat),
    SzpltReader(SzpltFormat),
}

impl TecReader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let inner = match path.extension().map(|os| os.to_str().unwrap_or("")){
            Some("szplt") => {
                let path = path.to_str().unwrap();
                InnerReader::SzpltReader(SzpltFormat::open(path)?)
            },
            Some("plt") => {
                InnerReader::PltReader(PltFormat::open(path)?)
            },
            _ => {
                return Err(
                    TecioError::WrongFileExtension
                )
            }
        };

        Ok(
            Self{
                inner,
            }
        )
    }

    pub fn dataset(&self) -> &Dataset{
        match &self.inner{
            InnerReader::SzpltReader(szplt) => &szplt.dataset,
            InnerReader::PltReader(plt) => unimplemented!(),
        }
    }

    pub fn zones(&self) -> &[TecZone] {
        match &self.inner{
            InnerReader::SzpltReader(szplt) => &szplt.zones,
            InnerReader::PltReader(plt) => unimplemented!(),
        }
    }

    pub fn get_data(&self) -> TecData{
        match &self.inner{
            InnerReader::SzpltReader(szplt) => unimplemented!(),
            InnerReader::PltReader(plt) => unimplemented!(),
        }
    }

    pub fn get_connectivity(&self) -> TecData{
        match &self.inner{
            InnerReader::SzpltReader(szplt) => unimplemented!(),
            InnerReader::PltReader(plt) => unimplemented!(),
        }
    }
}
