use crate::*;
// TODO REWRITE TESTS


#[test]
fn test_wrong_filename() {
    assert!(TecReader::open("test123.szplt").is_err());
}


