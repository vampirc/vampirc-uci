//! The `uci` module contains the definitions that represent UCI protocol messages.
//!
//! Usually, these messages will be obtained by calling the `parse` method of the `parser` module, but you can always
//! construct them in code and then print them to the standard output to communicate with the engine or GUI.


use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::uci::UciTimeControl::MoveTime;
use crate::uci::UciTimeControl::TimeLeft;

/// Specifies whether a message is engine- or GUI-bound.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CommunicationDirection {
    /// An engine-bound message.
    GuiToEngine,

    /// A GUI-bound message
    EngineToGui,
}

/// An enumeration type containing representations for all messages supported by the UCI protocol.
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum UciMessage {
    /// The `uci` engine-bound message.
    Uci,

    /// The `debug` engine-bound message. Its internal property specifies whether debug mode should be enabled (`true`),
    /// or disabled (`false`).
    Debug(bool),

    /// The `isready` engine-bound message.
    IsReady,

    /// The `register` engine-bound message.
    Register {
        /// The `register later` engine-bound message.
        later: bool,

        /// The name part of the `register <code> <name>` engine-bound message.
        name: Option<String>,

        /// The code part of the `register <code> <name>` engine-bound message.
        code: Option<String>,
    },

    /// The `position` engine-bound message.
    Position {
        /// If `true`, it denotes the starting chess position. Generally, if this property is `true`, then the value of
        /// the `fen` property will be `None`.
        startpos: bool,

        /// The [FEN format](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation) representation of a chess
        /// position.
        fen: Option<UciFen>,

        /// A list of moves to apply to the position.
        moves: Vec<UciMove>,
    },

    /// The `setoption` engine-bound message.
    SetOption {
        /// The name of the option to set.
        name: String,

        /// The value of the option to set. If the option has no value, this should be `None`.
        value: Option<String>,
    },

    /// The `ucinewgame` engine-bound message.
    UciNewGame,

    /// The `stop` engine-bound message.
    Stop,

    /// The `ponderhit` engine-bound message.
    PonderHit,

    /// The `quit` engine-bound message.
    Quit,

    /// The `go` engine-bound message.
    Go {
        /// Time-control-related `go` parameters (sub-commands).
        time_control: Option<UciTimeControl>,

        /// Search-related `go` parameters (sub-commands).
        search_control: Option<UciSearchControl>,
    },
}

impl UciMessage {
    /// Constructs a `register later` [UciMessage::Register](enum.UciMessage.html#variant.Register)  message.
    pub fn register_later() -> UciMessage {
        UciMessage::Register {
            later: true,
            name: None,
            code: None,
        }
    }

    /// Constructs a `register <code> <name>` [UciMessage::Register](enum.UciMessage.html#variant.Register) message.
    pub fn register_code(name: &str, code: &str) -> UciMessage {
        UciMessage::Register {
            later: false,
            name: Some(name.to_string()),
            code: Some(code.to_string()),
        }
    }

    /// Construct a `go ponder` [UciMessage::Register](enum.UciMessage.html#variant.Go) message.
    pub fn go_ponder() -> UciMessage {
        UciMessage::Go {
            search_control: None,
            time_control: Some(UciTimeControl::Ponder)
        }
    }

    /// Constructs a `go infinite` [UciMessage::Register](enum.UciMessage.html#variant.Go) message.
    pub fn go_infinite() -> UciMessage {
        UciMessage::Go {
            search_control: None,
            time_control: Some(UciTimeControl::Infinite)
        }
    }

    /// Constructs a `go movetime <milliseconds>` [UciMessage::Register](enum.UciMessage.html#variant.Go) message, with
    /// `milliseconds` as the argument.
    pub fn go_movetime(milliseconds: u64) -> UciMessage {
        UciMessage::Go {
            search_control: None,
            time_control: Some(UciTimeControl::MoveTime(milliseconds))
        }
    }

    /// Serializes the command into a String.
    ///
    /// # Examples
    /// ```
    /// use vampirc_uci::uci::UciMessage;
    ///
    /// println!("{}", UciMessage::Uci.serialize()); // Should print `uci`.
    /// ```
    pub fn serialize(&self) -> String {
        match self {
            UciMessage::Debug(on) => if *on { String::from("debug on") } else { String::from("debug off") },
            UciMessage::Register { later, name, code } => {
                if *later {
                    return String::from("register later");
                }

                let mut s: String = String::from("register ");
                if let Some(n) = name {
                    s += format!("name {}", *n).as_str();
                    if code.is_some() {
                        s += " ";
                    }
                }
                if let Some(c) = code {
                    s += format!("code {}", *c).as_str();
                }

                s
            }
            UciMessage::Position { startpos, fen, moves } => {
                let mut s = String::from("position ");
                if *startpos {
                    s += String::from("startpos").as_str();
                } else if let Some(uci_fen) = fen {
                    s += format!("fen {}", uci_fen.as_str()).as_str();
                }

                if moves.len() > 0 {
                    s += String::from(" moves ").as_str();

                    for (i, m) in moves.into_iter().enumerate() {
                        s += format!("{}", *m).as_str();

                        if i < moves.len() - 1 {
                            s += String::from(" ").as_str();
                        }
                    }
                }

                s
            }
            UciMessage::SetOption { name, value } => {
                let mut s: String = String::from(format!("setoption name {}", name));

                if let Some(val) = value {
                    s += format!(" value {}", *val).as_str();
                }

                s
            }
            UciMessage::Go { time_control, search_control } => {
                let mut s = String::from("go ");

                if let Some(tc) = time_control {
                    match tc {
                        UciTimeControl::Infinite => { s += "infinite "; }
                        UciTimeControl::Ponder => { s += "ponder "; }
                        UciTimeControl::MoveTime(milliseconds) => {
                            s += format!("movetime {} ", *milliseconds).as_str();
                        }
                        UciTimeControl::TimeLeft { white_time, black_time, white_increment, black_increment, moves_to_go } => {
                            if let Some(wt) = white_time {
                                s += format!("wtime {} ", *wt).as_str();
                            }

                            if let Some(bt) = black_time {
                                s += format!("bt {} ", *bt).as_str();
                            }

                            if let Some(wi) = white_increment {
                                s += format!("winc {} ", *wi).as_str();
                            }

                            if let Some(bi) = black_increment {
                                s += format!("binc {} ", *bi).as_str();
                            }

                            if let Some(mtg) = moves_to_go {
                                s += format!("movestogo {} ", *mtg).as_str();
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(sc) = search_control {
                    if let Some(depth) = sc.depth {
                        s += format!("depth {} ", depth).as_str();
                    }

                    if let Some(nodes) = sc.nodes {
                        s += format!("nodes {} ", nodes).as_str();
                    }

                    if let Some(mate) = sc.mate {
                        s += format!("mate {} ", mate).as_str();
                    }

                    if !sc.search_moves.is_empty() {
                        s += " searchmoves ";
                        for m in &sc.search_moves {
                            s += format!("{} ", m).as_str();
                        }
                    }
                }

                s
            }
            UciMessage::Uci => "uci".to_string(),
            UciMessage::IsReady => "isready".to_string(),
            UciMessage::UciNewGame => "ucinewgame".to_string(),
            UciMessage::Stop => "stop".to_string(),
            UciMessage::PonderHit => "ponderhit".to_string(),
            UciMessage::Quit => "quit".to_string()
        }
    }

    /// Returns whether the command was meant for the engine or for the GUI.
    fn direction(&self) -> CommunicationDirection {
        match self {
            UciMessage::Uci |
            UciMessage::Debug(..) |
            UciMessage::IsReady |
            UciMessage::Register { .. } |
            UciMessage::Position { .. } |
            UciMessage::SetOption { .. } |
            UciMessage::UciNewGame |
            UciMessage::Stop |
            UciMessage::PonderHit |
            UciMessage::Quit |
            UciMessage::Go { .. } => CommunicationDirection::GuiToEngine,
//            _ => CommunicationDirection::EngineToGui
        }
    }

    /// If this `UciMessage` is a `UciMessage::SetOption` and the value of that option is a `bool`, this method returns
    /// the `bool` value, otherwise it returns `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            UciMessage::SetOption { value, .. } => {
                if let Some(val) = value {
                    let pr = str::parse(val.as_str());
                    if pr.is_ok() {
                        return Some(pr.unwrap());
                    }
                }

                None
            }
            _ => None
        }
    }

    /// If this `UciMessage` is a `UciMessage::SetOption` and the value of that option is an integer, this method
    /// returns the `i32` value of the integer, otherwise it returns `None`.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            UciMessage::SetOption { value, .. } => {
                if let Some(val) = value {
                    let pr = str::parse(val.as_str());
                    if pr.is_ok() {
                        return Some(pr.unwrap());
                    }
                }

                None
            }
            _ => None
        }
    }
}

impl Display for UciMessage {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.serialize())
    }
}

/// This enum represents the possible variants of the `go` UCI message that deal with the chess game's time controls
/// and the engine's thinking time.
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum UciTimeControl {
    /// The `go ponder` message.
    Ponder,

    /// The `go infinite` message.
    Infinite,

    /// The information about the game's time controls.
    TimeLeft {
        /// White's time on the clock, in milliseconds.
        white_time: Option<u64>,

        /// Black's time on the clock, in milliseconds.
        black_time: Option<u64>,

        /// White's increment per move, in milliseconds.
        white_increment: Option<u64>,

        /// Black's increment per move, in milliseconds.
        black_increment: Option<u64>,

        /// The number of moves to go to the next time control.
        moves_to_go: Option<u8>,
    },

    /// Specifies how much time the engine should think about the move, in milliseconds.
    MoveTime(u64)
}

impl UciTimeControl {
    /// Returns a `UciTimeControl::TimeLeft` with all members set to `None`.
    pub fn time_left() -> UciTimeControl {
        TimeLeft {
            white_time: None,
            black_time: None,
            white_increment: None,
            black_increment: None,
            moves_to_go: None
        }
    }
}

/// A struct that controls the engine's (non-time-related) search settings.
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciSearchControl {
    /// Limits the search to these moves.
    pub search_moves: Vec<UciMove>,

    /// Search for mate in this many moves.
    pub mate: Option<u8>,

    /// Search to this ply depth.
    pub depth: Option<u8>,

    /// Search no more than this many nodes (positions).
    pub nodes: Option<u64>,
}

impl UciSearchControl {
    /// Creates an `UciSearchControl` with `depth` set to the parameter and everything else set to empty or `None`.
    pub fn depth(depth: u8) -> UciSearchControl {
        UciSearchControl {
            search_moves: vec![],
            mate: None,
            depth: Some(depth),
            nodes: None,
        }
    }

    /// Creates an `UciSearchControl` with `mate` set to the parameter and everything else set to empty or `None`.
    pub fn mate(mate: u8) -> UciSearchControl {
        UciSearchControl {
            search_moves: vec![],
            mate: Some(mate),
            depth: None,
            nodes: None,
        }
    }

    /// Creates an `UciSearchControl` with `nodes` set to the parameter and everything else set to empty or `None`.
    pub fn nodes(nodes: u64) -> UciSearchControl {
        UciSearchControl {
            search_moves: vec![],
            mate: None,
            depth: None,
            nodes: Some(nodes),
        }
    }

    /// Returns `true` if all of the struct's settings are either `None` or empty.
    pub fn is_empty(&self) -> bool {
        self.search_moves.is_empty() && self.mate.is_none() && self.depth.is_none() && self.nodes.is_none()
    }
}

impl Default for UciSearchControl {
    /// Creates an empty `UciSearchControl`.
    fn default() -> Self {
        UciSearchControl {
            search_moves: vec![],
            mate: None,
            depth: None,
            nodes: None,
        }
    }
}

//
//
//pub enum Argument {
//
//    Parameter(String),
//    Option {
//        name: String,
//        value:
//    }
//
//}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum OptionType {
    Check,
    Spin,
    Combo,
    Button,
    String,
}

impl Display for OptionType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            OptionType::Check => write!(f, "{}", "check"),
            OptionType::Spin => write!(f, "{}", "spin"),
            OptionType::Combo => write!(f, "{}", "combo"),
            OptionType::Button => write!(f, "{}", "button"),
            OptionType::String => write!(f, "{}", "string"),
        }
    }
}


//#[derive(Clone, Eq, PartialEq, Debug)]
//pub struct UciOption<T> where T: Display + Debug {
//    name: String,
//    option_type: OptionType,
//    min: Option<T>,
//    max: Option<T>,
//    default: T,
//    var: Vec<T>,
//}
//
//impl<T> UciOption<T> where T: Display + Debug {}
//
//impl<T> Display for UciOption<T> where T: Display + Debug {
//    fn fmt(&self, f: &mut Formatter) -> FmtResult {
//        write!(f, "{}", self.serialize())
//    }
//}
//
//impl<'a, T> UciMessage<'a> for UciOption<T> where T: Display + Debug {
//    fn name(&'a self) -> &'a str {
//        self.name.as_str()
//    }
//
//    fn serialize(&self) -> String {
//        let mut s: String = String::from("option name ");
//        s += self.name.as_str();
//        s += " type ";
//        s += format!(" type {} ", self.option_type).as_str();
//        s += format!(" default {} ", self.default).as_str();
//
//        if let Some(min) = &self.min {
//            s += format!(" min {}", *min).as_str();
//        }
//
//        if let Some(max) = &self.max {
//            s += format!(" max {}", *max).as_str();
//        }
//
//        if self.var.len() > 0 {
//            for (i, var) in (&self.var).into_iter().enumerate() {
//                s += format!(" var {}", *var).as_str();
//                if i < self.var.len() - 1 {
//                    s += " ";
//                }
//            }
//        }
//
//        s
//    }
//
//    fn direction(&self) -> CommunicationDirection {
//        CommunicationDirection::EngineToGui
//    }
//}

/// An enum representing the chess piece types.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum UciPiece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl UciPiece {
    /// Returns a character representing a piece in UCI move notation. Used for specifying promotion in moves.
    ///
    /// `n` – knight
    /// `b` - bishop
    /// `r` - rook
    /// `q` - queen
    /// `k` - king
    /// `None` - pawn
    pub fn as_char(self) -> Option<char> {
        match self {
            UciPiece::Pawn => None,
            UciPiece::Knight => Some('n'),
            UciPiece::Bishop => Some('b'),
            UciPiece::Rook => Some('r'),
            UciPiece::Queen => Some('q'),
            UciPiece::King => Some('k')
        }
    }
}

impl From<&str> for UciPiece {
    /// Creates a `UciPiece` from a `&str`, according to these rules:
    ///
    /// `"n"` - Knight
    /// `"p"` - Pawn
    /// `"b"` - Bishop
    /// `"r"` - Rook
    /// `"k"` - King
    /// `"q"` - Queen
    ///
    /// Works with uppercase letters as well.
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "n" => UciPiece::Knight,
            "p" => UciPiece::Pawn,
            "b" => UciPiece::Bishop,
            "r" => UciPiece::Rook,
            "k" => UciPiece::King,
            "q" => UciPiece::Queen,
            _ => panic!(format!("No piece mapping for {}", s))
        }
    }
}

/// A representation of a chessboard square.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciSquare {
    /// The file. A character in the range of `a..h`.
    pub file: char,

    /// The rank. A number in the range of `1..8`.
    pub rank: u8,
}

impl UciSquare {
    /// Create a `UciSquare` from file character and a rank number.
    pub fn from(file: char, rank: u8) -> UciSquare {
        UciSquare {
            file,
            rank,
        }
    }
}

impl Display for UciSquare {
    /// Formats the square in the regular notation (as in, `e4`).
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}{}", self.file, self.rank)
    }
}

impl Default for UciSquare {
    /// Default square is an invalid square with a file of `\0` and the rank of `0`.
    fn default() -> Self {
        UciSquare {
            file: '\0',
            rank: 0,
        }
    }
}

/// Representation of a chess move.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciMove {
    /// The source square.
    pub from: UciSquare,

    /// The destination square.
    pub to: UciSquare,

    /// The piece to be promoted to, if any.
    pub promotion: Option<UciPiece>,
}

impl UciMove {
    /// Create a regular, non-promotion move from the `from` square to the `to` square.
    pub fn from_to(from: UciSquare, to: UciSquare) -> UciMove {
        UciMove {
            from,
            to,
            promotion: None,
        }
    }
}

impl Display for UciMove {
    /// Formats the move in the UCI move notation.
    ///
    /// `e2e4` – A move from the square `e2` to the square `e4`.
    /// `a2a1q` – A move from the square `a2` to the square `a1` with the pawn promoting to a Queen..
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let mut r = write!(f, "{}{}", self.from, self.to);

        if let Some(p) = self.promotion {
            if let Some(c) = p.as_char() {
                r = write!(f, "{}", c);
            }
        }

        r
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
/// A representation of the notation in the [FEN notation](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation).
pub struct UciFen(pub String);

impl UciFen {
    /// Returns the FEN string.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for UciFen {
    /// Constructs an UciFen object from a `&str` containing a [FEN](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation)
    /// position. Does not validate the FEN.
    fn from(s: &str) -> Self {
        UciFen(s.to_string())
    }
}


/// A vector containing several `UciMessage`s.
pub type MessageList = Vec<UciMessage>;