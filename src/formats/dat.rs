use std::{
    borrow::Cow,
    convert::From,
    fs::{read_to_string, File},
    io::{BufRead, Cursor, Read},
    path::Path,
};

use nom::{
    bytes::complete::{tag, tag_no_case, take, take_while, take_while_m_n, take_until},
    character::{is_alphabetic, is_alphanumeric,
                complete::{
                    multispace0, alphanumeric1, newline, multispace1,
                },
    },
    combinator::{cond, map_res, not, opt},
    multi::{count, fold_many0, many0, many1, many_till},
    number::complete::{be_u8, le_f32, le_f64, le_i32, le_u32, double, float, recognize_float},
    sequence::{tuple, separated_pair, delimited},
    IResult,
};

use crate::{
    common::{try_err, Dataset, OrderedZone, Result, TecDataType, TecZone, TecioError, ZoneType, ParseError},
    ClassicFEZone, FaceNeighborMode, FileType, TecData, ValueLocation,
};
use nom::bytes::complete::{is_not, take_till};
use nom::character::complete::{anychar, line_ending, space1, space0};
use nom::sequence::terminated;
use nom::multi::separated_list;
use std::ptr::null_mut;
use crate::formats::plt::HeaderBlock;


#[derive(Clone, Debug)]
pub struct DatFormat {
    pub dataset: Dataset,
    pub zones: Vec<TecZone>,
    pub(crate) data_blocks: Vec<DataBlock>,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum KeyWord {
    Title,
    FileType,
    Variables,
    T,
    ZoneType,
    I,
    J,
    K,
    Nodes,
    Elements,
    Faces,
    TotalNumFaceNodes,
    NumConnectedBoundaryFaces,
    TotalNumBoundaryConnections,
    FaceNeighborConnections,
    DT,
    DataPacking,
    VarLocation,
    VarShareList,
    NV,
    ConnectivityShareZone,
    StrandId,
    SolutionTime,
    ParentZone,
    PassiveVarList,
    AuxData,
}

#[derive(Debug, Clone)]
enum Values<'a> {
    String(&'a str),
    StringList(Vec<&'a str>),
    Location(Vec<(Vec<&'a str>, &'a str)>),
    Number(f64),
}

fn keyword(input: &str) -> IResult<&str, KeyWord, ParseError> {
    let (rest, word) = alphanumeric1(input)?;
    match word {
        "ZONE" => {
            Err(nom::Err::Error(ParseError::EndOfHeader))
        }
        "TITLE" => {
            Ok((rest, KeyWord::Title))
        }
        "VARIABLES" => {
            Ok((rest, KeyWord::Variables))
        }
        "FILETYPE" => {
            Ok((rest, KeyWord::FileType))
        }
        "T" => {
            Ok((rest, KeyWord::T))
        }
        "ZONETYPE" => {
            Ok((rest, KeyWord::ZoneType))
        }
        "I" => {
            Ok((rest, KeyWord::I))
        }
        "J" => {
            Ok((rest, KeyWord::J))
        }
        "K" => {
            Ok((rest, KeyWord::K))
        }
        "Nodes" | "N" | "NODES" => {
            Ok((rest, KeyWord::Nodes))
        }
        "Elements" | "ELEMENTS" | "E" => {
            Ok((rest, KeyWord::Elements))
        }
        "FACES" => {
            Ok((rest, KeyWord::Faces))
        }
        "TOTALNUMFACENODES" => {
            Ok((rest, KeyWord::TotalNumFaceNodes))
        }
        "NUMCONNECTEDBOUNDARYFACES" => {
            Ok((rest, KeyWord::NumConnectedBoundaryFaces))
        }
        "TOTALNUMBOUNDARYCONNECTIONS" => {
            Ok((rest, KeyWord::TotalNumBoundaryConnections))
        }
        "FACENEIGHBORCONNECTIONS" => {
            Ok((rest, KeyWord::FaceNeighborConnections))
        }
        "DT" => {
            Ok((rest, KeyWord::DT))
        }
        "DATAPACKING" => {
            Ok((rest, KeyWord::DataPacking))
        }
        "VARLOCATION" => {
            Ok((rest, KeyWord::VarLocation))
        }
        "VARSHARELIST" => {
            Ok((rest, KeyWord::VarShareList))
        }
        "NV" => {
            Ok((rest, KeyWord::NV))
        }
        "CONNECTIVITYSHAREZONE" => {
            Ok((rest, KeyWord::ConnectivityShareZone))
        }
        "STRANDID" => {
            Ok((rest, KeyWord::StrandId))
        }
        "SOLUTIONTIME" => {
            Ok((rest, KeyWord::SolutionTime))
        }
        "PARENTZONE" => {
            Ok((rest, KeyWord::ParentZone))
        }
        "PASSIVEVARLIST" => {
            Ok((rest, KeyWord::PassiveVarList))
        }
        "AUXDATA" => {
            Ok((rest, KeyWord::AuxData))
        }

        _ => {
            Err(nom::Err::Error(ParseError::WrongHeaderTag))
        }
    }
}


fn word(input: &str) -> IResult<&str, &str, ParseError> {
    let (r, v) = delimited(tag("\""), take_while(|c| c != '"'), tag("\""))(input)?;
    Ok((r, v))
}

fn number(input: &str) -> IResult<&str, f64, ParseError> {
    let (r, v) = double(input)?;
    Ok((r, v))
}

fn value(input: &str) -> IResult<&str, &str, ParseError> {
    alphanumeric1(input)
}

fn separ_comma(input: &str) -> IResult<&str, (), ParseError> {
    let (r, _) = do_parse!(input,
                  space0
          >>     char!(',')
          >> opt!(line_ending)
          >>      space0
          >>
        ( () )
     )?;
    Ok((r, ()))
}

fn separ(input: &str) -> IResult<&str, (), ParseError> {
    let (r, _) = do_parse!(input,
                  space0
          >>      opt!(char!(','))
          >>       opt!(space0)
          >>      opt!(line_ending)
          >>
        ( () )
     )?;
    Ok((r, ()))
}


fn var_location(input: &str) -> IResult<&str, Values, ParseError> {
    fn sp1(input: &str) -> IResult<&str, (Vec<&str>, &str), ParseError> {
        fn pattern(input: &str) -> IResult<&str, Vec<&str>, ParseError> {
            fn var_specifier(input: &str) -> IResult<&str, &str, ParseError> {
                take_while::<_, &str, ParseError>(|c: char| c.is_numeric() || c == '-')(input)
            }

            let (r, v) = delimited(
                tag("["),
                separated_list(separ_comma, var_specifier),
                tag("]"),
            )(input).unwrap();

            Ok((r, v))
        }
        let (r, v) = do_parse!(input,
             pat: pattern
              >>      multispace0
              >>      char!('=')
              >>      multispace0
             >> val: value
             >>      multispace0 >>
            ( (pat, val) )
      )?;

        Ok((r, v))
    }

    let (r, v) = delimited(
        tag("("),
        separated_list(
            separ_comma,
            sp1,
        ),
        tag(")"),
    )(input).unwrap();

    Ok((r, Values::Location(v)))
}


fn keyword_parser(keyword: KeyWord, input: &str) -> IResult<&str, Values, ParseError>
{
    use KeyWord::*;
    match keyword {
        KeyWord::Title | KeyWord::T => {
            let (r, s) = word(input)?;
            Ok((r, Values::String(s)))
        }
        KeyWord::Variables => {
            let (r, v) = separated_list(
                separ,
                word,
            )(input)?;

            Ok((r, Values::StringList(v)))
        }
        KeyWord::FileType | ZoneType | DataPacking => {
            let (r, s) = value(input)?;
            Ok((r, Values::String(s)))
        }
        StrandId | I | J | K | Nodes | Elements | SolutionTime => {
            let (r, s) = number(input)?;
            Ok((r, Values::Number(s)))
        }
        DT => {
            let (r, v) = delimited(
                tag("("),
                terminated(
                    separated_list(space1, value),
                    space0,
                ),
                tag(")"),
            )(input)?;

            Ok((r, Values::StringList(v)))
        }
        VarLocation => {
            let (r, v) = var_location(input)?;

            Ok((r, v))
        }
        _ => unimplemented!()
    }
}


fn key_value(input: &str) -> IResult<&str, (KeyWord, Values), ParseError> {
    let (rest, key) = do_parse!(input,
             multispace0
              >> key: keyword
              >>      multispace0
              >>      char!('=')
              >>      multispace0 >>
            ( key )
      )?;

    let (rest, value) = keyword_parser(key, rest)?;
    Ok((rest, (key, value)))
}

fn line_with_spaces(input: &str) -> IResult<&str, (), ParseError> {
    let (r, _) = do_parse!(input,
                  space0
          >> line_ending
          >>      space0
          >>
        ( () )
     )?;
    Ok((r, ()))
}

fn parse_header(input: &str) -> IResult<&str, DatHeader, ParseError> {
    let (rest, values) = many0(terminated(key_value, opt(line_with_spaces)))(input)?;

    let title = values.iter().find_map(|(key, value)| {
        match key {
            KeyWord::Title => {
                match value {
                    Values::String(s) => Some(s.to_owned()),
                    _ => None,
                }
            }
            _ => None,
        }
    }).unwrap_or("Dataset").to_owned();

    let filetype = values.iter().find_map(|(key, value)| {
        match key {
            KeyWord::FileType => {
                match value {
                    Values::String(s) => {
                        match *s {
                            "FULL" => Some(FileType::Full),
                            "GRID" => Some(FileType::GridOnly),
                            "SOLUTION" => Some(FileType::SolutionOnly(null_mut())),
                            _ => None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }).unwrap_or(FileType::Full);

    let var_list = values.iter().find_map(|(key, value)| {
        match key {
            KeyWord::Variables => {
                match value {
                    Values::StringList(s) => Some(s.iter().map(|&s| s.to_owned()).collect()),
                    _ => None,
                }
            }
            _ => None,
        }
    }).unwrap();

    let header = DatHeader {
        title,
        filetype,
        var_list,
    };


    Ok((rest, header))
}

use std::collections::HashMap;


fn resolve_var_location(var_loc: &[(Vec<&str>, &str)], var_num: usize) -> Vec<ValueLocation> {
    let mut locations = vec![ValueLocation::Nodal; var_num];

    var_loc.iter().for_each(|(patterns, location)| {
        let location = match *location {
            "CELLCENTERED" => ValueLocation::CellCentered,
            "NODAL" => ValueLocation::Nodal,
            x => panic!("Unknown value location: {:?}", x)
        };

        patterns.iter().for_each(|&pattern| {
            if pattern.contains("-") {
                let mut sp = pattern.split("-");
                let first: usize = sp.next().unwrap().parse().expect("Bad number!");
                let last: usize = sp.next().unwrap().parse().expect("Bad number!");
                if last > first && first > 0 && last <= var_num {
                    for i in first..=last {
                        locations[i] = location;
                    }
                } else {
                    panic!("Var location num outside of var num count!")
                }
            } else {
                //2
                let num = pattern.parse::<usize>().expect("Bad number for zone location");
                if num <= var_num {
                    locations[num - 1] = location;
                } else {
                    panic!("Var location num outside of var num count!")
                }
            }
        })
    });
    locations
}

fn float_sep(input: &str) -> IResult<&str, (), ParseError> {
    do_parse!(input,
                  space0
          >>      opt!(line_ending)
          >> space0
          >>
        ( () )
     )
}

fn float_with_separ(input: &str) -> IResult<&str, &str, ParseError> {
    do_parse!(input,
            opt!(float_sep) >>
        f: recognize_float >>
        float_sep >>
        ( f )
    )
}

fn parse_zone(input: &str, var_num: usize) -> IResult<&str, (TecZone, DataBlock), ParseError> {
    let (rest, tag) = tag("ZONE")(input)?;
    let (rest, values) = many0(terminated(key_value, separ))(rest)?;
    let values: HashMap<KeyWord, Values> = values.into_iter().collect();

    let zonetype = values.get(&KeyWord::ZoneType).map(|t| {
        match t {
            Values::String(t) => {
                match t.to_lowercase().as_str() {
                    "ordered" => ZoneType::Ordered,
                    "felineseg" => ZoneType::FEPolygon,
                    "fetriangle" => ZoneType::FETriangle,
                    "fequadrilateral" => ZoneType::FEQuad,
                    "fetetrahedron" => ZoneType::FETetra,
                    "febrick" => ZoneType::FEBrick,
                    "fepolygon" => ZoneType::FEPolygon,
                    "fepolyhedral" => ZoneType::FEPolyhedron,
                    x => panic!("Unknown zone type: {:?}!", x)
                }
            }
            x => panic!("Unknown zone type: {:?}!", x)
        }
    }).unwrap_or(ZoneType::Ordered);

    let get_number = |key| values.get(&key).map(|v| {
        match v {
            Values::Number(n) => *n,
            x => panic!("Expected number, got {:?}", x)
        }
    }).unwrap_or(1.0);

    let zone_title = values.get(&KeyWord::T).map(|x| match x {
        Values::String(name) => (*name).to_owned(),
        _ => unimplemented!()
    }).unwrap_or_else(|| format!("Unnamed zone"));
    let solution_time = get_number(KeyWord::SolutionTime);
    let strand_id = get_number(KeyWord::StrandId) as _;
    let var_location = values.get(&KeyWord::VarLocation).map(|v| {
        match v {
            Values::Location(l) => resolve_var_location(l.as_slice(), var_num),
            x => panic!("Expected list of var locations, got: {:?}!", x)
        }
    }).unwrap_or_else(|| vec![ValueLocation::Nodal; var_num]);

    let var_types = values.get(&KeyWord::DT).map(|v| {
        match v {
            Values::StringList(list) => {
                list.iter().map(|n| match *n {
                    "SINGLE" => TecDataType::F32,
                    "DOUBLE" => TecDataType::F64,
                    x => panic!("Expected var type, got: {:?}!", x)
                }).collect()
            }
            x => panic!("Expected list of var types, got: {:?}!", x)
        }
    }).unwrap_or_else(|| vec![TecDataType::F64; var_num]);

    let data_pack = values.get(&KeyWord::DataPacking).map(|t| {
        match t {
            Values::String(t) => {
                match t.to_lowercase().as_str() {
                    "point" => DataPacking::Point,
                    "block" => DataPacking::Block,

                    x => panic!("Unknown zone type: {:?}!", x)
                }
            }
            x => panic!("Unknown zone type: {:?}!", x)
        }
    }).unwrap_or(DataPacking::Block);


    let zone = match zonetype {
        ZoneType::Ordered => {
            let i_max = get_number(KeyWord::I) as i64;
            let j_max = get_number(KeyWord::J) as i64;
            let k_max = get_number(KeyWord::K) as i64;


            let zone = TecZone::Ordered(OrderedZone {
                name: zone_title,
                id: 0,
                solution_time,
                strand: strand_id,
                i_max,
                j_max,
                k_max,
                var_location,
                var_types: Some(var_types),
            });


            zone
        }
        ZoneType::FEBrick
        | ZoneType::FETetra
        | ZoneType::FEQuad
        | ZoneType::FETriangle
        | ZoneType::FELine => {
            let cells = get_number(KeyWord::Elements) as i64;
            let nodes = get_number(KeyWord::Nodes) as i64;

            TecZone::ClassicFE(ClassicFEZone {
                name: zone_title,
                zone_type: zonetype,
                id: 0,
                solution_time,
                strand: strand_id,
                nodes,
                cells,
                var_location,
                var_types: Some(var_types),
            })
        }
        _ => unimplemented!()
    };
    let mut rest = rest;
    let mut data = Vec::with_capacity(var_num);
    let min_max = vec![(0.0, 0.0); var_num];


    match data_pack {
        DataPacking::Block => {
            for (num, (loc, ty)) in zone.var_locs().iter().zip(zone.data_types().unwrap().iter()).enumerate() {
                let c = match loc {
                    ValueLocation::Nodal => {
                        zone.node_count()
                    }
                    ValueLocation::CellCentered => {
                        zone.cell_count()
                    }
                };

                let (r, _) = float_sep(rest)?; // ??????????
                rest = r;

                let (r, x) = count(float_with_separ, c)(rest)?;

                rest = r;
                let d = match ty {
                    TecDataType::F32 => {
                        TecData::from(
                            x.into_iter().map(|p| p.parse::<f32>().unwrap()).collect::<Vec<_>>()
                        )
                    }
                    TecDataType::F64 => {
                        TecData::from(
                            x.into_iter().map(|p| p.parse::<f64>().unwrap()).collect::<Vec<_>>()
                        )
                    }
                    TecDataType::I32 => {
                        TecData::from(
                            x.into_iter().map(|p| p.parse::<i32>().unwrap()).collect::<Vec<_>>()
                        )
                    }
                    TecDataType::I16 => {
                        TecData::from(
                            x.into_iter().map(|p| p.parse::<i16>().unwrap()).collect::<Vec<_>>()
                        )
                    }
                    _ => unimplemented!()
                };
                data.push((num + 1, d));
            }
        }
        DataPacking::Point => {
            let nodes = zone.node_count();

            for (num, ty) in zone.data_types().unwrap().iter().enumerate() {
                match ty {
                    TecDataType::F32 => {
                        data.push((num +1, TecData::F32(Cow::Owned(Vec::with_capacity(nodes)))))
                    }
                    TecDataType::F64 => {
                        data.push((num +1, TecData::F64(Cow::Owned(Vec::with_capacity(nodes)))))
                    }
                    _ => unimplemented!()
                }
            }

            for point in 0..nodes {
                let (r, x) = count(float_with_separ, var_num)(rest)?;
                rest = r;
                for (num, (d, ty)) in x.into_iter().zip(zone.data_types().unwrap().iter()).enumerate() {
                    match ty {
                        TecDataType::F32 => {
                           match &mut data[num].1 {
                               TecData::F32(Cow::Owned(v)) => {
                                   v.push(d.parse().unwrap());
                               }
                               _ => unreachable!()
                           }
                        }
                        TecDataType::F64 => {
                            match &mut data[num].1 {
                                TecData::F64(Cow::Owned(v)) => {
                                    v.push(d.parse().unwrap());
                                },
                                _ => unreachable!()
                            }
                        },
                        _ => unimplemented!()
                    }
                }
            }
        }
    }


    let connectivity = match &zone {
        TecZone::ClassicFE(fe) => {
            let (r, v) = count(float_with_separ, fe.num_connections())(rest)?;
            rest = r;

            Some(TecData::from(
                v.into_iter().map(|p| p.parse::<i32>().unwrap()).collect::<Vec<_>>()
            ))
        }
        _ => None
    };


    let block = DataBlock {
        data,
        connectivity,
        min_max,
    };
    Ok((rest, (zone, block)))
}


impl DatFormat {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = read_to_string(path)?;
        let rest = file.as_str();


        let (rest, header) = parse_header(rest)?;

        let (rest, z) = many0(|rest| parse_zone(rest, header.var_list.len()))(rest)?;


        let dataset = Dataset {
            num_variables: header.var_list.len() as _,
            num_zones: z.len() as _,
            title: header.title,
            var_names: header.var_list,
        };
        let mut zones = Vec::with_capacity(z.len());
        let mut data_blocks = Vec::with_capacity(z.len());
        z.into_iter().for_each(|(zone, block)| {
            zones.push(zone);
            data_blocks.push(block);
        });
        Ok(
            Self {
                dataset,
                zones,
                data_blocks,
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct DatHeader {
    title: String,
    filetype: FileType,
    var_list: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum DataPacking {
    Point,
    Block,
}


#[cfg(test)]
mod tests {
    use super::DatFormat;

    #[test]
    fn simple_test() {
        let r = DatFormat::open(r"./tests/ice.dat");
        assert!(r.is_ok());
        if let Ok(r) = r {
            let c = &r.zones;
            println!("{:?}", c);
        }
    }
}







