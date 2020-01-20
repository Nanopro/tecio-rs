mod bindings;
mod common;

mod reader;
#[cfg(test)]
mod tests;
mod writer;

extern crate libc;

pub use common::*;
pub use reader::TecReader;
pub use writer::{TecWriter, TecZoneWriter, WriterConfig};
