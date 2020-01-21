mod plt;
mod szplt;
mod dat;
pub use plt::{PltFormat, PltParseError};
pub use szplt::SzpltFormat;
pub use dat::{DatFormat, DatParseError};
pub trait Format {}
