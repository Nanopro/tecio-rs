



mod common;
mod bindings;
mod reader;
mod writer;


#[cfg(test)]
mod tests;

extern crate libc;



pub use common::*;
pub use reader::TecReader;
pub use writer::TecWriter;
































