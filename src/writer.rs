use libc::{c_void, c_char};
use std::ffi::{OsStr, CString};
use std::ptr::null_mut;
use std::convert::From;
use crate::bindings;
use crate::common::{ZoneType, TecioError, Result};


pub struct TecWriter{
    file_handle: *mut c_void,

}





impl TecWriter{
    pub fn create<T>(file: T, dataset_title:T,var_list:T) -> Result<Self>
        where T: Into<Vec<u8>>{
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
                2, //default --- float
                null_mut(),
                &mut file_handle,
            )
        };

        /*er = unsafe{bindings::tecFileSetDiagnosticsLevel(file_handle, 1)};

        if er != 0{
            return Err(TecioError{
                message:"Error opening file.".to_owned(),
                code: er,
            });
        }*/

        Ok(Self{
            file_handle,

        })


    }



    pub fn add_fe_zone<T>(&mut self, title: T, zone_type: ZoneType, nodes: i64, cells: i64, time: f64, strand_id: i32) -> Result<i32>
        where T: Into<Vec<u8>>{
        let title = CString::new::<T>(title)?;
        let mut zone = 0;

        let var_types = vec![2,2,2,2i32];
        let var_share = vec![0,0,0,0i32];
        let passive_var_list = vec![0,0,0,0i32];
        let value_locs = vec![1,1,1,1i32];

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

















