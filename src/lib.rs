//! This is documentation for the `vampirc-uci` crate.
//!
//! The Vampirc project is a chess engine written in Rust. `vampirc-uci` is a crate that handles the parsing of the
//! [Universal Chess Interface (UCI) protocol](https://en.wikipedia.org/wiki/Universal_Chess_Interface), a way for a
//! chess engine to communicate with a GUI.
//!
//! See the README.md file for usage instructions.


extern crate pest;
#[macro_use]
extern crate pest_derive;

pub mod uci;
pub mod parser;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
