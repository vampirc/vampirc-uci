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

pub use pest::error::Error;

pub use self::parser::parse;
pub use self::parser::Rule;
pub use self::uci::CommunicationDirection;
pub use self::uci::MessageList;
pub use self::uci::UciFen;
pub use self::uci::UciMessage;
pub use self::uci::UciMove;
pub use self::uci::UciPiece;
pub use self::uci::UciSearchControl;
pub use self::uci::UciSquare;
// Re–exports
pub use self::uci::UciTimeControl;

pub mod uci;
pub mod parser;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
