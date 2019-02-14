//pub type Arguments = &Vec<&Argument>;
use std::fmt::{Display, Result as FmtResult, Formatter};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CommunicationDirection {
    GuiToEngine,
    EngineToGui
}

pub trait UciMessage<'a> : Display {

    fn name(&'a self) -> &'a str;
//    fn arguments(&self) -> Option<&Vec<&Argument>>;
//    fn sub_commands(&self) -> Option<&Vec<&dyn UciMessage>>;
    fn serialize(&self) -> String;

    fn direction(&self) -> CommunicationDirection;
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
    String
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


#[derive(Clone, Eq, PartialEq, Debug)]
pub struct UciOption<T> where T: Display {
    name: String,
    option_type: OptionType,
    min: Option<T>,
    max: Option<T>,
    default: T,
    var: Vec<T>
}

impl <T> UciOption<T> where T: Display {

}

impl <T> Display for UciOption<T> where T: Display {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.serialize())
    }
}

impl <'a, T> UciMessage<'a> for UciOption<T> where T: Display {
    fn name(&'a self) -> &'a str {
        self.name.as_str()
    }

    fn serialize(&self) -> String {
        let mut s: String = String::from("option name ");
        s += self.name.as_str();
        s += " type ";
        s += format!(" type {} ", self.option_type).as_str();
        s += format!(" default {} ", self.default).as_str();

        if let Some(min) = &self.min {
            s += format!(" min {}", *min).as_str();
        }

        if let Some(max) = &self.max {
            s += format!(" max {}", *max).as_str();
        }

        if self.var.len() > 0 {
            for (i, var) in (&self.var).into_iter().enumerate() {
                s += format!(" var {}", *var).as_str();
                if i < self.var.len() - 1 {
                    s += " ";
                }
            }
        }

        s
    }

    fn direction(&self) -> CommunicationDirection {
        CommunicationDirection::EngineToGui
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ChessPiece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King
}

impl ChessPiece {
    pub fn as_char(self) -> Option<char> {
        match self {
            ChessPiece::Pawn=> None,
            ChessPiece::Knight => Some('n'),
            ChessPiece::Bishop => Some('b'),
            ChessPiece::Rook => Some('r'),
            ChessPiece::Queen => Some('q'),
            ChessPiece::King => Some('k')
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct UciSquare {
    file: char,
    rank: u8
}

impl Display for UciSquare {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {

        write!(f, "{}{}", self.file, self.rank)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct UciMove {
    from: UciSquare,
    to: UciSquare,
    promotion: Option<ChessPiece>
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

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct UciFen(String);

impl UciFen {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}