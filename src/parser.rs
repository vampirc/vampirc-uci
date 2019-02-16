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
                        _ => unreachable!()
                    }
                }
                UciMessage::Debug(false)
            },
            Rule::isready => UciMessage::IsReady,
            Rule::setoption => {
                let mut name: String = String::default();
                let mut value: String = String::default();


                for sp in pair.into_inner() {
                    match sp.as_rule() {
                        Rule::option_internal => {
                            for spi in sp.into_inner() {
                                match spi.as_rule() {
                                    Rule::option_name => {
                                        name = spi.as_span().as_str().trim().to_string();
                                    },
                                    Rule::option_value => {
                                        value = spi.as_span().as_str().to_string();
                                    }
                                    _ => {}
                                }
                            }
                        },
//                        Rule::value => { value = sp.as_span().as_str().to_string(); },
                        _ => ()
                    }
                }

                let val = if value != String::default()  { Some(value) } else { None };
                UciMessage::SetOption { name, value: val }

            },
            Rule::register => {
                for sp in pair.into_inner() {
                    match sp.as_rule() {
                        Rule::register_later => {
                            return UciMessage::register_later();
                        },
                        Rule::register_nc => {
                            let mut name: &str = "";

                            for spi in sp.into_inner() {
                                match spi.as_rule() {
                                    Rule::register_name => { name = spi.as_span().as_str(); },
                                    Rule::register_code => {
                                        return UciMessage::register_code(name, spi.as_str());
                                    },
                                    _ => ()
                                }
                            }
                        },
                        _ => unreachable!()
                    }
                }

                unreachable!()
            },
            Rule::ucinewgame => {
                UciMessage::UciNewGame
            }
            _ => unreachable!()
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
        let ml = parse("debug off\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Debug(false));
    }

    #[test]
    fn test_debug_wrong_param() {
        let ml = parse("debug abc\r\n");
        assert_eq!(ml.is_err(), true);
    }

    #[test]
    fn test_debug_cutoff() {
        parse("debug    ontario\r\n").expect_err("Should not pass");

    }

    #[test]
    fn test_isready() {
        let ml = parse(" \tisready  \r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::IsReady);
    }

    #[test]
    fn test_set_option_bool() {
        let ml = parse("setoption name Nullmove value true\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value} => {

                assert_eq!(*name, String::from("Nullmove"));
                let val = value.clone();
                assert_eq!(val.is_some(), true);
                assert_eq!(val.unwrap().as_str(), String::from("true"));
                assert_eq!(so.as_bool().unwrap(), true);
            },
            _ => unreachable!()
        }
    }

    // setoption name Selectivity value 3\n
    #[test]
    fn test_set_option_int() {
        let ml = parse("setoption name Selectivity is awesome value 3\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value} => {

                assert_eq!(*name, String::from("Selectivity is awesome"));
                let val = value.clone();
                assert_eq!(val.is_some(), true);
                assert_eq!(val.unwrap().as_str(), String::from("3"));
                assert_eq!(so.as_bool().is_none(), true);
                assert_eq!(so.as_i32().unwrap(), 3);
            },
            _ => unreachable!()
        }
    }

    // setoption name Clear Hash
    #[test]
    fn test_set_option_button() {
        let ml = parse("setoption name Clear Hash\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value} => {

                assert_eq!(*name, String::from("Clear Hash"));
                let val = value.clone();
                assert_eq!(val.is_some(), false);
            },
            _ => unreachable!()
        }
    }

    #[test]
    fn test_set_option_str() {
        let ml = parse("setoption name NalimovPath value c:\\chess\\tb\\4;c:\\chess\\tb\\5\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value} => {

                assert_eq!(*name, String::from("NalimovPath"));
                let val = value.clone();
                assert_eq!(val.is_some(), true);
                assert_eq!(val.unwrap().as_str(), String::from("c:\\chess\\tb\\4;c:\\chess\\tb\\5"));
                assert_eq!(so.as_bool(), None);
            },
            _ => unreachable!()
        }
    }

    #[test]
    fn test_register_later() {
        let ml = parse("REGISTER    lateR\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::register_later());
    }

    #[test]
    fn test_register_name_code() {
        let ml = parse("register name Matija Kejžar code 4359874324\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::register_code("Matija Kejžar", "4359874324"));
    }

    #[test]
    fn test_register_invalid() {
        parse("register name Matija Kejžar\n").expect_err("Parse error expected.");
    }

    #[test]
    fn test_register_invalid2() {
        parse("register code XX-344-00LP name Matija Kejžar\n").expect_err("Parse error expected.");
    }

    #[test]
    fn test_ucinewgame() {
        let ml = parse(" ucinewGAME \r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::UciNewGame);
    }
}