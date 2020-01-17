use libc::{c_void, c_char};
use std::ffi::{OsStr, CString};
use std::ptr::null_mut;
use std::convert::From;
use crate::bindings;
use crate::common::{ZoneType, TecioError, Result};


pub struct TecWriter{
    file_handle: *mut c_void,
    num_vars: usize,
}





impl TecWriter{
    pub fn create<'a, T>(file: T, dataset_title:T,var_list:T, num_vars: usize,) -> Result<Self>
        where T: AsRef<&'a [u8]>{

        let cname = CString::new::<T>(file)?;
        let dataset_title = CString::new::<T>(dataset_title)?;
        let var_list = CString::new::<T>(var_list)?;

        let mut file_handle = null_mut();


        let mut er =  unsafe{
            bindings::tecFileWriterOpen(
                cname.as_ptr(),
                dataset_title.as_ptr(),
                var_list.as_ptr(),
                1, //szplt
                0, //fullfile
                1, //default --- float
                null_mut(),
                &mut file_handle,
            )
        };

        if er != 0{
            return Err(TecioError{
                message:"Error opening file.".to_owned(),
                code: er,
            });
        }

       // er = unsafe{bindings::tecFileSetDiagnosticsLevel(file_handle, 1)};

        if er != 0{
            return Err(TecioError{
                message:"Error opening file.".to_owned(),
                code: er,
            });
        }

        Ok(Self{
            file_handle,
            num_vars
        })


    }



    pub fn add_fe_zone<'a, T>(&mut self, title: T, zone_type: ZoneType, nodes: i64, cells: i64, time: f64, strand_id: i32) -> Result<i32>
        where T: AsRef<&'a [u8]>{
        let title = CString::new::<T>(title)?;
        let mut zone = 0;

        let var_types = (0..self.num_vars).map(|_| 1).collect::<Vec<_>>();
        let var_share = (0..self.num_vars).map(|_| 0).collect::<Vec<_>>();
        let passive_var_list = (0..self.num_vars).map(|_| 0).collect::<Vec<_>>();
        let value_locs = (0..self.num_vars).map(|_| 1).collect::<Vec<_>>();


        let mut er =  unsafe{
            bindings::tecZoneCreateFE(
                self.file_handle,
                title.as_ptr(),
                zone_type as i32,
                nodes,
                cells,
                var_types.as_ptr(),
                var_share.as_ptr(),
                value_locs.as_ptr(),
                passive_var_list.as_ptr(),
                0,
                0,
                0,
                &mut zone as *mut i32
            )
        };
        if er != 0{
            return Err(TecioError{
                message:"Error creating zone.".to_owned(),
                code: er,
            });
        }
        er = unsafe{
            bindings::tecZoneSetUnsteadyOptions(self.file_handle, zone, time, strand_id)
        };
        if er != 0{
            return Err(TecioError{
                message:"Error setting zone's unsteady options.".to_owned(),
                code: er,
            });
        }





        Ok(zone)
    }


    pub fn zone_write_data(&mut self, zone: i32, var: i32, data: &[f32]) -> Result<()>{

        let mut er = unsafe{
            bindings::tecZoneVarWriteFloatValues(self.file_handle, zone, var, 0, data.len() as i64, data.as_ptr())
        };
        if er != 0{
            return Err(TecioError{
                message: format!("Error writing zone's #{} var #{}.", zone, var),
                code: er,
            });
        }

        Ok(())
    }


    pub fn zone_write_nodemap(&mut self, zone: i32,  nodemap: &[i64]) -> Result<()>{

        let mut er = unsafe{
            bindings::tecZoneNodeMapWrite64(self.file_handle, zone, 0, 0, nodemap.len() as i64, nodemap.as_ptr())
        };
        if er != 0{
            return Err(TecioError{
                message: format!("Error writing zone's #{} nodemap.", zone),
                code: er,
            });
        }

        Ok(())
    }








}


impl Drop for TecWriter{
    fn drop(&mut self){
        let er =  unsafe{
            bindings::tecFileWriterClose(&mut self.file_handle)
        };
        if er != 0 {
            panic!("Error closing tecplot File!");
        }
    }
}

















