//! The `parser` module contains the `parse` method that performs the parsing of UCI messages into their respective
//! `UciMessage` variants.
//!
//! Behind the scenes, it uses the [PEST parser](https://github.com/pest-parser/pest). The corresponding PEG grammar is
//! available [here](https://github.com/vampirc/vampirc-uci/blob/master/res/uci.pest).

use std::str::FromStr;

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;

use crate::uci::{MessageList, UciFen, UciMessage, UciMove, UciPiece, UciSearchControl, UciSquare, UciTimeControl};
use crate::uci::UciMessage::Uci;

#[derive(Parser)]
#[grammar = "../res/uci.pest"]
struct UciParser;

/// Parses the specified `&str s` into a list of `UciMessage`s. Please note that this method will return an `Error` if
/// any of the input violates the grammar rules.
///
/// The UCI messages are separated by a newline character, as per the UCI protocol specification.
///
/// This method differs from the `parse(..)` method in the fact that any unrecognized tokens/messages will result in
/// an error being returned.
///
/// # Examples
///
/// ```
/// use vampirc_uci::UciMessage;
/// use vampirc_uci::parse_strict;
///
/// let messages = parse_strict("position startpos\ngo ponder searchmoves e2e4 d2d4\n").unwrap();
/// assert_eq!(messages.len(), 2);
///
/// ```
pub fn parse_strict(s: &str) -> Result<MessageList, Error<Rule>> {
    do_parse_uci(s, Rule::commands)
}

/// Parses the specified `&str s` into a list of `UciMessage`s. Please note that this method will ignore any
/// unrecognized messages, which is in-line with the recommendations of the UCI protocol specification.
///
/// The UCI messages are separated by a newline character, as per the UCI protocol specification.
///
/// This method differs from the `parse_strict(..)` method in the fact that any unrecognized tokens/messages will
/// simply be ignored.
///
/// # Examples
///
/// ```
/// use vampirc_uci::UciMessage;
/// use vampirc_uci::parse;
///
/// let messages = parse("position startpos\n  unknown message that will be ignored  \ngo infinite\n");
/// assert_eq!(messages.len(), 2);
///
/// ```
pub fn parse(s: &str) -> MessageList {
    do_parse_uci(s, Rule::commands_ignore_unknown).unwrap()
}

fn do_parse_uci(s: &str, top_rule: Rule) -> Result<MessageList, Error<Rule>> {
    let mut ml = MessageList::default();

    let pairs = UciParser::parse(top_rule, s)?;

    pairs
        .map(|pair: Pair<_>| {
            match pair.as_rule() {
                Rule::uci => UciMessage::Uci,
                Rule::debug => {
                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::switch => {
                                return UciMessage::Debug(sp.as_span().as_str().eq_ignore_ascii_case("on"));
                            }
                            _ => unreachable!()
                        }
                    }
                    UciMessage::Debug(false)
                }
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
                                        }
                                        Rule::option_value => {
                                            value = spi.as_span().as_str().to_string();
                                        }
                                        _ => {}
                                    }
                                }
                            }
//                        Rule::value => { value = sp.as_span().as_str().to_string(); },
                            _ => ()
                        }
                    }

                    let val = if value != String::default() { Some(value) } else { None };
                    UciMessage::SetOption { name, value: val }
                }
                Rule::register => {
                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::register_later => {
                                return UciMessage::register_later();
                            }
                            Rule::register_nc => {
                                let mut name: &str = "";

                                for spi in sp.into_inner() {
                                    match spi.as_rule() {
                                        Rule::register_name => { name = spi.as_span().as_str(); }
                                        Rule::register_code => {
                                            return UciMessage::register_code(name, spi.as_str());
                                        }
                                        _ => ()
                                    }
                                }
                            }
                            _ => unreachable!()
                        }
                    }

                    unreachable!()
                }
                Rule::ucinewgame => {
                    UciMessage::UciNewGame
                }
                Rule::stop => {
                    UciMessage::Stop
                }
                Rule::ponderhit => {
                    UciMessage::PonderHit
                }
                Rule::quit => {
                    UciMessage::Quit
                }
                Rule::position => {
                    let mut startpos = false;
                    let mut fen: Option<UciFen> = None;
                    let mut moves: Vec<UciMove> = Default::default();

                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::startpos => {
                                startpos = true;
                            }
                            Rule::fen => {
                                fen = Some(UciFen::from(sp.as_span().as_str()))
                            }
                            Rule::a_move => {
                                moves.push(parse_a_move(sp));
                            }
                            _ => {}
                        }
                    }

                    UciMessage::Position {
                        startpos,
                        fen,
                        moves,
                    }
                }
                Rule::go => {
                    let mut time_control: Option<UciTimeControl> = None;
                    let mut tl = false;
                    let mut wtime: Option<u64> = None;
                    let mut btime: Option<u64> = None;
                    let mut winc: Option<u64> = None;
                    let mut binc: Option<u64> = None;
                    let mut moves_to_go: Option<u8> = None;

                    let mut search: UciSearchControl = UciSearchControl::default();

                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::go_time => {
                                for spi in sp.into_inner() {
                                    println!("SPI RULE");
                                    match spi.as_rule() {
                                        Rule::go_ponder => { time_control = Some(UciTimeControl::Ponder); }
                                        Rule::go_infinite => { time_control = Some(UciTimeControl::Infinite); }
                                        Rule::go_movetime => { time_control = Some(UciTimeControl::MoveTime(parse_milliseconds(spi))); }
                                        Rule::go_timeleft => {
                                            if !tl {
                                                tl = true;
                                            }

                                            for sspi in spi.into_inner() {
                                                match sspi.as_rule() {
                                                    Rule::wtime => { wtime = Some(parse_milliseconds(sspi)); }
                                                    Rule::btime => { btime = Some(parse_milliseconds(sspi)); }
                                                    Rule::winc => { winc = Some(parse_milliseconds(sspi)); }
                                                    Rule::binc => { binc = Some(parse_milliseconds(sspi)); }
                                                    Rule::movestogo => { moves_to_go = Some(parse_u8(sspi, Rule::digits3)); }
                                                    _ => {}
                                                };
                                            }
                                        }

                                        _ => {}
                                    }
                                }
                            }
                            Rule::go_search => {
                                for spi in sp.into_inner() {
                                    match spi.as_rule() {
                                        Rule::depth => {
                                            search.depth = Some(parse_u8(spi, Rule::digits3));
                                        },
                                        Rule::mate => {
                                            search.mate = Some(parse_u8(spi, Rule::digits3))
                                        }
                                        Rule::nodes => {
                                            search.nodes = Some(parse_u64(spi, Rule::digits12))
                                        },
                                        Rule::searchmoves => {
                                            for mt in spi.into_inner() {
                                                search.search_moves.push(parse_a_move(mt));
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if tl {
                        time_control = Some(UciTimeControl::TimeLeft {
                            white_time: wtime,
                            black_time: btime,
                            white_increment: winc,
                            black_increment: binc,
                            moves_to_go,
                        });
                    }

                    let search_control: Option<UciSearchControl>;
                    if search.is_empty() {
                        search_control = None
                    } else {
                        search_control = Some(search);
                    }

                    UciMessage::Go {
                        time_control,
                        search_control,
                    }
                }
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
                    Rule::file => { file = sp.as_span().as_str().chars().into_iter().next().unwrap(); }
                    Rule::rank => { rank = str::parse(sp.as_span().as_str()).unwrap(); }
                    _ => unreachable!()
                }
            }
        }
        _ => unreachable!()
    }

    UciSquare::from(file, rank)
}

fn parse_milliseconds(pair: Pair<Rule>) -> u64 {
    for sp in pair.into_inner() {
        match sp.as_rule() {
            Rule::milliseconds => {
                return str::parse::<u64>(sp.as_span().as_str()).unwrap();
            }
            _ => {}
        }
    }

    0
}

fn parse_u8(pair: Pair<Rule>, rule: Rule) -> u8 {
    for sp in pair.into_inner() {
        if sp.as_rule() == rule {
            return str::parse::<u8>(sp.as_span().as_str()).unwrap();
        }
    }

    0
}

fn parse_u64(pair: Pair<Rule>, rule: Rule) -> u64 {
    for sp in pair.into_inner() {
        if sp.as_rule() == rule {
            return str::parse::<u64>(sp.as_span().as_str()).unwrap();
        }
    }

    0
}

fn parse_a_move(sp: Pair<Rule>) -> UciMove {
    let mut from_sq = UciSquare::default();
    let mut to_sq = UciSquare::default();
    let mut promotion: Option<UciPiece> = None;

    for move_token in sp.into_inner() {
        match move_token.as_rule() {
            Rule::from_sq => { from_sq = parse_square(move_token.into_inner().next().unwrap()); }
            Rule::to_sq => { to_sq = parse_square(move_token.into_inner().next().unwrap()); }
            Rule::promotion => {
                promotion = Some(UciPiece::from_str(move_token.as_span().as_str()).unwrap());
            }
            _ => unreachable!()
        }
    }

    UciMove {
        from: from_sq,
        to: to_sq,
        promotion,
    }
}

#[cfg(test)]
mod tests {
    use crate::uci::UciMessage::Uci;
    use crate::uci::UciMessage::UciNewGame;
    use crate::uci::UciTimeControl::TimeLeft;

    use super::*;

    #[test]
    fn test_uci() {
        let ml = parse_strict("uci\r\nuci\r\n").unwrap();
        assert_eq!(ml.len(), 2);
        for mb in ml {
            //let mbb = &(*mb);
            assert_eq!(mb, UciMessage::Uci);
        }
    }

    #[test]
    fn test_debug_on() {
        let ml = parse_strict("debug    on\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Debug(true));
    }

    #[test]
    fn test_debug_off() {
        let ml = parse_strict("debug off\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Debug(false));
    }

    #[test]
    fn test_debugon() {
        parse_strict("debugon\r\n").expect_err("Should not parse 'debugon'");
    }

    #[test]
    fn test_debug_wrong_param() {
        let ml = parse_strict("debug abc\r\n");
        assert_eq!(ml.is_err(), true);
    }

    #[test]
    fn test_debug_cutoff() {
        parse_strict("debug    ontario\r\n").expect_err("Should not parse");
    }

    #[test]
    fn test_isready() {
        let ml = parse_strict(" \tisready  \r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::IsReady);
    }

    #[test]
    fn test_set_option_bool() {
        let ml = parse_strict("setoption name Nullmove value true\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value } => {
                assert_eq!(*name, String::from("Nullmove"));
                let val = value.clone();
                assert_eq!(val.is_some(), true);
                assert_eq!(val.unwrap().as_str(), String::from("true"));
                assert_eq!(so.as_bool().unwrap(), true);
            }
            _ => unreachable!()
        }
    }

    // setoption name Selectivity value 3\n
    #[test]
    fn test_set_option_int() {
        let ml = parse_strict("setoption name Selectivity is awesome value 3\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value } => {
                assert_eq!(*name, String::from("Selectivity is awesome"));
                let val = value.clone();
                assert_eq!(val.is_some(), true);
                assert_eq!(val.unwrap().as_str(), String::from("3"));
                assert_eq!(so.as_bool().is_none(), true);
                assert_eq!(so.as_i32().unwrap(), 3);
            }
            _ => unreachable!()
        }
    }

    // setoption name Clear Hash
    #[test]
    fn test_set_option_button() {
        let ml = parse_strict("setoption name Clear Hash\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value } => {
                assert_eq!(*name, String::from("Clear Hash"));
                let val = value.clone();
                assert_eq!(val.is_some(), false);
            }
            _ => unreachable!()
        }
    }

    #[test]
    fn test_set_option_str() {
        let ml = parse_strict("setoption name NalimovPath value c:\\chess\\tb\\4;c:\\chess\\tb\\5\n").unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value } => {
                assert_eq!(*name, String::from("NalimovPath"));
                let val = value.clone();
                assert_eq!(val.is_some(), true);
                assert_eq!(val.unwrap().as_str(), String::from("c:\\chess\\tb\\4;c:\\chess\\tb\\5"));
                assert_eq!(so.as_bool(), None);
            }
            _ => unreachable!()
        }
    }

    #[test]
    fn test_register_later() {
        let ml = parse_strict("REGISTER    lateR\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::register_later());
    }

    #[test]
    fn test_register_name_code() {
        let ml = parse_strict("register name Matija Kejžar code 4359874324\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::register_code("Matija Kejžar", "4359874324"));
    }

    #[test]
    fn test_register_invalid() {
        parse_strict("register name Matija Kejžar\n").expect_err("Parse error expected.");
    }

    #[test]
    fn test_register_invalid2() {
        parse_strict("register code XX-344-00LP name Matija Kejžar\n").expect_err("Parse error expected.");
    }

    #[test]
    fn test_ucinewgame() {
        let ml = parse_strict(" ucinewGAME \r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::UciNewGame);
    }

    #[test]
    fn test_stop() {
        let ml = parse_strict("stop\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Stop);
    }

    #[test]
    fn test_stop_really_stop() {
        parse_strict("stopper\r\n").expect_err("Parse error expected for 'stopper'.");
    }

    #[test]
    fn test_ponderhit() {
        let ml = parse_strict("PonderHit   \r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::PonderHit);
    }

    #[test]
    fn test_quit() {
        let ml = parse_strict("QUIT\r\n").unwrap();
        assert_eq!(ml.len(), 1);
        assert_eq!(ml[0], UciMessage::Quit);
    }

    #[test]
    fn test_position_startpos() {
        let ml = parse_strict("position startpos moves e2e4 e7e5\r\n").unwrap();
        assert_eq!(ml.len(), 1);

        let m1 = UciMove {
            from: UciSquare {
                file: 'e',
                rank: 2,
            },
            to: UciSquare {
                file: 'e',
                rank: 4,
            },
            promotion: None,
        };

        let m2 = UciMove {
            from: UciSquare {
                file: 'e',
                rank: 7,
            },
            to: UciSquare {
                file: 'e',
                rank: 5,
            },
            promotion: None,
        };

        let pos = UciMessage::Position {
            startpos: true,
            fen: None,
            moves: vec![m1, m2],
        };

        assert_eq!(ml[0], pos);
    }

    #[test]
    fn test_position_startpos_as_fen() {
        let ml = parse_strict("position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves d2d4\r\n").unwrap();
        assert_eq!(ml.len(), 1);

        let m1 = UciMove {
            from: UciSquare {
                file: 'd',
                rank: 2,
            },
            to: UciSquare {
                file: 'd',
                rank: 4,
            },
            promotion: None,
        };

        let pos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(String::from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"))),
            moves: vec![m1],
        };

        assert_eq!(ml[0], pos);
    }

    // 2k5/6PR/8/8/2b4P/8/6K1/8 w - -
    #[test]
    fn test_position_endgame() {
        let ml = parse_strict("position fen 2k5/6PR/8/8/2b4P/8/6K1/8 w - - 0 53 moves g7g8q c4g8\r\n").unwrap();
        assert_eq!(ml.len(), 1);

        let m1 = UciMove {
            from: UciSquare {
                file: 'g',
                rank: 7,
            },
            to: UciSquare {
                file: 'g',
                rank: 8,
            },
            promotion: Some(UciPiece::Queen),
        };

        let m2 = UciMove {
            from: UciSquare {
                file: 'c',
                rank: 4,
            },
            to: UciSquare {
                file: 'g',
                rank: 8,
            },
            promotion: None,
        };

        let pos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(String::from("2k5/6PR/8/8/2b4P/8/6K1/8 w - - 0 53"))),
            moves: vec![m1, m2],
        };

        assert_eq!(ml[0], pos);
    }

    #[test]
    fn test_position_incorrect_fen() {
        parse_strict("position fen 2k50/6PR/8/8/2b4P/8/6K1/8 w - - 0 53 moves g7g8q c4g8\r\n").expect_err("Parse should fail.");
    }

    #[test]
    fn test_position_startpos_no_moves() {
        let ml = parse_strict("position   startpos\r\n").unwrap();
        assert_eq!(ml.len(), 1);


        let pos = UciMessage::Position {
            startpos: true,
            fen: None,
            moves: vec![],
        };

        assert_eq!(ml[0], pos);
    }

    #[test]
    fn test_position_fen_no_moves() {
        let ml = parse_strict("position    fen 2k5/6PR/8/8/2b4P/8/6K1/8 w   - - 0 53\r\n").unwrap();
        assert_eq!(ml.len(), 1);

        let pos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(String::from("2k5/6PR/8/8/2b4P/8/6K1/8 w   - - 0 53"))),
            moves: vec![],
        };

        assert_eq!(ml[0], pos);
    }

    #[test]
    fn test_go_ponder() {
        let ml = parse_strict("go ponder\n").unwrap();
        assert_eq!(ml.len(), 1);

        assert_eq!(ml[0], UciMessage::go_ponder());
    }

    #[test]
    fn test_go_infinite() {
        let ml = parse_strict("go infinite\n").unwrap();
        assert_eq!(ml.len(), 1);

        assert_eq!(ml[0], UciMessage::go_infinite());
    }

    #[test]
    fn test_go_movetime() {
        let ml = parse_strict("go movetime  55055\n").unwrap();
        assert_eq!(ml.len(), 1);

        assert_eq!(ml[0], UciMessage::go_movetime(55055));
    }

    #[test]
    fn test_go_timeleft() {
        let ml = parse_strict("go wtime 903000 btime 770908 winc 15000 movestogo 17 binc 10000\n").unwrap();
        assert_eq!(ml.len(), 1);

        let tl = UciTimeControl::TimeLeft {
            white_time: Some(903000),
            black_time: Some(770908),
            white_increment: Some(15000),
            black_increment: Some(10000),
            moves_to_go: Some(17),
        };

        assert_eq!(ml[0], UciMessage::Go {
            search_control: None,
            time_control: Some(tl),
        });
    }

    #[test]
    fn test_search_control_depth() {
        let ml = parse_strict("go ponder depth 6\n").unwrap();
        assert_eq!(ml.len(), 1);

        let result = UciMessage::Go {
            time_control: Some(UciTimeControl::Ponder),
            search_control: Some(UciSearchControl::depth(6)),
        };

        assert_eq!(ml[0], result);
    }

    #[test]
    fn test_search_control_mate() {
        let ml = parse_strict("go mate 12\n").unwrap();
        assert_eq!(ml.len(), 1);

        let result = UciMessage::Go {
            time_control: None,
            search_control: Some(UciSearchControl::mate(12)),
        };

        assert_eq!(ml[0], result);
    }

    #[test]
    fn test_nodes_searchmoves() {
        let ml = parse_strict("go nodes 79093455456 searchmoves e2e4 d2d4 g2g1n\n").unwrap();
        assert_eq!(ml.len(), 1);

        let sc = UciSearchControl {
            depth: None,
            nodes: Some(79093455456),
            mate: None,
            search_moves: vec![
                UciMove::from_to(UciSquare::from('e', 2), UciSquare::from('e', 4)),
                UciMove::from_to(UciSquare::from('d', 2), UciSquare::from('d', 4)),
                UciMove {
                    from: UciSquare::from('g', 2),
                    to: UciSquare::from('g', 1),
                    promotion: Some(UciPiece::Knight),
                }
            ],
        };

        let result = UciMessage::Go {
            time_control: None,
            search_control: Some(sc),
        };

        assert_eq!(ml[0], result);
    }

    #[test]
    fn test_go_full_example() {
        let ml = parse_strict("go movetime 10000 searchmoves a1h8 depth 6 nodes 55000000\n").unwrap();
        assert_eq!(ml.len(), 1);

        let tc = UciTimeControl::MoveTime(10000);

        let sc = UciSearchControl {
            depth: Some(6),
            nodes: Some(55000000),
            mate: None,
            search_moves: vec![
                UciMove::from_to(UciSquare::from('a', 1), UciSquare::from('h', 8)),
            ],
        };

        let result = UciMessage::Go {
            time_control: Some(tc),
            search_control: Some(sc),
        };

        assert_eq!(ml[0], result);
    }

    #[test]
    fn test_two_command_doc_example() {
        let ml = parse_strict("position startpos\ngo ponder searchmoves e2e4 d2d4\n").unwrap();
        assert_eq!(ml.len(), 2);
    }

    #[test]
    fn test_lax_mode() {
        let ml = parse("position startpos\nunknown command\ngo ponder searchmoves e2e4 d2d4\n");
        assert_eq!(ml.len(), 2);

        match ml[0] {
            UciMessage::Position { .. } => {},
            _ => panic!("Expected a `position` message here")
        };

        match ml[1] {
            UciMessage::Go { .. } => {},
            _ => panic!("Expected a `go` message here")
        };
    }

    #[test]
    #[should_panic]
    fn test_strict_mode() {
        parse_strict("position startpos\nunknown command\ngo ponder searchmoves e2e4 d2d4\n").unwrap();
    }
}