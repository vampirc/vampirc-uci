//use std::error::Result;


use pest::Parser;
use pest::iterators::Pair;
use pest::error::Error;

use crate::uci::{UciMessage, MessageList, UciFen, UciMove, UciSquare, UciPiece};

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
            },
            Rule::stop => {
                UciMessage::Stop
            },
            Rule::ponderhit => {
                UciMessage::PonderHit
            },
            Rule::quit => {
                UciMessage::Quit
            },
            Rule::position => {

                let mut startpos = false;
                let mut fen: Option<UciFen> = None;
                let mut moves: Vec<UciMove> = Default::default();

                for sp in pair.into_inner() {
                    match sp.as_rule() {
                        Rule::startpos => {
                            startpos = true;
                        },
                        Rule::fen => {
                            fen = Some(UciFen::from(sp.as_span().as_str()))
                        },
                        Rule::a_move => {
                            let mut from_sq = UciSquare::default();
                            let mut to_sq = UciSquare::default();
                            let mut promotion: Option<UciPiece> = None;

                            for move_token in sp.into_inner() {
                                match move_token.as_rule() {
                                    Rule::from_sq => { from_sq = parse_square(move_token.into_inner().next().unwrap()); },
                                    Rule::to_sq => { to_sq = parse_square(move_token.into_inner().next().unwrap()); },
                                    Rule::promotion => {
                                        promotion = Some(UciPiece::from(move_token.as_span().as_str()));
                                    }
                                    _ => unreachable!()
                                }
                            }

                            let m = UciMove {
                                from: from_sq,
                                to: to_sq,
                                promotion
                            };

                            moves.push(m);
                        }
                        _ => {}
                    }
                }

                UciMessage::Position {
                    startpos,
                    fen,
                    moves
                }
            },
            _ => unreachable!()
        }
    })
        .for_each(|msg| { ml.push(msg) })
    ;


    Ok(ml)
}

fn parse_square(sq_pair: Pair<Rule>) -> UciSquare {
    let mut file: char = '\0';
    let mut rank: u8 = 0;

    match sq_pair.as_rule() {
        Rule::square => {
            for sp in sq_pair.into_inner() {
                match sp.as_rule() {
                    Rule::file => { file = sp.as_span().as_str().chars().into_iter().next().unwrap(); },
                    Rule::rank => { rank = str::parse(sp.as_span().as_str()).unwrap();},
                    _ => unreachable!()
                }
            }
        },
        _ => unreachable!()
    }

    UciSquare::from(file, rank)

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
    fn test_debugon() {
        parse("debugon\r\n").expect_err("Should not parse 'debugon'");
    }

    #[test]
    fn test_debug_wrong_param() {
        let ml = parse("debug abc\r\n");
        assert_eq!(ml.is_err(), true);
    }

    #[test]
    fn test_debug_cutoff() {
        parse("debug    ontario\r\n").expect_err("Should not parse");

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

    #[test]
    fn test_stop() {
        let ml = parse("stop\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Stop);
    }

    #[test]
    fn test_stop_really_stop() {
        parse("stopper\r\n").expect_err("Parse error expected for 'stopper'.");
    }

    #[test]
    fn test_ponderhit() {
        let ml = parse("PonderHit   \r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::PonderHit);
    }

    #[test]
    fn test_quit() {
        let ml = parse("QUIT\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Quit);
    }

    #[test]
    fn test_position_startpos() {
        let ml = parse("position startpos moves e2e4 e7e5\r\n").unwrap();
        assert_eq!(ml.len(), 1);

        let m1 = UciMove {
            from: UciSquare {
                file: 'e',
                rank: 2
            },
            to: UciSquare {
                file: 'e',
                rank: 4
            },
            promotion: None
        };

        let m2 = UciMove {
            from: UciSquare {
                file: 'e',
                rank: 7
            },
            to: UciSquare {
                file: 'e',
                rank: 5
            },
            promotion: None
        };

        let pos = UciMessage::Position {
            startpos: true,
            fen: None,
            moves: vec![m1, m2]
        };

        assert_eq!(ml[0], pos);
    }

    #[test]
    fn test_position_startpos_as_fen() {
        let ml = parse("position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves d2d4\r\n").unwrap();
        assert_eq!(ml.len(), 1);

        let m1 = UciMove {
            from: UciSquare {
                file: 'd',
                rank: 2
            },
            to: UciSquare {
                file: 'd',
                rank: 4
            },
            promotion: None
        };

        let pos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(String::from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"))),
            moves: vec![m1]
        };

        assert_eq!(ml[0], pos);
    }

    // 2k5/6PR/8/8/2b4P/8/6K1/8 w - -
    #[test]
    fn test_position_endgame() {
        let ml = parse("position fen 2k5/6PR/8/8/2b4P/8/6K1/8 w - - 0 53 moves g7g8q c4g8\r\n").unwrap();
        assert_eq!(ml.len(), 1);

        let m1 = UciMove {
            from: UciSquare {
                file: 'g',
                rank: 7
            },
            to: UciSquare {
                file: 'g',
                rank: 8
            },
            promotion: Some(UciPiece::Queen)
        };

        let m2 = UciMove {
            from: UciSquare {
                file: 'c',
                rank: 4
            },
            to: UciSquare {
                file: 'g',
                rank: 8
            },
            promotion: None
        };

        let pos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(String::from("2k5/6PR/8/8/2b4P/8/6K1/8 w - - 0 53"))),
            moves: vec![m1, m2]
        };

        assert_eq!(ml[0], pos);
    }

    #[test]
    fn test_position_incorrect_fen() {
        parse("position fen 2k50/6PR/8/8/2b4P/8/6K1/8 w - - 0 53 moves g7g8q c4g8\r\n").expect_err("Parse should fail.");
    }
}