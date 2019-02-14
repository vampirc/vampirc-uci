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
            pair.as_rule()
        }).map(|rule| {
        match rule {
            Rule::uci => UciMessage::Uci,
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
}