//! The `uci` module contains the definitions that represent UCI protocol messages. Usually, these messages will be
//! obtained by calling the `parse` method of the `parser` module, but you can always construct them in code and then
//! print them to the standard output to communicate with the engine or GUI.


use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use crate::parser::parse;
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
    /// Construct `register later` a [UciMessage::Register](enum.UciMessage.html#variant.Register)  message.
    pub fn register_later() -> UciMessage {
        UciMessage::Register {
            later: true,
            name: None,
            code: None,
        }
    }

    /// Construct a `register <code> <name>` [UciMessage::Register](enum.UciMessage.html#variant.Register) message.
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

    /// Construct a `go infinite` [UciMessage::Register](enum.UciMessage.html#variant.Go) message.
    pub fn go_infinite() -> UciMessage {
        UciMessage::Go {
            search_control: None,
            time_control: Some(UciTimeControl::Infinite)
        }
    }

    /// Construct a `go movetime <milliseconds>` [UciMessage::Register](enum.UciMessage.html#variant.Go) message, with
    /// `milliseconds` as the argument.
    pub fn go_movetime(milliseconds: u64) -> UciMessage {
        UciMessage::Go {
            search_control: None,
            time_control: Some(UciTimeControl::from(milliseconds))
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
                        UciTimeControl::MoveTime { milliseconds } => {
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

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum UciTimeControl {
    Ponder,
    Infinite,
    TimeLeft {
        white_time: Option<u64>,
        black_time: Option<u64>,
        white_increment: Option<u64>,
        black_increment: Option<u64>,
        moves_to_go: Option<u8>,
    },
    MoveTime {
        milliseconds: u64
    },
}

impl UciTimeControl {
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

impl From<u64> for UciTimeControl {
    fn from(milliseconds: u64) -> Self {
        UciTimeControl::MoveTime {
            milliseconds
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciSearchControl {
    pub search_moves: Vec<UciMove>,
    pub mate: Option<u8>,
    pub depth: Option<u8>,
    pub nodes: Option<u64>,
}

impl UciSearchControl {
    pub fn depth(depth: u8) -> UciSearchControl {
        UciSearchControl {
            search_moves: vec![],
            mate: None,
            depth: Some(depth),
            nodes: None,
        }
    }

    pub fn mate(mate: u8) -> UciSearchControl {
        UciSearchControl {
            search_moves: vec![],
            mate: Some(mate),
            depth: None,
            nodes: None,
        }
    }

    pub fn nodes(nodes: u64) -> UciSearchControl {
        UciSearchControl {
            search_moves: vec![],
            mate: None,
            depth: None,
            nodes: Some(nodes),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.search_moves.is_empty() && self.mate.is_none() && self.depth.is_none() && self.nodes.is_none()
    }
}

impl Default for UciSearchControl {
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

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciSquare {
    pub file: char,
    pub rank: u8,
}

impl UciSquare {
    pub fn from(file: char, rank: u8) -> UciSquare {
        UciSquare {
            file,
            rank,
        }
    }
}

impl Display for UciSquare {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}{}", self.file, self.rank)
    }
}

impl Default for UciSquare {
    fn default() -> Self {
        UciSquare {
            file: '\0',
            rank: 0,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciMove {
    pub from: UciSquare,
    pub to: UciSquare,
    pub promotion: Option<UciPiece>,
}

impl UciMove {
    pub fn from_to(from: UciSquare, to: UciSquare) -> UciMove {
        UciMove {
            from,
            to,
            promotion: None,
        }
    }
}

impl Display for UciMove {
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
pub struct UciFen(pub String);

impl UciFen {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for UciFen {
    fn from(s: &str) -> Self {
        UciFen(s.to_string())
    }
}

pub type MessageList = Vec<UciMessage>;