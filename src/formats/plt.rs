use libc::c_char;
use std::{
    borrow::Cow,
    convert::From,
    ffi::{c_void, CString, OsStr},
    fs::{read, File},
    io::{BufRead, Cursor, Read},
    mem::transmute,
    path::Path,
    ptr::null_mut,
};

use nom::{
    bytes::complete::{tag, take, take_while, take_while_m_n, *},
    character::is_alphabetic,
    combinator::{cond, map_res, not, opt},
    error::ErrorKind,
    error::ParseError,
    multi::{count, fold_many0, many0, many1, many_till},
    number::complete::{be_u8, le_f32, le_f64, le_i32, le_u32},
    sequence::tuple,
    IResult,
};

use crate::{
    common::{try_err, Dataset, OrderedZone, Result, TecDataType, TecZone, TecioError, ZoneType},
    ClassicFEZone, FaceNeighborMode, FileType, TecData, ValueLocation,
};


const MIN_VERSION: i32 = 110;

#[derive(Debug, Copy, Clone)]
pub enum PltParseError {
    HeaderVersionMissing,
    VersionMismatch { min: i32, current: i32 },
    Utf8Error,
    NotSupportedFeature,
    WrongHeaderTag,
    WrongDataTag,
    EndOfHeader,
}

impl ParseError<&[u8]> for PltParseError {
    fn from_error_kind(input: &[u8], kind: ErrorKind) -> Self {
        println!("{:?} {:?}", input, kind);
        unimplemented!()
    }

    fn append(input: &[u8], kind: ErrorKind, other: Self) -> Self {
        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub struct PltFormat {
    version: i32,
    pub dataset: Dataset,
    pub zones: Vec<TecZone>,
    pub(crate) data_blocks: Vec<DataBlock>,
}

impl PltFormat {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        use PltParseError::*;
        let data = read(path)?;
        let mut rest = data.as_slice();

        let (rest, t): (&[u8], &[u8]) = tag::<&str, &[u8], PltParseError>("#!TDV")(rest)?;
        let (rest, version): (&[u8], _) =
            take(3u32)(rest).map(|(r, b)| (r, std::str::from_utf8(b).map(|n| n.parse::<i32>())))?;
        let version = match version {
            Ok(Ok(v)) => {
                if v > MIN_VERSION {
                    v
                } else {
                    Err(VersionMismatch {
                        min: MIN_VERSION,
                        current: v,
                    })?
                }
            }
            Ok(Err(e)) => Err(PltParseError::Utf8Error)?,
            Err(err) => Err(PltParseError::Utf8Error)?,
        };
        let (rest, _) = is_number(1, rest)?;
        let (mut rest, file_type) = le_i32(rest).map(|(r, f)| (r, FileType::from(f)))?;
        let (rest, title) = parse_utf8_null_terminated(rest)?;
        let (rest, num_vars) = le_i32(rest)?;
        let (rest, var_names) = count(parse_utf8_null_terminated, num_vars as usize)(rest)?;
        let (rest, header_blocks) = many0(|input| parse_header_block(input, num_vars))(rest)?;
        let (rest, t) = le_f32(rest)?;
        assert_eq!(t, 357.0f32);

        let mut zones = header_blocks
            .into_iter()
            .filter_map(|bl| match bl {
                HeaderBlock::Zone(zone) => Some(zone),
                _ => None,
            })
            .collect::<Vec<_>>();

        let dataset = Dataset {
            num_variables: num_vars as _,
            num_zones: zones.len() as _,
            title,
            var_names,
        };
        let mut rest = rest;
        let mut data_blocks = vec![];
        for (i, z) in zones.iter_mut().enumerate() {
            match z {
                TecZone::Ordered(z) => z.id = i as i32 + 1,
                TecZone::ClassicFE(z) => z.id = i as i32 + 1,
                T_ => unimplemented!(),
            }

            let (r, bl) = parse_data_block(rest, num_vars, z)?;

            rest = r;
            data_blocks.push(bl);
        }

        Ok(PltFormat {
            version,
            dataset,
            zones,
            data_blocks,
        })
    }
}

fn is_number(num: i32, input: &[u8]) -> IResult<&[u8], (), PltParseError> {
    le_i32(input).and_then(|(r, n)| {
        if n == num {
            Ok((r, ()))
        } else {
            Err(nom::Err::Error(PltParseError::Utf8Error))
        }
    })
}

fn parse_utf8_null_terminated(mut input: &[u8]) -> IResult<&[u8], String, PltParseError> {
    let mut title = vec![];
    let (rest, _) = loop {
        let (r, i) = le_u32(input)?;
        if i == 0 {
            break (r, ());
        } else {
            input = r;
            title.push(i as u8);
        }
    };
    let s = String::from_utf8(title).unwrap();
    Ok((rest, s))
}

fn parse_header_zone(input: &[u8], num_vars: i32) -> IResult<&[u8], TecZone, PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 299.0 {
        return Err(nom::Err::Error(PltParseError::WrongHeaderTag));
    }
    let (rest, name) = parse_utf8_null_terminated(rest)?;
    let (rest, parent_zone) = le_i32(rest)?;
    if parent_zone != -1 {
        return Err(nom::Err::Error(PltParseError::NotSupportedFeature));
    }
    let (rest, strand_id) = le_i32(rest)?;
    let (rest, solution_time) = le_f64(rest)?;
    let (rest, _) = tag(&i32::to_le_bytes(-1))(rest)?;
    let (rest, zone_type) = le_i32(rest).map(|(r, z)| (r, ZoneType::from(z)))?;
    let (rest, specify_var_loc) = le_i32(rest)?;
    let (rest, var_location) = if specify_var_loc == 1 {
        count(le_i32, num_vars as usize)(rest)
            .map(|(r, v)|
                (
                    r,
                    v.into_iter().map(|v| ValueLocation::from(1 - v )).collect::<Vec<_>>()
                )

            )?
    } else {
        (rest, vec![ValueLocation::Nodal; num_vars as usize])
    };
    let (rest, raw_local_supplied) = le_i32(rest)?;
    let (rest, misc_face_connect) = le_i32(rest)?;
    let (rest, neighbor_mode, is_specified) = if misc_face_connect != 0 {
        unimplemented!();
        let (rest, n) = le_i32(rest).map(|(r, m)| (r, FaceNeighborMode::from(m)))?;
        let (rest, is_spec) = if zone_type.is_fe() {
            le_i32(rest)?
        } else {
            (rest, 0)
        };
        (rest, n, is_spec)
    } else {
        (rest, FaceNeighborMode::LocalOneToOne, 0)
    };

    match zone_type {
        ZoneType::Ordered => {
            let (rest, (i_max, j_max, k_max)) = do_parse!(
                rest,
                i_max: le_i32 >> j_max: le_i32 >> k_max: le_i32 >> ((i_max, j_max, k_max))
            )?;

            let (rest, (aux_data, _)) =
                many_till(auxiliary_data, |input| is_number(0, input))(rest)?;

            Ok((
                rest,
                TecZone::Ordered(OrderedZone {
                    name,
                    id: strand_id,
                    solution_time,
                    strand: strand_id,
                    i_max: i_max as i64,
                    j_max: j_max as i64,
                    k_max: k_max as i64,
                    var_location,
                    var_types: None,
                }),
            ))
        }
        ZoneType::FEBrick
        | ZoneType::FETetra
        | ZoneType::FEQuad
        | ZoneType::FETriangle
        | ZoneType::FELine => {
            let (rest, (nodes, cells)) = do_parse!(
                rest,
                num_ptr: le_i32 >> num_elements: le_i32 >> ((num_ptr, num_elements))
            )?;
            let (rest, (i_cell_dim, j_cell_dim, k_cell_dim)) = do_parse!(
                rest,
                i_max: le_i32 >> j_max: le_i32 >> k_max: le_i32 >> ((i_max, j_max, k_max))
            )?;

            let (rest, (aux_data, _)) =
                many_till(auxiliary_data, |input| is_number(0, input))(rest)?;

            Ok((
                rest,
                TecZone::ClassicFE(ClassicFEZone {
                    name,
                    zone_type,
                    id: strand_id as _,
                    solution_time,
                    strand: strand_id as _,
                    nodes: nodes as _,
                    cells: cells as _,
                    var_location,
                    var_types: None,
                }),
            ))
        }
        _ => unimplemented!(),
    }
}

#[derive(Debug)]
pub enum HeaderBlock {
    Zone(TecZone),
    AuxDataset(String, String),
    AuxVar(i32, String, String),
    Text,
    Geom,
}

fn parse_header_block(input: &[u8], num_vars: i32) -> IResult<&[u8], HeaderBlock, PltParseError> {
    let (rest, splitter) = le_f32(input)?;
    match splitter {
        299.0 => {
            let (rest, zone) = parse_header_zone(input, num_vars)?;
            Ok((rest, HeaderBlock::Zone(zone)))
        }
        399.0 => unimplemented!(),
        799.0 => {
            let (rest, data) = parse_dataset_aux(input)?;
            Ok((rest, HeaderBlock::AuxDataset(data.0, data.1)))
        }
        899.0 => {
            let (rest, data) = parse_var_aux(input)?;
            Ok((rest, HeaderBlock::AuxVar(data.0, data.1, data.2)))
        }
        357.0 => {
            return Err(nom::Err::Error(PltParseError::EndOfHeader));
        }
        p => panic!("Not implemented for block of type {:?}", p),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DataBlock {
    pub(crate) data: Vec<(usize, TecData<'static>)>,
    pub(crate) connectivity: Option<TecData<'static>>,
    pub(crate) min_max: Vec<(f64, f64)>,
}

impl DataBlock {
    pub fn get_data(&self, var_id: usize) -> TecData {
        self.data[var_id].1.get()
    }
}

fn parse_data_block<'a>(
    input: &'a [u8],
    num_vars: i32,
    zone: &mut TecZone,
) -> IResult<&'a [u8], DataBlock, PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 299.0 {
        return Err(nom::Err::Error(PltParseError::WrongDataTag));
    }
    let (rest, data_format): (_, Vec<TecDataType>) =
        count(le_i32, num_vars as _)(rest).map(|(r, v)| (r, unsafe { transmute(v) }))?;
    let zone_data_types = zone.data_types_mut();
    *zone_data_types = Some(data_format);
    let (rest, has_passive) = le_i32(rest)?;
    let (rest, passive_list): (_, Vec<i32>) = if has_passive != 0 {
        unimplemented!()
    } else {
        (rest, vec![])
    };
    let (rest, has_share) = le_i32(rest)?;
    let (rest, share_list): (_, Vec<i32>) = if has_share != 0 {
        unimplemented!()
    } else {
        (rest, vec![])
    };
    let (rest, share_connectivity) = le_i32(rest)?;
    let (mut rest, min_max) = count(
        |input: &[u8]| do_parse!(input, min: le_f64 >> max: le_f64 >> ((min, max))),
        num_vars as usize,
    )(rest)?;

    let mut data = vec![];

    for (n, (&loc, &format)) in zone
        .var_locs()
        .iter()
        .zip(zone.data_types().unwrap().iter())
        .enumerate()
    {
        let len = match loc {
            ValueLocation::Nodal => zone.node_count(),
            ValueLocation::CellCentered => match &zone {
                TecZone::ClassicFE(z) => z.cells as _,
                TecZone::Ordered(z) => (z.i_max * z.j_max * (z.k_max - 1)) as _,
                _ => unimplemented!(),
            },
        };

        let d = match format {
            TecDataType::F64 => {
                let (r, d) = count(le_f64, len)(rest)?;
                rest = r;
                TecData::F64(Cow::Owned(d))
            }
            TecDataType::F32 => {
                let (r, d) = count(le_f32, len)(rest)?;
                rest = r;
                TecData::F32(Cow::Owned(d))
            }
            _ => unimplemented!(),
        };

        let d = match &zone {
            TecZone::Ordered(z) => {
                match loc {
                    ValueLocation::CellCentered => {
                        match d {
                            TecData::F64(Cow::Owned(v)) => {
                                let mut out = Vec::with_capacity(zone.cell_count());
                                //TODO! Optimize!
                                for k in 0..z.k_max - 1 {
                                    for j in 0..z.j_max - 1 {
                                        for i in 0..z.i_max - 1 {
                                            let ind =
                                                (i + j * (z.i_max) + k * (z.i_max) * (z.j_max))
                                                    as usize;
                                            out.push(v[ind]);
                                        }
                                    }
                                }
                                TecData::F64(Cow::Owned(out))
                            }
                            _ => unimplemented!(),
                        }
                    }
                    _ => d,
                }
            }
            _ => d,
        };

        data.push((n, d));
    }

    let connectivity = match zone {
        TecZone::Ordered(_) => None,
        TecZone::ClassicFE(z) => {
            if share_connectivity == -1 {
                let len = match z.zone_type {
                    ZoneType::FELine => 2,
                    ZoneType::FETriangle => 3,
                    ZoneType::FEQuad => 4,
                    ZoneType::FETetra => 4,
                    ZoneType::FEBrick => 8,
                    _ => unreachable!(),
                };
                let (r, c) = count(le_i32, len as _)(rest)?;
                rest = r;
                Some(TecData::I32(Cow::Owned(c)))
            } else {
                unimplemented!()
            }
        }
        _ => unimplemented!(),
    };

    Ok((
        rest,
        DataBlock {
            data,
            connectivity,
            min_max,
        },
    ))
}

fn parse_geom(input: &[u8]) -> IResult<&[u8], (), PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 399.0 {
        return Err(nom::Err::Error(PltParseError::WrongHeaderTag));
    }

    unimplemented!()
}

fn parse_text(input: &[u8]) -> IResult<&[u8], (), PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 499.0 {
        return Err(nom::Err::Error(PltParseError::WrongHeaderTag));
    }

    unimplemented!()
}

fn parse_custom_label(input: &[u8]) -> IResult<&[u8], (), PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 599.0 {
        return Err(nom::Err::Error(PltParseError::WrongHeaderTag));
    }

    unimplemented!()
}

fn parse_user_recs(input: &[u8]) -> IResult<&[u8], (), PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 699.0 {
        return Err(nom::Err::Error(PltParseError::WrongHeaderTag));
    }

    unimplemented!()
}

fn parse_dataset_aux(input: &[u8]) -> IResult<&[u8], (String, String), PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 799.0 {
        return Err(nom::Err::Error(PltParseError::WrongHeaderTag));
    }
    let (rest, aux_data) = auxiliary_data(rest)?;
    Ok((rest, aux_data))
}

fn parse_var_aux(input: &[u8]) -> IResult<&[u8], (i32, String, String), PltParseError> {
    let (rest, t) = le_f32(input)?;
    if t != 899.0 {
        return Err(nom::Err::Error(PltParseError::WrongHeaderTag));
    }
    let (rest, data) = do_parse!(
        rest,
        var_num: le_i32
            >> name: parse_utf8_null_terminated
            >> format: le_i32
            >> value: parse_utf8_null_terminated
            >> ((var_num, name, value))
    )?;
    Ok((rest, data))
}

fn auxiliary_data(input: &[u8]) -> IResult<&[u8], (String, String), PltParseError> {
    let (rest, (name, format, value)) = do_parse!(
        input,
        name: parse_utf8_null_terminated
            >> format: le_i32
            >> value: parse_utf8_null_terminated
            >> ((name, format, value))
    )?;
    Ok((rest, (name, value)))
}

#[cfg(test)]
mod tests {
    use crate::PltFormat;

    #[test]
    fn simple_test() {
        let f = PltFormat::open(r".\tests\heated_fin.plt");



        if let Ok(format) = f {
            println!("{:?}", format.zones);
            println!("Min max: {:?}", format.data_blocks[2].min_max);
            let xi = &format.data_blocks[2].data[3].1;
            println!("xi: {:?}", xi);
        }else{
           println!("{:?}", f);
           assert!(false);
        }


    }
}
