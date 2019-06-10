
use crate::*;


#[test]
fn test_open_1(){
    let r = TecReader::open("test.szplt").unwrap();
    for z in &r.zones{
        assert_eq!(z.name(), "K=    1");
        assert_eq!(z.zone_type(), ZoneType::Ordered);
    }
} 

#[test]
fn test_wrong_filename(){
    assert!(TecReader::open("test123.szplt").is_err());
} 

#[test]
fn test_big_file(){
    assert!(TecReader::open("/home/ndgorelov/test_data/07_Anim_T-0.783-0.789.szplt").is_ok());
    
} 

#[test]
fn get_values_1(){
    let r = TecReader::open("test.szplt").unwrap();
    let X = r.get_data(1, 1).unwrap();
    println!("{:?}", &X[0..15]);
    assert_eq!(X.len(), 105);
}











