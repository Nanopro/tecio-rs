
mod bindings;


#[cfg(test)]
mod tests;

extern crate libc;

use libc::{c_void, c_char};
use std::ffi::{OsStr, CString};
use std::ptr::null_mut;
use std::convert::From;



pub struct TecReader{
    file_handle: *mut c_void,
    pub zones: Vec<TecZone>,
}

pub struct TecWriter{
    file_handle: *mut c_void,
   
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

            er = unsafe{bindings::tecFileSetDiagnosticsLevel(file_handle, 1)};

            if er != 0{
                return Err(TecioError{
                    message:"Error opening file.".to_owned(),
                    code: er,
                });
            }
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


#[derive(Debug)]
pub struct TecioError {
    message: String,
    code: i32,
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













