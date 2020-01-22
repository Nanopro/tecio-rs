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
use crate::formats::DatFormat;


pub struct TecReader {
    inner: InnerReader,
}

pub enum InnerReader {
    PltReader(PltFormat),
    SzpltReader(SzpltFormat),
    DatReader(DatFormat),
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
            InnerReader::PltReader(plt) => &plt.dataset,
            InnerReader::DatReader(dat) => unimplemented!(),
        }
    }

    pub fn zones(&self) -> &[TecZone] {
        match &self.inner{
            InnerReader::SzpltReader(szplt) => &szplt.zones,
            InnerReader::PltReader(plt) => &plt.zones,
            InnerReader::DatReader(dat) => unimplemented!(),
        }
    }

    pub fn get_data(&self, zone_id: usize, var_id: usize) -> Result<TecData>{
        match &self.inner{
            InnerReader::SzpltReader(szplt) => {
                szplt.get_data(zone_id, var_id)
            },
            InnerReader::PltReader(plt) => {
                Ok(plt.data_blocks[zone_id - 1].get_data(var_id - 1))
            },
            InnerReader::DatReader(dat) => unimplemented!(),
        }
    }
    pub fn get_var_min_max(&self, zone_id: usize, var_id: usize) -> Option<(f64, f64)>{
        match &self.inner{
            InnerReader::SzpltReader(szplt) => {
                None
            },
            InnerReader::PltReader(plt) => {
                Some(plt.data_blocks[zone_id - 1].min_max[var_id - 1])
            },
            InnerReader::DatReader(dat) => unimplemented!(),
        }
    }

    pub fn get_connectivity(&self, zone_id: usize) -> Result<Option<TecData>>{
        match &self.inner{
            InnerReader::SzpltReader(szplt) => {
                szplt.get_connectivity(zone_id as _)
            },
            InnerReader::PltReader(plt) => {
                Ok(plt.data_blocks[zone_id - 1].connectivity.as_ref().map(|c| c.get()))
            },
            InnerReader::DatReader(dat) => unimplemented!(),
        }
    }
}


#[cfg(test)]
mod tests{
    use crate::{TecReader, TecioError};
    use std::borrow::{Borrow};
    #[test]
    fn test_plt() -> Result<(), TecioError>{
        let plt = TecReader::open("./tests/heated_fin.plt")?;
        let szplt = TecReader::open("./tests/heated_fin.szplt")?;
        let num_vars = plt.dataset().num_variables as usize;
        for (i, z) in plt.zones().iter().enumerate(){
            for v in 1..=num_vars{
                let p = plt.get_data(i + 1, v)?;
                let s = szplt.get_data(i + 1, v)?;

                assert_eq!(s.borrow(), p.borrow());
            }
            let p = plt.get_connectivity(i + 1)?;
            let s = szplt.get_connectivity(i + 1)?;
            assert_eq!(s.as_ref().map(|v|v.borrow()), p.as_ref().map(|v| v.borrow()));
        }
        Ok(())
    }
}