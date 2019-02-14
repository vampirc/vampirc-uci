//use std::error::Result;


use pest::Parser;
use pest::iterators::Pair;
use pest::error::Error;

use crate::uci::MessageList;
use crate::engine_bound::Command;

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
            Rule::uci => Command::Uci,
            _ => panic!("Unsupported")
        }
    })
        .map(|msg| { Box::from(msg) })
        .for_each(|box_msg| { ml.push(box_msg) })

    ;


    Ok(ml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uci() {

        let ml = parse("uci\r\nuci\r\n");
//        for mb in ml.unwrap() {
//            //let mbb = &(*mb);
//            match *mb {
//                Command::Uci => {},
//                _ => panic!("Not UCI")
//            }
//        }
    }
}