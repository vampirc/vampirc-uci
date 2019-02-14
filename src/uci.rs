//pub type Arguments = &Vec<&Argument>;
use std::fmt::{Display, Result as FmtResult, Formatter, Debug};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CommunicationDirection {
    GuiToEngine,
    EngineToGui,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum UciMessage {
    Uci,
    Debug(bool),
    IsReady,
    Register {
        later: bool,
        name: Option<String>,
        code: Option<String>,
    },
    Position {
        startpos: bool,
        fen: Option<UciFen>,
        moves: Vec<UciMove>,
    },
    SetOption {
        name: String,
        value: Option<String>
    },
    UciNewGame,
    Stop,
    PonderHit,
    Quit,
}

impl UciMessage {
    fn serialize(&self) -> String {
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
            },
            UciMessage::SetOption {name, value} => {
                let mut s: String = String::from(format!("setoption name {}", name));

                if let Some(val) = value {
                    s += format!(" value {}", *val).as_str();
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
            UciMessage::Quit => CommunicationDirection::GuiToEngine,
//            _ => CommunicationDirection::EngineToGui
        }
    }
}

impl Display for UciMessage {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.serialize())
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
pub enum ChessPiece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl ChessPiece {
    pub fn as_char(self) -> Option<char> {
        match self {
            ChessPiece::Pawn => None,
            ChessPiece::Knight => Some('n'),
            ChessPiece::Bishop => Some('b'),
            ChessPiece::Rook => Some('r'),
            ChessPiece::Queen => Some('q'),
            ChessPiece::King => Some('k')
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciSquare {
    file: char,
    rank: u8,
}

impl Display for UciSquare {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}{}", self.file, self.rank)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct UciMove {
    from: UciSquare,
    to: UciSquare,
    promotion: Option<ChessPiece>,
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
pub struct UciFen(String);

impl UciFen {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

pub type MessageList = Vec<UciMessage>;