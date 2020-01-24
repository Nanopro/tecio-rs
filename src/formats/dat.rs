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
    number::complete::{be_u8, le_f32, le_f64, le_i32, le_u32, double},
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

#[derive(Debug, Copy, Clone)]
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
    println!("Word: {}", word);
    match word {
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
        "Nodes" => {
            Ok((rest, KeyWord::Nodes))
        }
        "Elements" => {
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
          >>       opt!(multispace0)
          >>      opt!(line_ending)
          >>
        ( () )
     )?;
    Ok((r, ()))
}


fn var_location(input: &str) -> IResult<&str, Values, ParseError>{

    fn sp1(input: &str) -> IResult<&str, (Vec<&str>, &str), ParseError>{

        fn pattern(input: &str) -> IResult<&str, Vec<&str>, ParseError>{
            fn var_specifier(input: &str) -> IResult<&str, &str, ParseError>{
                take_while::<_, &str, ParseError>(|c: char| c.is_numeric() || c == '-')(input)
            }
            println!("{:?}", &input[0..5]);
            let (r, v) = delimited(
                tag("["),
                separated_list(separ_comma, var_specifier),
                tag("]"),
            )(input).unwrap();
            println!("{:?}", v);
            Ok((r, v))
        }
        do_parse!(input,
             pat: pattern
              >>      multispace0
              >>      char!('=')
              >>      multispace0
             >> val: value
             >>      multispace0 >>
            ( (pat, val) )
      )
    }

    let (r, v) = delimited(
        tag("("),
        separated_list(
            separ,
            sp1
        ),
        tag(")"),
    )(input)?;

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
                    space0
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

fn parse_header(input: &str) -> IResult<&str, DatHeader, ParseError> {
    let (rest, values) = many0(terminated(key_value, opt(line_ending)))(input)?;

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

fn parse_data(input: &str) -> IResult<&str, ZoneRecord, ParseError> {
    let (rest, tag) = tag("ZONE")(input)?;
    let (rest, values) = many0(terminated(key_value, separ))(rest)?;
    println!("{:#?}", values);
    unimplemented!()
}


impl DatFormat {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = read_to_string(path)?;
        let rest = file.as_str();


        let (rest, c) = parse_header(rest)?;
        println!("{:#?}", c);
        let (rest, c) = parse_data(rest)?;
        println!("{:#?}", c);


        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct DatHeader {
    title: String,
    filetype: FileType,
    var_list: Vec<String>,
}

#[derive(Debug, Clone)]
struct ZoneRecord {
    header: ZoneHeader,
    data: DataBlock,
    footer: ZoneFooter,
}

#[derive(Debug, Clone)]
struct ZoneHeader {
    title: String
}

#[derive(Debug, Clone)]
struct ZoneFooter {}


#[cfg(test)]
mod tests {
    use super::DatFormat;

    #[test]
    fn simple_test() {
        let r = DatFormat::open("./tests/heated_fin.dat");
        assert!(r.is_ok());
    }
}







