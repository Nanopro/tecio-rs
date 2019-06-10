use libc::{c_void, c_char};
use std::ffi::{OsStr, CString};
use std::ptr::null_mut;
use std::convert::From;
use crate::bindings;

use crate::common::{TecZone, ZoneType, TecioError, Result};


pub struct TecReader{
    file_handle: *mut c_void,
    pub zones: Vec<TecZone>,
}



impl TecReader{
    pub fn open<T>(file: T ) -> Result<Self>
        where T: Into<Vec<u8>>
    {
        let cname = CString::new::<T>(file)?;

        let mut file_handle = null_mut();


        let mut er =  unsafe{
            bindings::tecFileReaderOpen(cname.as_ptr(), &mut file_handle)
        };
        if er != 0{
            return Err(TecioError{
                message:"Error opening file.".to_owned(),
                code: er,
            });
        }

        let mut num_zones: i32 = 0;
        er = unsafe{
            bindings::tecDataSetGetNumZones(file_handle, &mut num_zones)
        };
        if er != 0{
            return Err(TecioError{
                message:"Error reading zone number.".to_owned(),
                code: er,
            });
        }

        let mut zones = Vec::with_capacity(num_zones as usize);
        for i in 1..num_zones+1 {
            let mut title = null_mut();
            er = unsafe{
                bindings::tecZoneGetTitle(file_handle, i as i32, &mut title)
            };
            if er != 0{
                return Err(TecioError{
                    message: format!("Error reading zone name, num = {}.", i),
                    code: er,
                });
            }

            let name = unsafe{
                CString::from_raw(title as *mut c_char)
            };

            let mut zone_type = -1;
            er = unsafe{
                bindings::tecZoneGetType(file_handle, i as i32, &mut zone_type)
            };
            if er != 0{
                return Err(TecioError{
                    message: format!("Error reading zone type, num = {}.", i),
                    code: er,
                });
            }
            let zone_type = match zone_type {
                0 => ZoneType::Ordered,
                1 => ZoneType::FELine,
                2 => ZoneType::FETriangle,
                3 => ZoneType::FEQuad,
                4 => ZoneType::FETetra,
                5 => ZoneType::FEBrick,
                6 => ZoneType::FEPolygon,
                7 => ZoneType::FEPolyhedron,
                _ => {
                    return Err(TecioError{
                        message: format!("Unknown zone type, num = {}.", i),
                        code: -1,
                    });
                }
            };

            let mut i_max: i64 = 0;
            let mut j_max: i64 = 0;
            let mut k_max: i64 = 0;
            er = unsafe{
                bindings::tecZoneGetIJK(file_handle, i, &mut i_max, &mut j_max, &mut k_max)
            };
            if er != 0{
                return Err(TecioError{
                    message: format!("Error reading zone IJK, num = {}.", i),
                    code: er,
                });
            }

            let mut time: f64 = 0.0;
            er = unsafe{
                bindings::tecZoneGetSolutionTime(file_handle, i, &mut time)
            };
            if er != 0{
                return Err(TecioError{
                    message: format!("Error reading zone solution time, num = {}.", i),
                    code: er,
                });
            }


            zones.push(TecZone{
                name: name.into_string().unwrap(),
                zone_type: zone_type,
                time,
                id: i as i32,
                i_max: i_max as i64,
                j_max: j_max as i64,
                k_max: k_max as i64,
            });
        }





        Ok(TecReader{
            file_handle: file_handle,
            zones: zones,

        })


    }

    pub fn get_data(&self, zone_id: i32, var_id: i32) -> Result<Vec<f32>>{
        let mut num_values = -1;
        let mut er = unsafe {bindings::tecZoneVarGetNumValues(self.file_handle, zone_id, var_id, &mut num_values)};
        if er != 0 {
            return Err(TecioError{
                message: format!("Cannot get num values for var = {}.", var_id),
                code: er,
            });
        }
        let mut vec = Vec::with_capacity(num_values as usize);

        er = unsafe{ bindings::tecZoneVarGetFloatValues(self.file_handle, zone_id, var_id, 1, num_values, vec.as_mut_ptr())};
        if er != 0 {
            return Err(TecioError{
                message: format!("Cannot get F32 values for var = {} of zone = {}.", var_id, zone_id),
                code: er,
            });
        }
        unsafe{vec.set_len(num_values as usize)};
        Ok(vec)
    }
    pub fn get_connectivity(&self, zone_id: i32) -> Result<Vec<u32>>{
        let mut i_max: i64 = 0;
        let mut j_max: i64 = 0;
        let mut k_max: i64 = 0;
        let mut er = unsafe{ bindings::tecZoneGetIJK(self.file_handle, zone_id, &mut i_max, &mut j_max, &mut k_max)};
        if er != 0 {
            return Err(TecioError{
                message: format!("Cannot get imax, jmax, kmax for zone = {}.", zone_id),
                code: er,
            });
        }


        let mut num_connections = -1;
        er = unsafe { bindings::tecZoneNodeMapGetNumValues(self.file_handle, zone_id, j_max, &mut num_connections) };

        if er != 0 {
            return Err(TecioError{
                message: format!("Cannot get num connections for zone = {}.", zone_id),
                code: er,
            });
        }
        let mut vec: Vec<u32> = Vec::with_capacity(num_connections as usize);
        let buffer_ind= unsafe{ libc::malloc(std::mem::size_of::<u32>()* (num_connections) as usize) as *mut u32 };
        er = unsafe{ bindings::tecZoneNodeMapGet(self.file_handle, zone_id, 1, j_max, buffer_ind as *mut i32) };
        if er != 0 {
            return Err(TecioError{
                message: format!("Cannot get node map for zone = {}.", zone_id),
                code: er,
            });
        }

        unsafe{ vec = Vec::from_raw_parts(buffer_ind, num_connections as usize, num_connections as usize) };
        println!("{}",vec.len());

        Ok(vec)
    }

}




impl Drop for TecReader{
    fn drop(&mut self){
        let er =  unsafe{
            bindings::tecFileReaderClose(&mut self.file_handle)
        };
        if er != 0 {
            panic!("Error closing tecplot File!");
        }
    }
}






