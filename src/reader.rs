use libc::{c_void, c_char};
use std::ffi::{OsStr, CString};
use std::ptr::null_mut;
use std::convert::From;
use crate::{bindings, ClassicFEZone};


use crate::common::{Dataset, TecZone, ZoneType, TecioError, Result, try_err, OrderedZone, TecDataType};
use std::marker::PhantomData;


pub struct TecReader{
    file_handle: *mut c_void,
    pub dataset: Dataset,
    pub zones: Vec<TecZone>,
}




macro_rules! try_err {
    ($f: expr, $l: expr) => {
        unsafe{
            try_err($f, $l)?
        }
    };
}





impl TecReader{
    pub fn open<T>(file: T ) -> Result<Self>
        where T: Into<Vec<u8>>
    {
        unsafe{


            let cname = CString::new::<T>(file)?;

            let mut file_handle = null_mut();
            let mut dataset = Dataset::empty();




            try_err(bindings::tecFileReaderOpen(cname.as_ptr(), &mut file_handle), "Error opening file.")?;

            let mut title = null_mut();
            try_err(bindings::tecDataSetGetTitle(file_handle, &mut title), "Error reading dataset title.")?;
            let title = unsafe{
                CString::from_raw(title as *mut c_char)
            };
            let title = title.into_string()?;
            dataset.title = title;

            let mut num_zones: i32 = 0;
            try_err(bindings::tecDataSetGetNumZones(file_handle, &mut num_zones), "Error reading zone number.")?;

            dataset.num_zones = num_zones;

            let mut num_vars: i32 = 0;
            try_err(bindings::tecDataSetGetNumVars(file_handle, &mut num_vars), "Error reading var number.")?;

            dataset.num_variables = num_vars;
            dataset.var_names.reserve(num_vars as usize);



            for i in 1..=num_vars{
                let mut var = null_mut();
                try_err(
                    bindings::tecVarGetName(file_handle, i, &mut var),
                    format!("Error reading var name, num = {}.", i)
                )?;

                let name = unsafe{
                    CString::from_raw(var as *mut c_char)
                };
                let name = name.into_string()?;

                dataset.var_names.push(name);
            }



            let mut zones = Vec::with_capacity(num_zones as usize);
            for i in 1..num_zones+1 {
                let mut zone_type = -1;
                try_err(bindings::tecZoneGetType(file_handle, i as i32, &mut zone_type),
                        format!("Error reading zone type, num = {}.", i))?;
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

                let mut title = null_mut();
                try_err(bindings::tecZoneGetTitle(file_handle, i as i32, &mut title),
                        format!("Error reading zone name, num = {}.", i))?;
                let zone_name = unsafe{
                    CString::from_raw(title as *mut c_char)
                };

                let zone_name = zone_name.into_string()?;

                let mut i_max: i64 = 0;
                let mut j_max: i64 = 0;
                let mut k_max: i64 = 0;
                try_err(bindings::tecZoneGetIJK(file_handle, i, &mut i_max, &mut j_max, &mut k_max),
                        format!("Error reading zone IJK, num = {}.", i))?;

                let mut time: f64 = 0.0;
                try_err(
                    bindings::tecZoneGetSolutionTime(file_handle, i, &mut time),
                    format!("Error reading zone solution time, num = {}.", i),
                )?;

                let mut strand_id: i32 = 0;
                try_err(
                    bindings::tecZoneGetStrandID(file_handle, i, &mut strand_id),
                    format!("Error reading zone strand id, num = {}.", i),
                )?;


                let zone = match zone_type{
                    ZoneType::Ordered => {
                        TecZone::Ordered(OrderedZone{
                            name: zone_name,
                            id: i,
                            solution_time: time,
                            strand: strand_id,
                            i_max: i_max,
                            j_max: j_max,
                            k_max: k_max,

                        })
                    },
                    ZoneType::FEQuad | ZoneType::FETriangle => {

                        TecZone::ClassicFE(ClassicFEZone{
                            name: zone_name,
                            zone_type: zone_type,
                            id: i,
                            solution_time: time,
                            strand: strand_id,
                        })
                    },
                    zone => {
                        return Err(TecioError{
                            message: format!("Not implemented zone type: {:?}", zone),
                            code: -1,
                        });

                    }
                };











                zones.push(zone);
            }





            Ok(TecReader{
                file_handle: file_handle,
                zones: zones,
                dataset,
            })

        }
    }


    pub fn get_data_type(&self, zone_id: i32, var_id: i32) -> Result<TecDataType>{
        let mut is_enabled = 0;
        unsafe {try_err(bindings::tecVarIsEnabled(self.file_handle, var_id, &mut is_enabled), format!("Var {} is not enabled.", var_id))};


        let mut data_type = 0;
        unsafe {try_err(bindings::tecZoneVarGetType(self.file_handle,zone_id, var_id, &mut data_type), format!("Cannot load var's {} data type.", var_id))    };

        println!("{}", data_type);
        match data_type{
            1 => Ok(TecDataType::F32),
            2 => Ok(TecDataType::F64),
            _ =>  Err(TecioError{
                message: format!("Unknown data type for var {}", var_id),
                code: -1,
            })
        }
    }

    pub fn get_data(&self, zone_id: i32, var_id: i32) -> Result<Vec<f32>>{
        let mut num_values = -1;
        unsafe {try_err(bindings::tecZoneVarGetNumValues(self.file_handle, zone_id, var_id, &mut num_values), format!("Cannot get num values for var = {}.", var_id))};

        let mut is_enabled = 0;
        unsafe {try_err(bindings::tecVarIsEnabled(self.file_handle, var_id, &mut is_enabled), format!("Var {} is not enabled.", var_id))};
        //println!("Is enabled: {}", is_enabled);



        let mut vec = Vec::with_capacity(num_values as usize);
        //assert_ne!(num_values, 0);

        unsafe{ try_err(bindings::tecZoneVarGetFloatValues(self.file_handle, zone_id, var_id, 1, num_values, vec.as_mut_ptr()), format!("Cannot get F32 values for var = {} of zone = {}.", var_id, zone_id))};

        unsafe{vec.set_len(num_values as usize)};
        Ok(vec)
    }

    pub fn get_data_f64(&self, zone_id: i32, var_id: i32) -> Result<Vec<f64>>{
        let mut num_values = -1;
        unsafe {try_err(bindings::tecZoneVarGetNumValues(self.file_handle, zone_id, var_id, &mut num_values), format!("Cannot get num values for var = {}.", var_id))};

        let mut is_enabled = 0;
        unsafe {try_err(bindings::tecVarIsEnabled(self.file_handle, var_id, &mut is_enabled), format!("Var {} is not enabled.", var_id))};
        //println!("Is enabled: {}", is_enabled);



        let mut vec = Vec::with_capacity(num_values as usize);
        //assert_ne!(num_values, 0);

        unsafe{ try_err(bindings::tecZoneVarGetDoubleValues(self.file_handle, zone_id, var_id, 1, num_values, vec.as_mut_ptr()), format!("Cannot get F64 values for var = {} of zone = {}.", var_id, zone_id))};

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






