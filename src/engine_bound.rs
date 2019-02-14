use std::fmt::{Display, Result as FmtResult, Formatter};

use crate::uci::{UciMessage, CommunicationDirection, UciMove, UciFen};

pub trait EngineBoundMessage<'a> : UciMessage<'a> {

    #[inline]
    fn direction(&self) -> CommunicationDirection {
        CommunicationDirection::GuiToEngine
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Command {
    Uci,
    Debug(bool),
    IsReady,
    Register {
        later: bool,
        name: Option<String>,
        code: Option<String>
    },
    Position {
        startpos: bool,
        fen: Option<UciFen>,
        moves: Vec<UciMove>
    },
    UciNewGame,
    Stop,
    PonderHit,
    Quit
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.serialize())
    }
}

impl <'a> UciMessage<'a> for Command {
    fn name(&'a self) -> &'a str {
        match *self {
            Command::Uci => "uci",
            Command::Debug(..) => "debug",
            Command::IsReady => "isready",
            Command::Register {..} => "register",
            Command::UciNewGame => "ucinewgame",
            Command::Position {..} => "position",
            Command::Stop => "stop",
            Command::PonderHit => "ponderhit",
            Command::Quit => "quit"
        }
    }

    fn serialize(&self) -> String {
        match self {
            Command::Debug(on) => if *on {String::from("debug on")} else {String::from("debug off")},
            Command::Register {later, name, code} => {
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
            },
            Command::Position { startpos, fen, moves} => {
                let mut s = String::from("position ");
                if *startpos {
                    s += String::from("startpos").as_str();
                } else if let Some(uci_fen) = fen {
                    s +=  format!("fen {}", uci_fen.as_str()).as_str();
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
            _ => self.name().to_string()
        }
    }

    // TODO CHECK THIS
    #[inline]
    fn direction(&self) -> CommunicationDirection {
        EngineBoundMessage::direction(self as &EngineBoundMessage<'a>)
    }
}

impl <'a> EngineBoundMessage<'a> for Command {

}


#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SetOption<T> where T: Display {
    name: String,
    value: Option<T>
}

impl <T> Display for SetOption<T> where T: Display {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.serialize())
    }
}

impl <'a, T> UciMessage<'a> for SetOption<T> where T: Display {
    #[inline]
    fn name(&'a self) -> &'a str {
        self.name.as_str()
    }

    fn serialize(&self) -> String {
        let mut s: String = String::from(format!("setoption name {}", self.name()));

        if let Some(value) = &self.value {
            s += format!(" value {}", *value).as_str();
        }

        s
    }

    #[inline]
    fn direction(&self) -> CommunicationDirection {
        EngineBoundMessage::direction(self as &EngineBoundMessage<'a>)
    }
}

impl <'a, T> EngineBoundMessage<'a> for SetOption<T> where T: Display {

}