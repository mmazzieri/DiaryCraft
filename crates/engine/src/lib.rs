//! DiaryCraft pure logic: file names of the entries, front matter, search, calendar.
//!
//! No I/O and no clock: everything takes its inputs as parameters, so it builds for wasm32 too.
#![forbid(unsafe_code)]

pub mod calendar;
pub mod layout;
pub mod search;
pub mod text;

#[cfg(test)]
mod tests;
