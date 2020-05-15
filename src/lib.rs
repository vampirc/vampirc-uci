//! The Vampirc project is a chess engine written in Rust. `vampirc-uci` is a crate that handles the parsing of the
//! [Universal Chess Interface (UCI) protocol](https://en.wikipedia.org/wiki/Universal_Chess_Interface), a way for a
//! chess engine to communicate with a GUI.
//!
//! To parse the UCI messages, it uses the [PEST parser](https://github.com/pest-parser/pest). The corresponding PEG
//! grammar is available [here](https://github.com/vampirc/vampirc-uci/blob/master/res/uci.pest).
//!
//! See the [README.md](https://github.com/vampirc/vampirc-uci/blob/master/README.md) file for usage instructions.


#[cfg(feature = "chess")]
extern crate chess;
extern crate pest;
#[macro_use]
extern crate pest_derive;

#[cfg(feature = "chess")]
pub use chess::ChessMove;
#[cfg(feature = "chess")]
pub use chess::Piece;
#[cfg(feature = "chess")]
pub use chess::Square;
pub use pest::error::Error;

pub use self::parser::parse;
pub use self::parser::parse_one;
pub use self::parser::parse_strict;
pub use self::parser::parse_with_unknown;
pub use self::parser::Rule;
pub use self::uci::ByteVecUciMessage;
pub use self::uci::CommunicationDirection;
pub use self::uci::MessageList;
pub use self::uci::ProtectionState;
pub use self::uci::Serializable;
pub use self::uci::UciFen;
pub use self::uci::UciInfoAttribute;
pub use self::uci::UciMessage;
#[cfg(not(feature = "chess"))]
pub use self::uci::UciMove;
pub use self::uci::UciOptionConfig;
#[cfg(not(feature = "chess"))]
pub use self::uci::UciPiece;
pub use self::uci::UciSearchControl;
#[cfg(not(feature = "chess"))]
pub use self::uci::UciSquare;
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
