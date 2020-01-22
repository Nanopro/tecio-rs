use std::{
    borrow::Cow,
    convert::From,
    ffi::{c_void, CString, OsStr},
    path::Path,
    ptr::null_mut,
};

use libc::c_char;

use crate::{
    bindings,
    common::{
        try_err, ClassicFEZone, Dataset, OrderedZone, Result, TecData, TecDataType, TecZone,
        TecioError, ValueLocation, ZoneType,
    },
};

macro_rules! try_err {
    ($f: expr, $l: expr) => {
        unsafe { try_err($f, $l)? }
    };
}

pub struct SzpltFormat {
    file_handle: *mut c_void,
    pub dataset: Dataset,
    pub zones: Vec<TecZone>,
}

impl SzpltFormat {
    pub fn open<T>(file: T) -> Result<Self>
    where
        T: Into<Vec<u8>>,
    {
        unsafe {
            let cname = CString::new::<T>(file)?;

            let mut file_handle = null_mut();
            let mut dataset = Dataset::empty();

            try_err(
                bindings::tecFileReaderOpen(cname.as_ptr(), &mut file_handle),
                "Error opening file.",
            )?;

            let mut title = null_mut();
            try_err(
                bindings::tecDataSetGetTitle(file_handle, &mut title),
                "Error reading dataset title.",
            )?;
            let title = unsafe { CString::from_raw(title as *mut c_char) };
            let title = title.into_string()?;
            dataset.title = title;

            let mut num_zones: i32 = 0;
            try_err(
                bindings::tecDataSetGetNumZones(file_handle, &mut num_zones),
                "Error reading zone number.",
            )?;

            dataset.num_zones = num_zones;

            let mut num_vars: i32 = 0;
            try_err(
                bindings::tecDataSetGetNumVars(file_handle, &mut num_vars),
                "Error reading var number.",
            )?;

            dataset.num_variables = num_vars;
            dataset.var_names.reserve(num_vars as usize);

            for i in 1..=num_vars {
                let mut var = null_mut();
                try_err(
                    bindings::tecVarGetName(file_handle, i, &mut var),
                    format!("Error reading var name, num = {}.", i),
                )?;

                let name = unsafe { CString::from_raw(var as *mut c_char) };
                let name = name.into_string()?;

                dataset.var_names.push(name);
            }

            let mut zones = Vec::with_capacity(num_zones as usize);
            for i in 1..num_zones + 1 {
                let mut zone_type = -1;
                try_err(
                    bindings::tecZoneGetType(file_handle, i as i32, &mut zone_type),
                    format!("Error reading zone type, num = {}.", i),
                )?;
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
                        return Err(TecioError::Other {
                            message: format!("Unknown zone type, num = {}.", i),
                            code: -1,
                        });
                    }
                };

                let mut title = null_mut();
                try_err(
                    bindings::tecZoneGetTitle(file_handle, i as i32, &mut title),
                    format!("Error reading zone name, num = {}.", i),
                )?;
                let zone_name = unsafe { CString::from_raw(title as *mut c_char) };

                let zone_name = zone_name.into_string()?;

                let mut i_max: i64 = 0;
                let mut j_max: i64 = 0;
                let mut k_max: i64 = 0;
                try_err(
                    bindings::tecZoneGetIJK(file_handle, i, &mut i_max, &mut j_max, &mut k_max),
                    format!("Error reading zone IJK, num = {}.", i),
                )?;

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

                let mut locs = vec![ValueLocation::Nodal; num_vars as usize];
                for (v, loc) in locs.iter_mut().enumerate() {
                    try_err(
                        bindings::tecZoneVarGetValueLocation(
                            file_handle,
                            i,
                            v as i32 + 1,
                            loc as *mut ValueLocation as *mut i32,
                        ),
                        format!("Error reading zone locations"),
                    )?;
                }

                let zone = match zone_type {
                    ZoneType::Ordered => TecZone::Ordered(OrderedZone {
                        name: zone_name,
                        id: i,
                        solution_time: time,
                        strand: strand_id,
                        i_max,
                        j_max,
                        k_max,

                        var_location: locs,
                        var_types: None,
                    }),
                    ZoneType::FEQuad
                    | ZoneType::FETriangle
                    | ZoneType::FEBrick
                    | ZoneType::FELine
                    | ZoneType::FETetra => {
                        assert_eq!(k_max, 0);
                        TecZone::ClassicFE(ClassicFEZone {
                            name: zone_name,
                            zone_type: zone_type,
                            id: i,
                            solution_time: time,
                            strand: strand_id,

                            nodes: i_max,
                            cells: j_max,

                            var_location: locs,
                            var_types: None,
                        })
                    }
                    zone => {
                        return Err(TecioError::Other {
                            message: format!("Not implemented zone type: {:?}", zone),
                            code: -1,
                        });
                    }
                };

                zones.push(zone);
            }

            Ok(Self {
                file_handle: file_handle,
                zones: zones,
                dataset,
            })
        }
    }

    pub fn get_data_type(&self, zone_id: i32, var_id: i32) -> Result<TecDataType> {
        let mut is_enabled = 0;
        unsafe {
            try_err(
                bindings::tecVarIsEnabled(self.file_handle, var_id, &mut is_enabled),
                format!("Var {} is not enabled.", var_id),
            )?
        };

        let mut data_type = 0;
        unsafe {
            try_err(
                bindings::tecZoneVarGetType(self.file_handle, zone_id, var_id, &mut data_type),
                format!("Cannot load var's {} data type.", var_id),
            )?
        };

        Ok(TecDataType::from(data_type))
    }

    pub fn get_data(&self, zone_id: usize, var_id: usize) -> Result<TecData> {
        let mut num_values = -1;
        unsafe {
            try_err(
                bindings::tecZoneVarGetNumValues(
                    self.file_handle,
                    zone_id as _,
                    var_id as _,
                    &mut num_values,
                ),
                format!("Cannot get num values for var = {}.", var_id),
            )?;
        }

        let mut is_enabled = 0;
        unsafe {
            try_err(
                bindings::tecVarIsEnabled(self.file_handle, var_id as _, &mut is_enabled),
                format!("Var {} is not enabled.", var_id),
            )?;
        }

        let mut data_type = 0;
        unsafe {
            try_err(
                bindings::tecZoneVarGetType(
                    self.file_handle,
                    zone_id as _,
                    var_id as _,
                    &mut data_type,
                ),
                format!("Cannot get var's {} data type", var_id),
            )?;
        }
        let data_type = TecDataType::from(data_type);

        match data_type {
            TecDataType::F64 => {
                let mut vec = vec![0.0; num_values as _];
                unsafe {
                    try_err(
                        bindings::tecZoneVarGetDoubleValues(
                            self.file_handle,
                            zone_id as _,
                            var_id as _,
                            1,
                            num_values,
                            vec.as_mut_ptr(),
                        ),
                        format!(
                            "Cannot get F64 values for var = {} of zone = {}.",
                            var_id, zone_id
                        ),
                    )
                };
                Ok(TecData::F64(Cow::Owned(vec)))
            }
            TecDataType::F32 => {
                let mut vec = vec![0.0; num_values as _];
                unsafe {
                    try_err(
                        bindings::tecZoneVarGetFloatValues(
                            self.file_handle,
                            zone_id as _,
                            var_id as _,
                            1,
                            num_values,
                            vec.as_mut_ptr(),
                        ),
                        format!(
                            "Cannot get F64 values for var = {} of zone = {}.",
                            var_id, zone_id
                        ),
                    )
                };
                Ok(TecData::F32(Cow::Owned(vec)))
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_connectivity(&self, zone_id: i32) -> Result<Option<TecData>> {
        match &self.zones[zone_id as usize - 1] {
            TecZone::ClassicFE(zone) => {
                let mut is_64b = 0;

                try_err(
                    unsafe {
                        bindings::tecZoneNodeMapIs64Bit(self.file_handle, zone_id, &mut is_64b)
                    },
                    format!("Could not get is zone map 64 bit flag"),
                )?;

                match is_64b {
                    0 => {
                        let mut values = vec![0; zone.num_connections()];
                        try_err(
                            unsafe {
                                bindings::tecZoneNodeMapGet(
                                    self.file_handle,
                                    zone_id,
                                    1,
                                    zone.cells as _,
                                    values.as_mut_ptr(),
                                )
                            },
                            format!("Could not get zone's {} nodemap", zone_id),
                        )?;
                        Ok(Some(TecData::I32(Cow::Owned(values))))
                    }
                    1 => {
                        let mut values = vec![0; zone.num_connections()];
                        try_err(
                            unsafe {
                                bindings::tecZoneNodeMapGet64(
                                    self.file_handle,
                                    zone_id,
                                    1,
                                    zone.cells as _,
                                    values.as_mut_ptr(),
                                )
                            },
                            format!("Could not get zone's {} nodemap", zone_id),
                        )?;
                        Ok(Some(TecData::I64(Cow::Owned(values))))
                    }
                    _ => unreachable!(),
                }
            }
            TecZone::Ordered(_) => Ok(None),
            _ => unimplemented!(),
        }
        //        let mut i_max: i64 = 0;
        //        let mut j_max: i64 = 0;
        //        let mut k_max: i64 = 0;
        //        let mut er = unsafe {
        //            bindings::tecZoneGetIJK(
        //                self.file_handle,
        //                zone_id,
        //                &mut i_max,
        //                &mut j_max,
        //                &mut k_max,
        //            )
        //        };
        //        if er != 0 {
        //            return Err(TecioError::Other {
        //                message: format!("Cannot get imax, jmax, kmax for zone = {}.", zone_id),
        //                code: er,
        //            });
        //        }
        //
        //        let mut num_connections = -1;
        //        er = unsafe {
        //            bindings::tecZoneNodeMapGetNumValues(
        //                self.file_handle,
        //                zone_id,
        //                j_max,
        //                &mut num_connections,
        //            )
        //        };
        //
        //        if er != 0 {
        //            return Err(TecioError::Other {
        //                message: format!("Cannot get num connections for zone = {}.", zone_id),
        //                code: er,
        //            });
        //        }
        //        let mut vec: Vec<u32> = Vec::with_capacity(num_connections as usize);
        //        let buffer_ind = unsafe {
        //            libc::malloc(std::mem::size_of::<u32>() * (num_connections) as usize) as *mut u32
        //        };
        //        er = unsafe {
        //            bindings::tecZoneNodeMapGet(self.file_handle, zone_id, 1, j_max, buffer_ind as *mut i32)
        //        };
        //        if er != 0 {
        //            return Err(TecioError::Other {
        //                message: format!("Cannot get node map for zone = {}.", zone_id),
        //                code: er,
        //            });
        //        }
        //
        //        unsafe {
        //            vec = Vec::from_raw_parts(
        //                buffer_ind,
        //                num_connections as usize,
        //                num_connections as usize,
        //            )
        //        };
        //
        //        Ok(vec)
    }
}

impl Drop for SzpltFormat {
    fn drop(&mut self) {
        let er = unsafe { bindings::tecFileReaderClose(&mut self.file_handle) };
        if er != 0 {
            panic!("Error closing tecplot File!");
        }
    }
}
