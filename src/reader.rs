use libc::c_char;
use std::{
    convert::From,
    ffi::{c_void, CString, OsStr},
    marker::PhantomData,
    path::Path,
    ptr::null_mut,
};

use crate::{
    bindings,
    common::{try_err, Dataset, OrderedZone, Result, TecDataType, TecZone, TecioError, ZoneType},
    formats::DatFormat,
    reader::InnerReader::SzpltReader,
    ClassicFEZone, PltFormat, SzpltFormat, TecData, ValueLocation,
};

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
        let inner = match path.extension().map(|os| os.to_str().unwrap_or("")) {
            Some("szplt")  | Some("szplt_geom")=> {
                let path = path.to_str().unwrap();
                InnerReader::SzpltReader(SzpltFormat::open(path)?)
            }
            Some("plt") => InnerReader::PltReader(PltFormat::open(path)?),
            Some("dat") => InnerReader::DatReader(DatFormat::open(path)?),
            _ => return Err(TecioError::WrongFileExtension),
        };

        Ok(Self { inner })
    }

    pub fn tecio<P: AsRef<Path>>(path: P) -> Result<Self>{
        let path = path.as_ref().to_str().unwrap();
        Ok(Self{
            inner: InnerReader::SzpltReader(SzpltFormat::open(path)?)
        })
    }

    pub fn dataset(&self) -> &Dataset {
        match &self.inner {
            InnerReader::SzpltReader(szplt) => &szplt.dataset,
            InnerReader::PltReader(plt) => &plt.dataset,
            InnerReader::DatReader(dat) => &dat.dataset,
        }
    }

    pub fn zones(&self) -> &[TecZone] {
        match &self.inner {
            InnerReader::SzpltReader(szplt) => &szplt.zones,
            InnerReader::PltReader(plt) => &plt.zones,
            InnerReader::DatReader(dat) => &dat.zones,
        }
    }

    pub fn get_data(&self, zone_id: usize, var_id: usize) -> Result<TecData> {
        match &self.inner {
            InnerReader::SzpltReader(szplt) => szplt.get_data(zone_id, var_id),
            InnerReader::PltReader(plt) => Ok(plt.data_blocks[zone_id - 1].get_data(var_id - 1)),
            InnerReader::DatReader(dat) => Ok(dat.data_blocks[zone_id - 1].get_data(var_id - 1)),
        }
    }
    pub fn get_var_min_max(&self, zone_id: usize, var_id: usize) -> Option<(f64, f64)> {
        match &self.inner {
            InnerReader::SzpltReader(szplt) => None,
            InnerReader::PltReader(plt) => Some(plt.data_blocks[zone_id - 1].min_max[var_id - 1]),
            InnerReader::DatReader(dat) => None,
        }
    }

    pub fn get_connectivity(&self, zone_id: usize) -> Result<Option<TecData>> {
        match &self.inner {
            InnerReader::SzpltReader(szplt) => szplt.get_connectivity(zone_id as _),
            InnerReader::PltReader(plt) => Ok(plt.data_blocks[zone_id - 1]
                .connectivity
                .as_ref()
                .map(|c| c.get())),
            InnerReader::DatReader(dat) => Ok(dat.data_blocks[zone_id - 1]
                .connectivity
                .as_ref()
                .map(|c| c.get())),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{TecReader, TecioError};
    use std::borrow::Borrow;
    #[test]
    fn test_plt() -> Result<(), TecioError> {
        let plt = TecReader::open("./tests/heated_fin.plt")?;
        let szplt = TecReader::open("./tests/heated_fin.szplt")?;
        let dat = TecReader::open("./tests/heated_fin.dat")?;
        let num_vars = plt.dataset().num_variables as usize;
        for (i, z) in plt.zones().iter().enumerate() {
            for v in 1..=num_vars {
                let p = plt.get_data(i + 1, v)?;
                let s = szplt.get_data(i + 1, v)?;
                let d = dat.get_data(i + 1, v)?;

                assert_eq!(s.borrow(), p.borrow(), "Data in zone {}, var {}, is not equal", i + 1, v);
                s.borrow().as_f64().into_iter().zip(d.borrow().as_f64().into_iter()).for_each(|(s, d)|{
                    assert!((s - d).abs() < 1e-5, "|s - d| = {:?}", (s - d).abs());
                })
               //assert_eq!(s.borrow(), d.borrow(), "Data in zone {}, var {}, is not equal", i + 1, v);
            }
            let p = plt.get_connectivity(i + 1)?;
            let s = szplt.get_connectivity(i + 1)?;
            let d = szplt.get_connectivity(i + 1)?;
            assert_eq!(
                s.as_ref().map(|v| v.borrow()),
                p.as_ref().map(|v| v.borrow()),
                "Connectivity is not equal"
            );
            assert_eq!(
                s.as_ref().map(|v| v.borrow()),
                d.as_ref().map(|v| v.borrow()),
                "Connectivity is not equal"
            );
        }
        Ok(())
    }
}
