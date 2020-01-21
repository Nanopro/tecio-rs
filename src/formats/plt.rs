use libc::c_char;
use std::{
    convert::From,
    ffi::{CString, OsStr, c_void},
    ptr::null_mut,
    path::{Path},
    fs::{File, read},
    io::{Cursor, Read, BufRead}
};

use nom::{
    IResult,
    bytes::complete::{tag, take_while_m_n, take_while, take, *},
    number::complete::{le_i32, be_u8, le_u32, le_f64, le_f32},
    combinator::{map_res, not,opt, cond},
    sequence::{tuple, },
    multi::{count, },
    error::ParseError,
};

use crate::{ClassicFEZone, ValueLocation, common::{
    try_err, Dataset, OrderedZone, Result, TecDataType, TecZone, TecioError, ZoneType,
}, FaceNeighborMode, FileType};
use nom::error::ErrorKind;
use nom::multi::{many0, many1, many_till, fold_many0};
use nom::character::is_alphabetic;
use widestring::MissingNulError;
use nom::lib::std::mem::transmute;

const MIN_VERSION: i32 = 110;
#[derive(Debug, Copy, Clone)]
pub enum PltParseError{
    HeaderVersionMissing,
    VersionMismatch { min: i32, current: i32 },
    Utf8Error,
    Utf32Error,
    NotSupportedFeature,
    WrongTag,
    EndOfHeader
}

impl ParseError<&[u8]> for PltParseError{
    fn from_error_kind(input: &[u8], kind: ErrorKind) -> Self {
        println!("{:?} {:?}", input, kind);
        unimplemented!()
    }

    fn append(input: &[u8], kind: ErrorKind, other: Self) -> Self {
        unimplemented!()
    }
}
impl From<widestring::MissingNulError<i32>> for PltParseError{
    fn from(_: MissingNulError<i32>) -> Self {
        Self::Utf32Error
    }
}




#[derive(Clone, Debug)]
pub struct PltFormat{
    version: i32,
    pub dataset: Dataset,

}

impl PltFormat{
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self>{
        use PltParseError::*;
        let data = read(path)?;
        let mut rest = data.as_slice();

        let (rest, t): (&[u8], &[u8]) = tag::<&str, &[u8], PltParseError>("#!TDV")(rest)?;
        let (rest, version): (&[u8], _) = take(3u32)(rest).map(|(r, b)| (r, std::str::from_utf8(b).map(|n| n.parse::<i32>())))?;
        let version = match version{
            Ok(Ok(v)) => {
                if v > MIN_VERSION{
                    v
                } else{
                    Err(VersionMismatch { min: MIN_VERSION, current: v})?
                }
            },
            Ok(Err(e)) => {
                Err(PltParseError::Utf8Error)?
            }
            Err(err) => {
                Err(PltParseError::Utf8Error)?
            }
        };
        let (rest, _) = is_number(1,rest)?;
        let (mut rest, file_type) = le_i32(rest).map(|(r, f)| (r, FileType::from(f)))?;
        let (rest, title) = parse_utf8_null_terminated(rest)?;
        let (rest, num_vars) = le_i32(rest)?;
        let(rest, var_names) = count(parse_utf8_null_terminated, num_vars as usize)(rest)?;
        let (rest, header_blocks) = many0(|input| parse_header_block(input, num_vars))(rest)?;
        let (rest, t) = le_f32(rest)?;
        assert_eq!(t, 357.0f32);

        let zones = header_blocks.into_iter().filter_map(|bl| {
            match bl{
                HeaderBlock::Zone(zone) => Some(zone),
                _ => None,
            }
        }).collect::<Vec<_>>();

        let dataset = Dataset{
            num_variables: num_vars as _,
            num_zones: zones.len() as _,
            title,
            var_names
        };
        let (rest, data_blocks) = many0(|input| parse_data_block(input, num_vars))(rest)?;


        Ok(
            PltFormat{
                version,
                dataset
            }
        )
    }
}



fn is_number(num: i32, input: &[u8]) -> IResult<&[u8], (), PltParseError>{
    le_i32(input).and_then(|(r, n)| {
        if n == num {
            Ok((r, ()))
        } else {
            Err(nom::Err::Error(PltParseError::Utf8Error))
        }
    })
}

fn parse_utf8_null_terminated(mut input: &[u8]) -> IResult<&[u8], String, PltParseError>{
    let mut title = vec![];
    let (rest, _ ) = loop {
        let (r, i) = le_u32(input)?;
        if i == 0 {
            break (r, ())
        } else{
            input = r;
            title.push(i as u8);
        }
    };
    let s = String::from_utf8(title).unwrap();
    Ok((rest, s))
}

fn parse_header_zone(input: &[u8], num_vars: i32) -> IResult<&[u8], TecZone, PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 299.0 {
        return Err(nom::Err::Error(PltParseError::WrongTag));
    }
    let (rest, name) = parse_utf8_null_terminated(rest)?;
    let (rest, parent_zone) = le_i32(rest)?;
    if parent_zone != -1 {
        return Err(nom::Err::Error(PltParseError::NotSupportedFeature))
    }
    let (rest, strand_id) = le_i32(rest)?;
    let (rest, solution_time) = le_f64(rest)?;
    let (rest, _) = tag(&i32::to_le_bytes(-1))(rest)?;
    let (rest, zone_type) = le_i32(rest).map(|(r, z)| (r, ZoneType::from(z)))?;
    let (rest, specify_var_loc) = le_i32(rest)?;
    let (rest, var_location) = if specify_var_loc == 1{
        count(le_i32, num_vars as usize)(rest).map(|(r, v)| (r,  unsafe { transmute(v) }) )?
    } else{
        (rest, vec![ValueLocation::Nodal; num_vars as usize])
    };
    let (rest, raw_local_supplied) = le_i32(rest)?;
    let (rest, misc_face_connect) = le_i32(rest)?;
    let (rest, neighbor_mode, is_specified) = if misc_face_connect != 0 {
        let (rest, n) = le_i32(rest).map(|(r, m)| (r, FaceNeighborMode::from(m)))?;
        let (rest, is_spec) = if zone_type.is_fe(){
            le_i32(rest)?
        }else{
            (rest, 0)
        };
        (rest, n, is_spec)
    }else{
        (rest, FaceNeighborMode::LocalOneToOne, 0)
    };


    match zone_type{
        ZoneType::Ordered => {
            let (rest, (i_max, j_max, k_max)) = do_parse!(rest,
                i_max: le_i32 >> j_max: le_i32 >> k_max: le_i32 >>
                ( (i_max, j_max, k_max ) )
            )?;

            let (rest, (aux_data, _)) = many_till(auxiliary_data, |input| is_number(0, input))(rest)?;

            Ok((
                rest,
                TecZone::Ordered(
                    OrderedZone{
                        name,
                        id: strand_id,
                        solution_time,
                        strand: strand_id,
                        i_max: i_max as i64,
                        j_max: j_max as i64,
                        k_max: k_max as i64,
                        var_location,
                        var_types: None
                    }
                )
            ))
        },
        ZoneType::FEBrick | ZoneType::FETetra | ZoneType::FEQuad | ZoneType::FETriangle | ZoneType::FELine => {
            let (rest, (nodes, cells)) = do_parse!(rest,
                num_ptr: le_i32 >> num_elements: le_i32 >>
                ( (num_ptr, num_elements) )
            )?;
            let (rest, (i_cell_dim, j_cell_dim, k_cell_dim)) = do_parse!(rest,
                i_max: le_i32 >> j_max: le_i32 >> k_max: le_i32 >>
                ( (i_max, j_max, k_max ) )
            )?;

            let (rest, (aux_data, _)) = many_till(auxiliary_data, |input| is_number(0, input))(rest)?;

            Ok((
                rest,
                TecZone::ClassicFE(ClassicFEZone{
                    name,
                    zone_type,
                    id: strand_id as _,
                    solution_time,
                    strand: strand_id as _,
                    nodes: nodes as _,
                    cells: cells as _,
                    var_location,
                    var_types: None
                })
            ))
        }
        _ => {

            unimplemented!()
        }
    }
}
#[derive(Debug)]
pub enum HeaderBlock{
    Zone(TecZone),
    AuxDataset(String, String),
    AuxVar(i32, String, String),
    Text,
    Geom,

}
fn parse_header_block(input: &[u8], num_vars: i32) -> IResult<&[u8], HeaderBlock, PltParseError>{
    let (rest, splitter) = le_f32(input)?;
    match splitter{
        299.0 => {
            let (rest, zone) = parse_header_zone(input, num_vars)?;
            Ok((rest,
                HeaderBlock::Zone(zone)
            ))
        },
        399.0 => {
            unimplemented!()
        },
        799.0 => {
            let( rest, data) = parse_dataset_aux(input)?;
            Ok((rest,
                HeaderBlock::AuxDataset(data.0, data.1)
            ))
        }
        899.0 => {
            let( rest, data ) = parse_var_aux(input)?;
            Ok((rest,
                HeaderBlock::AuxVar(data.0, data.1, data.2)
            ))
        }
        357.0 => {
            return Err(nom::Err::Error(PltParseError::EndOfHeader))
        }
        p => panic!("Not implemented for block of type {:?}", p)
    }
}

pub(crate) enum DataBlock{
    ZoneDataBlock
}

fn parse_data_block(input: &[u8], num_vars: i32) -> IResult<&[u8], DataBlock, PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 299.0{
        return Err(nom::Err::Error(PltParseError::WrongTag))
    }
    let (rest, data_format): (_, Vec<TecDataType>) = count(le_i32, num_vars as _)(rest).map(|(r, v)| (r, unsafe{ transmute(v) }))?;

    println!(" f {:?}", data_format);



    unimplemented!()
}




fn parse_geom(input: &[u8]) -> IResult<&[u8], (), PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 399.0 {
        return Err(nom::Err::Error(PltParseError::WrongTag));
    }

    unimplemented!()
}
fn parse_text(input: &[u8]) -> IResult<&[u8], (), PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 499.0 {
        return Err(nom::Err::Error(PltParseError::WrongTag));
    }

    unimplemented!()
}
fn parse_custom_label(input: &[u8]) -> IResult<&[u8], (), PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 599.0 {
        return Err(nom::Err::Error(PltParseError::WrongTag));
    }

    unimplemented!()
}
fn parse_user_recs(input: &[u8]) -> IResult<&[u8], (), PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 699.0 {
        return Err(nom::Err::Error(PltParseError::WrongTag));
    }

    unimplemented!()
}
fn parse_dataset_aux(input: &[u8]) -> IResult<&[u8], (String, String), PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 799.0 {
        return Err(nom::Err::Error(PltParseError::WrongTag));
    }
    let (rest, aux_data) = auxiliary_data(rest)?;
    Ok((rest, aux_data))
}
fn parse_var_aux(input: &[u8]) -> IResult<&[u8], (i32, String, String), PltParseError>{
    let (rest, t) = le_f32(input)?;
    if t != 899.0 {
        return Err(nom::Err::Error(PltParseError::WrongTag));
    }
    let(rest, data) = do_parse!(rest,
            var_num: le_i32 >> name: parse_utf8_null_terminated >> format: le_i32 >> value: parse_utf8_null_terminated >>
            ( (var_num, name, value ))
        )?;
    Ok((rest, data))
}

fn auxiliary_data(input: &[u8]) -> IResult<&[u8], (String, String), PltParseError>{
    let(rest, (name, format, value)) = do_parse!(input,
            name: parse_utf8_null_terminated >> format: le_i32 >> value: parse_utf8_null_terminated >>
            ( (name, format, value ))
        )?;
    Ok((rest, (name, value)))
}





#[cfg(test)]
mod tests{
    use crate::PltFormat;
    #[test]
    fn simple_test(){
        let f = PltFormat::open(r".\tests/with_aux.plt");
        println!("{:?}", f);
        assert!(false);
    }
}