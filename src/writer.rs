use crate::common::{FileFormat, FileType, Result, TecioError, ZoneType};
use crate::{bindings, try_err, FaceNeighborMode, TecData, TecDataType, TecZone};
use libc::c_char;
use std::convert::From;
use std::ffi::{c_void, CStr, CString, OsStr};
use std::fmt::Error;
use std::ptr::{null, null_mut};

pub struct TecWriter {
    file_handle: *mut c_void,
    num_vars: usize,
}

unsafe impl Send for TecWriter {}

pub struct WriterConfig {
    diagnostics_level: i32,
    file_format: FileFormat,
    file_type: FileType,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            diagnostics_level: 0,
            file_format: FileFormat::Subzone,
            file_type: FileType::Full,
        }
    }
}

impl WriterConfig {
    pub fn diagnostics_level(mut self, level: i32) -> Self {
        self.diagnostics_level = level;
        self
    }
    pub fn file_format(mut self, file_format: FileFormat) -> Self {
        self.file_format = file_format;
        self
    }
    pub fn file_type(mut self, file_type: FileType) -> Self {
        self.file_type = file_type;
        self
    }
}

impl TecWriter {
    pub fn create<T>(
        file: T,
        dataset_title: T,
        var_list: T,
        num_vars: usize,
        config: &WriterConfig,
    ) -> Result<Self>
    where
        T: AsRef<[u8]>,
    {
        let cname = CString::new(file.as_ref())?;
        let dataset_title = CString::new(dataset_title.as_ref())?;
        let var_list = CString::new(var_list.as_ref())?;

        let mut file_handle = null_mut();

        let mut er = unsafe {
            match config.file_format {
                FileFormat::Subzone => match config.file_type {
                    FileType::SolutionOnly(handler) => bindings::tecFileWriterOpen(
                        cname.as_ptr(),
                        dataset_title.as_ptr(),
                        var_list.as_ptr(),
                        FileFormat::Subzone as i32,
                        2,
                        TecDataType::F32 as i32,
                        handler,
                        &mut file_handle,
                    ),
                    x => bindings::tecFileWriterOpen(
                        cname.as_ptr(),
                        dataset_title.as_ptr(),
                        var_list.as_ptr(),
                        FileFormat::Subzone as i32,
                        x.as_i32(),
                        TecDataType::F32 as i32,
                        null_mut(),
                        &mut file_handle,
                    ),
                },
                FileFormat::Binary => {
                    return Err(TecioError::Other {
                        message: format!(
                            "Unsupported file format {:?}! Supported types: [Subzone]",
                            config.file_format
                        ),
                        code: -1,
                    });
                }
            }
        };

        if er != 0 {
            return Err(TecioError::Other {
                message: "Error opening file.".to_owned(),
                code: er,
            });
        }

        er = unsafe { bindings::tecFileSetDiagnosticsLevel(file_handle, config.diagnostics_level) };

        if er != 0 {
            return Err(TecioError::Other {
                message: "Error setting diagnostics level".to_owned(),
                code: er,
            });
        }

        Ok(Self {
            file_handle,
            num_vars,
        })
    }

    pub fn add_zone(&mut self, zone: TecZone) -> Result<TecZoneWriter> {
        match zone {
            TecZone::Ordered(zone) => {
                let zone_title = CString::new(zone.name.clone()).unwrap();
                let mut id = -1;
                let array_of_nulls = vec![0; self.num_vars];

                try_err(
                    unsafe {
                        bindings::tecZoneCreateIJK(
                            self.file_handle,
                            zone_title.as_ptr(),
                            zone.i_max,
                            zone.j_max,
                            zone.k_max,
                            zone.var_types
                                .as_ref()
                                .map(|v| v.as_ptr() as *const _)
                                .unwrap_or(null()),
                            array_of_nulls.as_ptr(),
                            zone.var_location.as_ptr() as *const _,
                            array_of_nulls.as_ptr(),
                            0,
                            0,
                            FaceNeighborMode::GlobalOneToMany as i32,
                            &mut id,
                        )
                    },
                    format!("Error creating zone with parameters: {:?}", zone),
                )?;

                Ok(TecZoneWriter {
                    writer: self,
                    zone: TecZone::Ordered(zone),
                    id,
                })
            }
            TecZone::ClassicFE(zone) => {
                let zone_title = CString::new(zone.name.clone()).unwrap();
                let mut id = -1;
                let array_of_nulls = vec![0; self.num_vars];
                try_err(
                    unsafe {
                        bindings::tecZoneCreateFE(
                            self.file_handle,
                            zone_title.as_ptr(),
                            zone.zone_type as _,
                            zone.nodes,
                            zone.cells,
                            zone.var_types
                                .as_ref()
                                .map(|v| v.as_ptr() as *const _)
                                .unwrap_or(null()),
                            array_of_nulls.as_ptr(),
                            zone.var_location.as_ptr() as *const _,
                            array_of_nulls.as_ptr(),
                            0,
                            0,
                            0,
                            &mut id,
                        )
                    },
                    format!("Error creating zone with parameters: {:?}", zone),
                )?;
                Ok(TecZoneWriter {
                    writer: self,
                    zone: TecZone::ClassicFE(zone),
                    id,
                })
            }
            _ => unimplemented!(),
        }
    }

    pub fn handler(&self) -> *mut c_void {
        self.file_handle
    }

    pub fn add_fe_zone<T>(
        &mut self,
        title: T,
        zone_type: ZoneType,
        nodes: i64,
        cells: i64,
        time: f64,
        strand_id: i32,
    ) -> Result<i32>
    where
        T: AsRef<[u8]>,
    {
        let title = CString::new(title.as_ref())?;
        let mut zone = 0;

        let var_types = (0..self.num_vars).map(|_| 1).collect::<Vec<_>>();
        let var_share = (0..self.num_vars).map(|_| 0).collect::<Vec<_>>();
        let passive_var_list = (0..self.num_vars).map(|_| 0).collect::<Vec<_>>();
        let value_locs = (0..self.num_vars).map(|_| 1).collect::<Vec<_>>();

        let mut er = unsafe {
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
                &mut zone as *mut i32,
            )
        };
        if er != 0 {
            return Err(TecioError::Other {
                message: "Error creating zone.".to_owned(),
                code: er,
            });
        }
        er =
            unsafe { bindings::tecZoneSetUnsteadyOptions(self.file_handle, zone, time, strand_id) };
        if er != 0 {
            return Err(TecioError::Other {
                message: "Error setting zone's unsteady options.".to_owned(),
                code: er,
            });
        }

        Ok(zone)
    }
}

impl Drop for TecWriter {
    fn drop(&mut self) {
        let er = unsafe { bindings::tecFileWriterClose(&mut self.file_handle) };
        if er != 0 {
            panic!("Error closing tecplot File!");
        }
    }
}

pub struct TecZoneWriter<'a> {
    writer: &'a mut TecWriter,
    zone: TecZone,
    id: i32,
}

impl<'a> TecZoneWriter<'a> {
    pub fn write_data(&mut self, var: i32, data: &TecData) -> Result<()> {
        match data {
            TecData::F32(data) => {
                try_err(
                    unsafe {
                        bindings::tecZoneVarWriteFloatValues(
                            self.writer.handler(),
                            self.id,
                            var,
                            0,
                            data.len() as i64,
                            data.as_ptr(),
                        )
                    },
                    format!(
                        "Error writing to zone {}, var {}, data {:?}",
                        self.id, var, data
                    ),
                )?;

                Ok(())
            }
            TecData::F64(data) => {
                try_err(
                    unsafe {
                        bindings::tecZoneVarWriteDoubleValues(
                            self.writer.handler(),
                            self.id,
                            var,
                            0,
                            data.len() as i64,
                            data.as_ptr(),
                        )
                    },
                    format!(
                        "Error writing to zone {}, var {}, data {:?}",
                        self.id, var, data
                    ),
                )?;

                Ok(())
            }
            _ => unimplemented!(),
        }
    }

    pub fn write_nodemap(&mut self, nodemap: &TecData, one_based: bool) -> Result<()> {
        match self.zone {
            TecZone::ClassicFE(_) => match nodemap {
                TecData::I32(data) => {
                    try_err(
                        unsafe {
                            bindings::tecZoneNodeMapWrite32(
                                self.writer.file_handle,
                                self.id,
                                0,
                                one_based as _,
                                data.len() as _,
                                data.as_ptr(),
                            )
                        },
                        format!(
                            "Error writing nodemap to zone #{}, {:?}",
                            self.id, self.zone
                        ),
                    )?;
                }
                TecData::I64(data) => {
                    try_err(
                        unsafe {
                            bindings::tecZoneNodeMapWrite64(
                                self.writer.file_handle,
                                self.id,
                                0,
                                one_based as _,
                                data.len() as _,
                                data.as_ptr(),
                            )
                        },
                        format!(
                            "Error writing nodemap to zone #{}, {:?}",
                            self.id, self.zone
                        ),
                    )?;
                }
                _ => {
                    return Err(TecioError::Other {
                        message: format!("Unsupported datatype for nodemap!"),
                        code: -1,
                    })
                }
            },
            _ => {
                return Err(TecioError::Other {
                    message: format!(
                        "Error, zone #{} of type {:?}, cannot contain nodemap!",
                        self.id,
                        self.zone.zone_type()
                    ),
                    code: -1,
                })
            }
        }
        Ok(())
    }
}
