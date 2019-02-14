//use std::error::Result;


use pest::Parser;
use pest::iterators::Pair;
use pest::error::Error;

use crate::uci::{UciMessage, MessageList};

#[derive(Parser)]
#[grammar = "../res/uci.pest"]
struct UciParser;

pub fn parse(s: &str) -> Result<MessageList, Error<Rule>> {
    let mut ml = MessageList::default();

    let pairs = UciParser::parse(Rule::commands, s)?;

    pairs
        .map(|pair: Pair<_>| {
        match pair.as_rule() {
            Rule::uci => UciMessage::Uci,
            Rule::debug => {
                for sp in pair.into_inner() {
                    match sp.as_rule() {
                        Rule::switch => {
                            return UciMessage::Debug(sp.as_span().as_str().eq_ignore_ascii_case("on"));
                        },
                        _ => unimplemented!("Debug toggle")
                    }
                }
                UciMessage::Debug(false)
            },
            Rule::isready => UciMessage::IsReady,
            _ => panic!("Unsupported")
        }
    })
        .for_each(|msg| { ml.push(msg) })
    ;


    Ok(ml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uci() {
        let ml = parse("uci\r\nuci\r\n").unwrap();
        assert_eq!(ml.len(), 2);
        for mb in ml {
            //let mbb = &(*mb);
            assert_eq!(mb, UciMessage::Uci);
        }
    }

    #[test]
    fn test_debug_on() {
        let ml = parse("debug    on\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Debug(true));
    }

    #[test]
    fn test_debug_off() {
        let ml = parse("debug off").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Debug(false));
    }

    #[test]
    fn test_debug_wrong_param() {
        let ml = parse("debug abc\r\n");
        assert_eq!(ml.is_err(), true);
    }

    #[test]
    fn test_isready() {
        let ml = parse(" \tisready  \r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::IsReady);
    }
}