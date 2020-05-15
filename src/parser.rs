//! The `parser` module contains the `parse` method that performs the parsing of UCI messages into their respective
//! `UciMessage` variants.
//!
//! Behind the scenes, it uses the [PEST parser](https://github.com/pest-parser/pest). The corresponding PEG grammar is
//! available [here](https://github.com/vampirc/vampirc-uci/blob/master/res/uci.pest).

#[cfg(feature = "chess")]
use std::fmt::Error as FmtError;
#[cfg(not(feature = "chess"))]
use std::str::FromStr;
use std::time::Duration;

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;

#[cfg(feature = "chess")]
use crate::chess::{ChessMove, Piece, Square};
use crate::uci::{
    MessageList, UciFen, UciInfoAttribute, UciMessage, UciSearchControl, UciTimeControl,
};
#[cfg(not(feature = "chess"))]
use crate::uci::{UciMove, UciPiece, UciSquare};
use crate::uci::ProtectionState;
use crate::UciOptionConfig;

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
    let mut ml = MessageList::new();
    do_parse_uci(s, Rule::commands, Some(&mut ml))?;

    Ok(ml)
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
    let mut ml = MessageList::new();
    do_parse_uci(s, Rule::commands_ignore_unknown, Some(&mut ml)).unwrap();

    ml
}

/// This is like `parse`, except that it returns a `UciMessage::UnknownMessage` variant if it does not recognize the
/// message.
///
/// /// # Examples
///
/// ```
/// use vampirc_uci::UciMessage;
/// use vampirc_uci::parse_with_unknown;
///
/// let messages = parse_with_unknown("not really a message\n");
/// assert_eq!(messages.len(), 1);
/// ```
pub fn parse_with_unknown(s: &str) -> MessageList {
    let parse_att = parse_strict(s);

    if let Err(e) = parse_att {
        let m = UciMessage::Unknown(s.trim_end().to_owned(), Some(e));
        return vec![m];
    }

    parse_att.unwrap()
}

/// Parses and returns a single message, with or without a terminating newline. Usually used
/// in a loop that reads a single line from an input stream, such as the stdin. Note that if the
/// message is unrecognizable to the parser, a `UciMessage::UnknownMessage` variant is returned.
///
/// /// # Examples
///
/// ```
/// use std::io::{self, BufRead};
/// use vampirc_uci::{UciMessage, parse_one};
///
/// for line in io::stdin().lock().lines() {
///         let msg: UciMessage = parse_one(&line.unwrap());
///         println!("Received message: {}", msg);
///     }
/// ```
pub fn parse_one(s: &str) -> UciMessage {
    let r = do_parse_uci(s, Rule::single_message_per_line, None);

    if let Err(e) = r {
        let m = UciMessage::Unknown(s.trim_end().to_owned(), Some(e));
        return m;
    }

    if let Some(m) = r.unwrap() {
        return m;
    }

    return UciMessage::Unknown(String::new(), None);
}

fn do_parse_uci(s: &str, top_rule: Rule, mut ml: Option<&mut MessageList>) -> Result<Option<UciMessage>, Error<Rule>> {
    let pairs = UciParser::parse(top_rule, s)?;

    let mut single: Option<UciMessage> = None;

    pairs
        .map(|pair: Pair<_>| {
            match pair.as_rule() {
                Rule::uci => UciMessage::Uci,
                Rule::debug => {
                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::switch => {
                                return UciMessage::Debug(
                                    sp.as_span().as_str().eq_ignore_ascii_case("on"),
                                );
                            }
                            _ => unreachable!(),
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
                            _ => (),
                        }
                    }

                    let val = if value != String::default() {
                        Some(value)
                    } else {
                        None
                    };
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
                                        Rule::register_name => {
                                            name = spi.as_span().as_str();
                                        }
                                        Rule::register_code => {
                                            return UciMessage::register_code(name, spi.as_str());
                                        }
                                        _ => (),
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                    }

                    unreachable!()
                }
                Rule::ucinewgame => UciMessage::UciNewGame,
                Rule::stop => UciMessage::Stop,
                Rule::ponderhit => UciMessage::PonderHit,
                Rule::quit => UciMessage::Quit,
                Rule::position => {
                    let mut startpos = false;
                    let mut fen: Option<UciFen> = None;
                    #[cfg(not(feature = "chess"))] let mut moves: Vec<UciMove> = Default::default();
                    #[cfg(feature = "chess")] let mut moves: Vec<ChessMove> = Default::default();

                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::startpos => {
                                startpos = true;
                            }
                            Rule::fen => fen = Some(UciFen::from(sp.as_span().as_str())),
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
                                    match spi.as_rule() {
                                        Rule::go_ponder => {
                                            time_control = Some(UciTimeControl::Ponder);
                                        }
                                        Rule::go_infinite => {
                                            time_control = Some(UciTimeControl::Infinite);
                                        }
                                        Rule::go_movetime => {
                                            time_control = Some(UciTimeControl::MoveTime(
                                                Duration::from_millis(parse_milliseconds(spi)),
                                            ));
                                        }
                                        Rule::go_timeleft => {
                                            if !tl {
                                                tl = true;
                                            }

                                            for sspi in spi.into_inner() {
                                                match sspi.as_rule() {
                                                    Rule::wtime => {
                                                        wtime = Some(parse_milliseconds(sspi));
                                                    }
                                                    Rule::btime => {
                                                        btime = Some(parse_milliseconds(sspi));
                                                    }
                                                    Rule::winc => {
                                                        winc = Some(parse_milliseconds(sspi));
                                                    }
                                                    Rule::binc => {
                                                        binc = Some(parse_milliseconds(sspi));
                                                    }
                                                    Rule::movestogo => {
                                                        moves_to_go =
                                                            Some(parse_u8(sspi, Rule::digits3));
                                                    }
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
                                        }
                                        Rule::mate => {
                                            search.mate = Some(parse_u8(spi, Rule::digits3))
                                        }
                                        Rule::nodes => {
                                            search.nodes = Some(parse_u64(spi, Rule::digits12))
                                        }
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
                            white_time: wtime.map(|millis| Duration::from_millis(millis)),
                            black_time: btime.map(|millis| Duration::from_millis(millis)),
                            white_increment: winc.map(|millis| Duration::from_millis(millis)),
                            black_increment: binc.map(|millis| Duration::from_millis(millis)),
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
                Rule::id => {
                    for sp in pair.into_inner() {
                        let id_rule: Rule = sp.as_rule();
                        match id_rule {
                            Rule::id_name | Rule::id_author => {
                                return parse_id_text(sp, id_rule);
                            }
                            _ => {}
                        }
                    }

                    unreachable!()
                }
                Rule::uciok => UciMessage::UciOk,
                Rule::readyok => UciMessage::ReadyOk,
                Rule::bestmove => {
                    #[cfg(not(feature = "chess"))] let mut bm: Option<UciMove> = None;
                    #[cfg(not(feature = "chess"))] let mut ponder: Option<UciMove> = None;
                    #[cfg(feature = "chess")] let mut bm: Option<ChessMove> = None;
                    #[cfg(feature = "chess")] let mut ponder: Option<ChessMove> = None;
                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::a_move => {
                                bm = Some(parse_a_move(sp));
                            }
                            Rule::bestmove_ponder => {
                                for ssp in sp.into_inner() {
                                    match ssp.as_rule() {
                                        Rule::a_move => ponder = Some(parse_a_move(ssp)),
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    UciMessage::BestMove {
                        best_move: bm.unwrap(),
                        ponder,
                    }
                }
                Rule::copyprotection | Rule::registration => {
                    let mut ps: Option<ProtectionState> = None;
                    let pc = pair.clone();
                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::protection_checking => ps = Some(ProtectionState::Checking),
                            Rule::protection_ok => ps = Some(ProtectionState::Ok),
                            Rule::protection_error => ps = Some(ProtectionState::Error),
                            _ => {}
                        }
                    }

                    if pc.as_rule() == Rule::copyprotection {
                        UciMessage::CopyProtection(ps.unwrap())
                    } else {
                        UciMessage::Registration(ps.unwrap())
                    }
                }
                Rule::option => {
                    let mut name: Option<&str> = None;
                    let mut opt_default: Option<&str> = None;
                    let mut opt_min: Option<i64> = None;
                    let mut opt_max: Option<i64> = None;
                    let mut opt_var: Vec<String> = Vec::default();
                    let mut type_pair: Option<Pair<Rule>> = None;

                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::option_name2 => {
                                name = Some(sp.as_span().as_str());
                            }
                            Rule::option_type => {
                                for spi in sp.into_inner() {
                                    match spi.as_rule() {
                                        Rule::option_check
                                        | Rule::option_spin
                                        | Rule::option_combo
                                        | Rule::option_string
                                        | Rule::option_button => {
                                            type_pair = Some(spi);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Rule::option_default => {
                                opt_default = Some(sp.as_span().as_str());
                            }
                            Rule::option_min => {
                                opt_min = Some(parse_i64(sp, Rule::i64));
                            }
                            Rule::option_max => {
                                opt_max = Some(parse_i64(sp, Rule::i64));
                            }
                            Rule::option_var => {
                                opt_var.push(String::from(sp.as_span().as_str()));
                            }
                            _ => unreachable!(),
                        }
                    }

                    let uoc: UciOptionConfig = match type_pair.unwrap().as_rule() {
                        Rule::option_check => UciOptionConfig::Check {
                            name: String::from(name.unwrap()),
                            default: if let Some(def) = opt_default {
                                match def.to_lowercase().as_str() {
                                    "true" => Some(true),
                                    "false" => Some(false),
                                    _ => None,
                                }
                            } else {
                                None
                            },
                        },
                        Rule::option_spin => UciOptionConfig::Spin {
                            name: String::from(name.unwrap()),
                            default: if let Some(def) = opt_default {
                                if let Ok(def_i64) = str::parse::<i64>(def) {
                                    Some(def_i64)
                                } else {
                                    None
                                }
                            } else {
                                None
                            },
                            min: if let Some(min1) = opt_min {
                                Some(min1)
                            } else {
                                None
                            },
                            max: if let Some(max1) = opt_max {
                                Some(max1)
                            } else {
                                None
                            },
                        },
                        Rule::option_combo => UciOptionConfig::Combo {
                            name: String::from(name.unwrap()),
                            default: if let Some(def) = opt_default {
                                if def.eq_ignore_ascii_case("<empty>") {
                                    Some(String::from(""))
                                } else {
                                    Some(String::from(def))
                                }
                            } else {
                                None
                            },
                            var: opt_var,
                        },
                        Rule::option_string => UciOptionConfig::String {
                            name: String::from(name.unwrap()),
                            default: if let Some(def) = opt_default {
                                if def.eq_ignore_ascii_case("<empty>") {
                                    Some(String::from(""))
                                } else {
                                    Some(String::from(def))
                                }
                            } else {
                                None
                            },
                        },
                        Rule::option_button => UciOptionConfig::Button {
                            name: String::from(name.unwrap()),
                        },
                        _ => unreachable!(),
                    };

                    UciMessage::Option(uoc)
                }
                Rule::info => {
                    let mut info_attr: Vec<UciInfoAttribute> = vec![];

                    for sp in pair.into_inner() {
                        match sp.as_rule() {
                            Rule::info_attribute => {
                                for spi in sp.into_inner() {
                                    match spi.as_rule() {
                                        Rule::info_depth => {
                                            let info_depth = UciInfoAttribute::Depth(parse_u8(
                                                spi,
                                                Rule::digits3,
                                            ));
                                            info_attr.push(info_depth);
                                            break;
                                        }
                                        Rule::info_seldepth => {
                                            let info_depth = UciInfoAttribute::SelDepth(parse_u8(
                                                spi,
                                                Rule::digits3,
                                            ));
                                            info_attr.push(info_depth);
                                            break;
                                        }
                                        Rule::info_time => {
                                            let info_time = UciInfoAttribute::Time(Duration::from_millis(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            )));
                                            info_attr.push(info_time);
                                            break;
                                        }
                                        Rule::info_nodes => {
                                            let info_nodes = UciInfoAttribute::Nodes(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            ));
                                            info_attr.push(info_nodes);
                                            break;
                                        }
                                        Rule::info_currmovenum => {
                                            let an_info = UciInfoAttribute::CurrMoveNum(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            )
                                                as u16);
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        Rule::info_hashfull => {
                                            let an_info = UciInfoAttribute::HashFull(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            )
                                                as u16);
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        Rule::info_nps => {
                                            let an_info = UciInfoAttribute::Nps(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            ));
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        Rule::info_tbhits => {
                                            let an_info = UciInfoAttribute::TbHits(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            ));
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        Rule::info_sbhits => {
                                            let an_info = UciInfoAttribute::SbHits(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            ));
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        Rule::info_cpuload => {
                                            let an_info = UciInfoAttribute::CpuLoad(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            )
                                                as u16);
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        Rule::info_multipv => {
                                            let an_info = UciInfoAttribute::MultiPv(parse_u64(
                                                spi,
                                                Rule::digits12,
                                            )
                                                as u16);
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        Rule::info_pv => {
                                            #[cfg(not(feature = "chess"))] let mut mv: Vec<UciMove> = vec![];
                                            #[cfg(feature = "chess")] let mut mv: Vec<ChessMove> = vec![];
                                            for spii in spi.into_inner() {
                                                match spii.as_rule() {
                                                    Rule::a_move => {
                                                        let a_move = parse_a_move(spii);
                                                        mv.push(a_move);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            info_attr.push(UciInfoAttribute::Pv(mv));
                                            break;
                                        }
                                        Rule::info_refutation => {
                                            #[cfg(not(feature = "chess"))] let mut mv: Vec<UciMove> = vec![];
                                            #[cfg(feature = "chess")] let mut mv: Vec<ChessMove> = vec![];
                                            for spii in spi.into_inner() {
                                                match spii.as_rule() {
                                                    Rule::a_move => {
                                                        let a_move = parse_a_move(spii);
                                                        mv.push(a_move);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            info_attr.push(UciInfoAttribute::Refutation(mv));
                                            break;
                                        }
                                        Rule::info_currline => {
                                            #[cfg(not(feature = "chess"))] let mut mv: Vec<UciMove> = vec![];
                                            #[cfg(feature = "chess")] let mut mv: Vec<ChessMove> = vec![];
                                            let mut cpu_nr: Option<u16> = None;
                                            for spii in spi.into_inner() {
                                                match spii.as_rule() {
                                                    Rule::a_move => {
                                                        let a_move = parse_a_move(spii);
                                                        mv.push(a_move);
                                                    }
                                                    Rule::info_cpunr => {
                                                        cpu_nr =
                                                            Some(parse_u64(spii, Rule::digits3)
                                                                as u16);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            info_attr.push(UciInfoAttribute::CurrLine {
                                                cpu_nr,
                                                line: mv,
                                            });
                                            break;
                                        }
                                        Rule::info_string => {
                                            for spii in spi.into_inner() {
                                                match spii.as_rule() {
                                                    Rule::info_string_string => {
                                                        let an_info = UciInfoAttribute::String(
                                                            spii.as_span().as_str().to_owned(),
                                                        );
                                                        info_attr.push(an_info);
                                                        break;
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            break;
                                        }
                                        Rule::info_currmove => {
                                            for spii in spi.into_inner() {
                                                match spii.as_rule() {
                                                    Rule::a_move => {
                                                        let an_info = UciInfoAttribute::CurrMove(
                                                            parse_a_move(spii),
                                                        );
                                                        info_attr.push(an_info);
                                                        break;
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            break;
                                        }
                                        Rule::info_score => {
                                            let mut cp: Option<i32> = None;
                                            let mut mate: Option<i8> = None;
                                            let mut lb: Option<bool> = None;
                                            let mut ub: Option<bool> = None;

                                            for spii in spi.into_inner() {
                                                match spii.as_rule() {
                                                    Rule::info_cp => cp = Some(parse_i64(spii, Rule::i64) as i32),
                                                    Rule::info_mate => mate = Some(parse_i64(spii, Rule::i64) as i8),
                                                    Rule::info_lowerbound => lb = Some(true),
                                                    Rule::info_upperbound => ub = Some(true),
                                                    _ => {}
                                                }
                                            }

                                            info_attr.push(UciInfoAttribute::Score {
                                                cp,
                                                mate,
                                                lower_bound: lb,
                                                upper_bound: ub,
                                            });
                                        }
                                        Rule::info_any => {
                                            let mut s: Option<String> = None;
                                            let mut t: Option<String> = None;

                                            for spii in spi.into_inner() {
                                                match spii.as_rule() {
                                                    Rule::token => {
                                                        t = Some(
                                                            spii.as_span().as_str().to_owned(),
                                                        );
                                                    }
                                                    Rule::info_string_string => {
                                                        s = Some(
                                                            spii.as_span().as_str().to_owned(),
                                                        );
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            let an_info =
                                                UciInfoAttribute::Any(t.unwrap(), s.unwrap());
                                            info_attr.push(an_info);
                                            break;
                                        }
                                        _ => unreachable!(),
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                    }

                    UciMessage::Info(info_attr)
                }

                _ => unreachable!(),
            }
        })
        .for_each(|msg| {
            if let Some(a_ml) = &mut ml {
                (*a_ml).push(msg);
            } else {
                single = Some(msg);
            }
        });

    Ok(single)
}

fn parse_id_text(id_pair: Pair<Rule>, rule: Rule) -> UciMessage {
    for sp in id_pair.into_inner() {
        match sp.as_rule() {
            Rule::id_text => {
                let text = sp.as_span().as_str();
                match rule {
                    Rule::id_name => {
                        return UciMessage::Id {
                            name: Some(String::from(text)),
                            author: None,
                        };
                    }
                    Rule::id_author => {
                        return UciMessage::Id {
                            author: Some(String::from(text)),
                            name: None,
                        };
                    }
                    _ => unreachable!(),
                }
            }
            _ => {}
        }
    }

    unreachable!();
}

#[cfg(not(feature = "chess"))]
fn parse_square(sq_pair: Pair<Rule>) -> UciSquare {
    let mut file: char = '\0';
    let mut rank: u8 = 0;

    match sq_pair.as_rule() {
        Rule::square => {
            for sp in sq_pair.into_inner() {
                match sp.as_rule() {
                    Rule::file => {
                        file = sp.as_span().as_str().chars().into_iter().next().unwrap();
                    }
                    Rule::rank => {
                        rank = str::parse(sp.as_span().as_str()).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        }
        _ => unreachable!(),
    }

    UciSquare::from(file, rank)
}

#[cfg(feature = "chess")]
fn parse_square(sq_pair: Pair<Rule>) -> Square {
    let mut file: char = '\0';
    let mut rank: u8 = 0;

    match sq_pair.as_rule() {
        Rule::square => {
            for sp in sq_pair.into_inner() {
                match sp.as_rule() {
                    Rule::file => {
                        file = sp.as_span().as_str().chars().into_iter().next().unwrap();
                    }
                    Rule::rank => {
                        rank = str::parse(sp.as_span().as_str()).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        }
        _ => unreachable!(),
    }

    Square::from_string(file.to_string() + rank.to_string().as_str()).unwrap()
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

fn parse_i64(pair: Pair<Rule>, rule: Rule) -> i64 {
    for sp in pair.into_inner() {
        if sp.as_rule() == rule {
            return str::parse::<i64>(sp.as_span().as_str()).unwrap();
        }
    }

    0
}

#[cfg(not(feature = "chess"))]
fn parse_a_move(sp: Pair<Rule>) -> UciMove {
    let mut from_sq = UciSquare::default();
    let mut to_sq = UciSquare::default();
    let mut promotion: Option<UciPiece> = None;

    for move_token in sp.into_inner() {
        match move_token.as_rule() {
            Rule::from_sq => {
                from_sq = parse_square(move_token.into_inner().next().unwrap());
            }
            Rule::to_sq => {
                to_sq = parse_square(move_token.into_inner().next().unwrap());
            }
            Rule::promotion => {
                promotion = Some(UciPiece::from_str(move_token.as_span().as_str()).unwrap());
            }
            _ => unreachable!(),
        }
    }

    UciMove {
        from: from_sq,
        to: to_sq,
        promotion,
    }
}

#[cfg(feature = "chess")]
fn parse_a_move(sp: Pair<Rule>) -> ChessMove {
    let mut from_sq = Square::default();
    let mut to_sq = Square::default();
    let mut promotion: Option<Piece> = None;

    for move_token in sp.into_inner() {
        match move_token.as_rule() {
            Rule::from_sq => {
                from_sq = parse_square(move_token.into_inner().next().unwrap());
            }
            Rule::to_sq => {
                to_sq = parse_square(move_token.into_inner().next().unwrap());
            }
            Rule::promotion => {
                promotion = Some(piece_from_str(move_token.as_span().as_str()).unwrap());
            }
            _ => unreachable!(),
        }
    }

    ChessMove::new(from_sq, to_sq, promotion)
}

#[cfg(feature = "chess")]
fn piece_from_str(s: &str) -> Result<Piece, FmtError> {
    match s.to_ascii_lowercase().as_str() {
        "n" => Ok(Piece::Knight),
        "p" => Ok(Piece::Pawn),
        "b" => Ok(Piece::Bishop),
        "r" => Ok(Piece::Rook),
        "k" => Ok(Piece::King),
        "q" => Ok(Piece::Queen),
        _ => Err(FmtError)
    }
}

#[cfg(test)]
mod tests {
    use std::io::*;

    use crate::uci::Serializable;

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
            _ => unreachable!(),
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
            _ => unreachable!(),
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
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_set_option_str() {
        let ml =
            parse_strict("setoption name NalimovPath value c:\\chess\\tb\\4;c:\\chess\\tb\\5\n")
                .unwrap();
        assert_eq!(ml.len(), 1);
        let so = &ml[0];

        match so {
            UciMessage::SetOption { name, value } => {
                assert_eq!(*name, String::from("NalimovPath"));
                let val = value.clone();
                assert_eq!(val.is_some(), true);
                assert_eq!(
                    val.unwrap().as_str(),
                    String::from("c:\\chess\\tb\\4;c:\\chess\\tb\\5")
                );
                assert_eq!(so.as_bool(), None);
            }
            _ => unreachable!(),
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
        assert_eq!(
            ml[0],
            UciMessage::register_code("Matija Kejžar", "4359874324")
        );
    }

    #[test]
    fn test_register_invalid() {
        parse_strict("register name Matija Kejžar\n").expect_err("Parse error expected.");
    }

    #[test]
    fn test_register_invalid2() {
        parse_strict("register code XX-344-00LP name Matija Kejžar\n")
            .expect_err("Parse error expected.");
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

        #[cfg(not(feature = "chess"))]
            {
                let m1 = UciMove {
                    from: UciSquare { file: 'e', rank: 2 },
                    to: UciSquare { file: 'e', rank: 4 },
                    promotion: None,
                };

                let m2 = UciMove {
                    from: UciSquare { file: 'e', rank: 7 },
                    to: UciSquare { file: 'e', rank: 5 },
                    promotion: None,
                };

                let pos = UciMessage::Position {
                    startpos: true,
                    fen: None,
                    moves: vec![m1, m2],
                };

                assert_eq!(ml[0], pos);
            }

        #[cfg(feature = "chess")]
            {
                let m1 = ChessMove::new(Square::E2, Square::E4, None);
                let m2 = ChessMove::new(Square::E7, Square::E5, None);


                let pos = UciMessage::Position {
                    startpos: true,
                    fen: None,
                    moves: vec![m1, m2],
                };

                assert_eq!(ml[0], pos);
            }
    }

    #[test]
    fn test_position_startpos_as_fen() {
        let ml = parse_strict(
            "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves d2d4\r\n",
        )
            .unwrap();
        assert_eq!(ml.len(), 1);

        #[cfg(not(feature = "chess"))]
            let m1 = UciMove {
            from: UciSquare { file: 'd', rank: 2 },
            to: UciSquare { file: 'd', rank: 4 },
            promotion: None,
        };

        #[cfg(feature = "chess")]
            let m1 = ChessMove::new(Square::D2, Square::D4, None);

        let pos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(String::from(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            ))),
            moves: vec![m1],
        };

        assert_eq!(ml[0], pos);
    }

    // 2k5/6PR/8/8/2b4P/8/6K1/8 w - -
    #[test]
    fn test_position_endgame() {
        let ml =
            parse_strict("position fen 2k5/6PR/8/8/2b4P/8/6K1/8 w - - 0 53 moves g7g8q c4g8\r\n")
                .unwrap();
        assert_eq!(ml.len(), 1);

        #[cfg(not(feature = "chess"))]
        let m1 = UciMove {
            from: UciSquare { file: 'g', rank: 7 },
            to: UciSquare { file: 'g', rank: 8 },
            promotion: Some(UciPiece::Queen),
        };

        #[cfg(not(feature = "chess"))]
            let m2 = UciMove {
            from: UciSquare { file: 'c', rank: 4 },
            to: UciSquare { file: 'g', rank: 8 },
            promotion: None,
        };

        #[cfg(feature = "chess")]
            let m1 = ChessMove::new(Square::G7, Square::G8, Some(Piece::Queen));

        #[cfg(feature = "chess")]
            let m2 = ChessMove::new(Square::C4, Square::G8, None);

        let pos = UciMessage::Position {
            startpos: false,
            fen: Some(UciFen(String::from("2k5/6PR/8/8/2b4P/8/6K1/8 w - - 0 53"))),
            moves: vec![m1, m2],
        };

        assert_eq!(ml[0], pos);
    }

    #[test]
    fn test_position_incorrect_fen() {
        parse_strict("position fen 2k50/6PR/8/8/2b4P/8/6K1/8 w - - 0 53 moves g7g8q c4g8\r\n")
            .expect_err("Parse should fail.");
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
            fen: Some(UciFen(String::from(
                "2k5/6PR/8/8/2b4P/8/6K1/8 w   - - 0 53",
            ))),
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

        assert_eq!(ml[0], UciMessage::go_movetime(Duration::from_millis(55055)));
    }

    #[test]
    fn test_go_timeleft() {
        let ml = parse_strict("go wtime 903000 btime 770908 winc 15000 movestogo 17 binc 10000\n")
            .unwrap();
        assert_eq!(ml.len(), 1);

        let tl = UciTimeControl::TimeLeft {
            white_time: Some(Duration::from_millis(903000)),
            black_time: Some(Duration::from_millis(770908)),
            white_increment: Some(Duration::from_millis(15000)),
            black_increment: Some(Duration::from_millis(10000)),
            moves_to_go: Some(17),
        };

        assert_eq!(
            ml[0],
            UciMessage::Go {
                search_control: None,
                time_control: Some(tl),
            }
        );
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

        #[cfg(not(feature = "chess"))]
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
                },
            ],
        };

        #[cfg(feature = "chess")]
            let sc = UciSearchControl {
            depth: None,
            nodes: Some(79093455456),
            mate: None,
            search_moves: vec![
                ChessMove::new(Square::E2, Square::E4, None),
                ChessMove::new(Square::D2, Square::D4, None),
                ChessMove::new(Square::G2, Square::G1, Some(Piece::Knight)),
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
        let ml =
            parse_strict("go movetime 10000 searchmoves a1h8 depth 6 nodes 55000000\n").unwrap();
        assert_eq!(ml.len(), 1);

        let tc = UciTimeControl::MoveTime(Duration::from_millis(10000));

        #[cfg(not(feature = "chess"))]
            let sc = UciSearchControl {
            depth: Some(6),
            nodes: Some(55000000),
            mate: None,
            search_moves: vec![UciMove::from_to(
                UciSquare::from('a', 1),
                UciSquare::from('h', 8),
            )],
        };

        #[cfg(feature = "chess")]
            let sc = UciSearchControl {
            depth: Some(6),
            nodes: Some(55000000),
            mate: None,
            search_moves: vec![
                ChessMove::new(Square::A1, Square::H8, None),
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
            UciMessage::Position { .. } => {}
            _ => panic!("Expected a `position` message here"),
        };

        match ml[1] {
            UciMessage::Go { .. } => {}
            _ => panic!("Expected a `go` message here"),
        };
    }

    #[test]
    #[should_panic]
    fn test_strict_mode() {
        parse_strict("position startpos\nunknown command\ngo ponder searchmoves e2e4 d2d4\n")
            .unwrap();
    }

    #[test]
    fn test_id() {
        let ml = parse_strict("id name Vampirc 1.0\nid    author    Matija Kejžar\n").unwrap();
        assert_eq!(ml.len(), 2);

        let result = UciMessage::Id {
            name: Some("Vampirc 1.0".to_string()),
            author: None,
        };

        assert_eq!(ml[0], result);

        let result2 = UciMessage::Id {
            author: Some("Matija Kejžar".to_string()),
            name: None,
        };

        assert_eq!(ml[1], result2);
    }

    #[test]
    fn test_uciok() {
        let ml = parse_strict("uci\n    uciok    \r\n").unwrap();
        assert_eq!(ml.len(), 2);
        assert_eq!(ml[0], UciMessage::Uci);
        assert_eq!(ml[1], UciMessage::UciOk);
    }

    #[test]
    fn test_readyok() {
        let ml = parse_strict("isready\nreadyok\n").unwrap();
        assert_eq!(ml.len(), 2);
        assert_eq!(ml[0], UciMessage::IsReady);
        assert_eq!(ml[1], UciMessage::ReadyOk);
    }

    // bestmove g1f3
    #[test]
    fn test_bestmove() {
        let ml = parse_strict("bestmove  g1f3\n").unwrap();
        assert_eq!(ml.len(), 1);

        #[cfg(not(feature = "chess"))]
            let m = UciMessage::BestMove {
            best_move: UciMove {
                from: UciSquare::from('g', 1),
                to: UciSquare::from('f', 3),
                promotion: None,
            },

            ponder: None,
        };

        #[cfg(feature = "chess")]
            let m = UciMessage::BestMove {
            best_move: ChessMove::new(Square::G1, Square::F3, None),

            ponder: None,
        };

        assert_eq!(m, ml[0]);
    }

    // bestmove g1f3 ponder d8f6
    #[test]
    fn test_bestmove_with_ponder() {
        let ml = parse_strict("bestmove g1f3 ponder d8f6\n").unwrap();
        assert_eq!(ml.len(), 1);

        #[cfg(not(feature = "chess"))]
        let m = UciMessage::BestMove {
            best_move: UciMove {
                from: UciSquare::from('g', 1),
                to: UciSquare::from('f', 3),
                promotion: None,
            },

            ponder: Some(UciMove {
                from: UciSquare::from('d', 8),
                to: UciSquare::from('f', 6),
                promotion: None,
            }),
        };

        #[cfg(feature = "chess")]
            let m = UciMessage::BestMove {
            best_move: ChessMove::new(Square::G1, Square::F3, None),

            ponder: Some(ChessMove::new(Square::D8, Square::F6, None)),
        };

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_copyprotection() {
        let ml = parse_strict("copyprotection checking\ncopyprotection   ok\n").unwrap();
        assert_eq!(ml.len(), 2);
        assert_eq!(ml[0], UciMessage::CopyProtection(ProtectionState::Checking));
        assert_eq!(ml[1], UciMessage::CopyProtection(ProtectionState::Ok));
    }

    #[test]
    fn test_registration() {
        let ml = parse_strict("registration   checking\nregistration error\n").unwrap();
        assert_eq!(ml.len(), 2);
        assert_eq!(ml[0], UciMessage::Registration(ProtectionState::Checking));
        assert_eq!(ml[1], UciMessage::Registration(ProtectionState::Error));
    }

    #[test]
    fn test_parse_option_check() {
        let ml = parse_strict("option name Nullmove type check default true\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::Check {
            name: "Nullmove".to_string(),
            default: Some(true),
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_check_no_default() {
        let ml = parse_strict("option    name   A long option name type  check   \n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::Check {
            name: "A long option name".to_string(),
            default: None,
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_spin() {
        let ml =
            parse_strict("option name Selectivity type spin default 2 min 0 max 4\n\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::Spin {
            name: "Selectivity".to_string(),
            default: Some(2),
            min: Some(0),
            max: Some(4),
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_spin_no_default() {
        let ml = parse_strict(
            "option name A spin option without a default type spin min -5676 max -33\n",
        )
            .unwrap();

        let m = UciMessage::Option(UciOptionConfig::Spin {
            name: "A spin option without a default".to_string(),
            default: None,
            min: Some(-5676),
            max: Some(-33),
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_spin_just_min() {
        let ml = parse_strict("option name JUST MIN type spin min -40964656\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::Spin {
            name: "JUST MIN".to_string(),
            default: None,
            min: Some(-40964656),
            max: None,
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_spin_just_max() {
        let ml = parse_strict("option name just_max type spin max 56565464509\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::Spin {
            name: "just_max".to_string(),
            default: None,
            max: Some(56565464509),
            min: None,
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_spin_just_default_and_max() {
        let ml = parse_strict("option name def max type spin default -5 max 55\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::Spin {
            name: "def max".to_string(),
            default: Some(-5),
            max: Some(55),
            min: None,
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_combo() {
        let ml = parse_strict(
            "option name Style type combo default Normal var Solid var Normal var Risky\n",
        )
            .unwrap();

        let m = UciMessage::Option(UciOptionConfig::Combo {
            name: "Style".to_string(),
            default: Some("Normal".to_string()),
            var: vec![
                String::from("Solid"),
                String::from("Normal"),
                String::from("Risky"),
            ],
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_combo_no_default() {
        let ml = parse_strict(
            "option name Some ccccc-combo type combo      var A B C var D E   F var 1 2 3\n",
        )
            .unwrap();

        let m = UciMessage::Option(UciOptionConfig::Combo {
            name: "Some ccccc-combo".to_string(),
            default: None,
            var: vec![
                String::from("A B C"),
                String::from("D E   F"),
                String::from("1 2 3"),
            ],
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_string() {
        let ml = parse_strict("option name Nalimov Path  type string default c:\\\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::String {
            name: "Nalimov Path".to_string(),
            default: Some("c:\\".to_string()),
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_string_no_default() {
        let ml = parse_strict("option name NP type string\r\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::String {
            name: "NP".to_string(),
            default: None,
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_button() {
        let ml = parse_strict("option name Clear Hash type button\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::Button {
            name: "Clear Hash".to_string(),
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_button_ignore_default() {
        let ml =
            parse_strict("option name CH type button default Ignore min 5 max 6 var A var B\n")
                .unwrap();

        let m = UciMessage::Option(UciOptionConfig::Button {
            name: "CH".to_string(),
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_option_string_empty() {
        let ml = parse_strict("option name Nalimov Path  type string default <empty>\n").unwrap();

        let m = UciMessage::Option(UciOptionConfig::String {
            name: "Nalimov Path".to_string(),
            default: Some("".to_string()),
        });

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_depth() {
        let ml = parse_strict("info depth 23\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::Depth(23)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_seldepth() {
        let ml = parse_strict("info seldepth 9\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::SelDepth(9)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_time() {
        let ml = parse_strict("info    time    9002\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::Time(Duration::from_millis(9002))]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_nodes() {
        let ml = parse_strict("info nodes    56435234425\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::Nodes(56435234425)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_currmovenum() {
        let ml = parse_strict("info currmovenum 102\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::CurrMoveNum(102)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_hashfull() {
        let ml = parse_strict("info hashfull 673\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::HashFull(673)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_nps() {
        let ml = parse_strict("info nps 12003\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::Nps(12003)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_tbhits() {
        let ml = parse_strict("info tbhits 5305\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::TbHits(5305)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_sbhits() {
        let ml = parse_strict("info sbhits 0\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::SbHits(0)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_cpuload() {
        let ml = parse_strict("info cpuload 773\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::CpuLoad(773)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_multipv() {
        let ml = parse_strict("info multipv 2\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::MultiPv(2)]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_string() {
        let ml = parse_strict("info string    I am   the Walrus! Cuckoo cachoo.\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::String(
            "I am   the Walrus! Cuckoo cachoo.".to_owned(),
        )]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_any() {
        let ml = parse_strict("info UCI_Whatever -29 A3 57\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::Any(
            "UCI_Whatever".to_owned(),
            "-29 A3 57".to_owned(),
        )]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_currmove() {
        let ml = parse_strict("info currmove a7a8q\n").unwrap();

        #[cfg(not(feature = "chess"))]
            let m = UciMessage::Info(vec![UciInfoAttribute::CurrMove(UciMove {
            from: UciSquare::from('a', 7),
            to: UciSquare::from('a', 8),
            promotion: Some(UciPiece::Queen),
        })]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![UciInfoAttribute::CurrMove(ChessMove::new(Square::A7, Square::A8, Some(Piece::Queen)))]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_pv() {
        let ml = parse_strict("info pv e2e4 e7e5 g1f3\n").unwrap();

        #[cfg(not(feature = "chess"))]
            let m = UciMessage::Info(vec![UciInfoAttribute::Pv(vec![
            UciMove::from_to(UciSquare::from('e', 2), UciSquare::from('e', 4)),
            UciMove::from_to(UciSquare::from('e', 7), UciSquare::from('e', 5)),
            UciMove::from_to(UciSquare::from('g', 1), UciSquare::from('f', 3)),
        ])]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![UciInfoAttribute::Pv(vec![
            ChessMove::new(Square::E2, Square::E4, None),
            ChessMove::new(Square::E7, Square::E5, None),
            ChessMove::new(Square::G1, Square::F3, None),
        ])]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_parse_info_refutation() {
        let ml = parse_strict("info refutation d1h5 g6h5\n").unwrap();

        #[cfg(not(feature = "chess"))]
            let m = UciMessage::Info(vec![UciInfoAttribute::Refutation(vec![
            UciMove::from_to(UciSquare::from('d', 1), UciSquare::from('h', 5)),
            UciMove::from_to(UciSquare::from('g', 6), UciSquare::from('h', 5)),
        ])]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![UciInfoAttribute::Refutation(vec![
            ChessMove::new(Square::D1, Square::H5, None),
            ChessMove::new(Square::G6, Square::H5, None),
        ])]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_info_currline_no_cpu_nr() {
        let ml = parse_strict("info currline d1h5 g6h5\n").unwrap();

        #[cfg(not(feature = "chess"))]
            let m = UciMessage::Info(vec![UciInfoAttribute::CurrLine {
            cpu_nr: None,
            line: vec![
                UciMove::from_to(UciSquare::from('d', 1), UciSquare::from('h', 5)),
                UciMove::from_to(UciSquare::from('g', 6), UciSquare::from('h', 5)),
            ],
        }]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![UciInfoAttribute::CurrLine {
            cpu_nr: None,
            line: vec![
                ChessMove::new(Square::D1, Square::H5, None),
                ChessMove::new(Square::G6, Square::H5, None),
            ],
        }]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_info_currline_with_cpu_nr() {
        let ml = parse_strict("info currline 1 d1h5 g6h5\n").unwrap();

        #[cfg(not(feature = "chess"))]
            let m = UciMessage::Info(vec![UciInfoAttribute::CurrLine {
            cpu_nr: Some(1),
            line: vec![
                UciMove::from_to(UciSquare::from('d', 1), UciSquare::from('h', 5)),
                UciMove::from_to(UciSquare::from('g', 6), UciSquare::from('h', 5)),
            ],
        }]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![UciInfoAttribute::CurrLine {
            cpu_nr: Some(1),
            line: vec![
                ChessMove::new(Square::D1, Square::H5, None),
                ChessMove::new(Square::G6, Square::H5, None),
            ],
        }]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_info_currline_multi_cpu_nr() {
        let ml = parse_strict("info currline 1 d1h5 g6h5 currline 2 e2e4 currline 3 d2d4 d7d5\n")
            .unwrap();

        #[cfg(not(feature = "chess"))]
        let m = UciMessage::Info(vec![
            UciInfoAttribute::CurrLine {
                cpu_nr: Some(1),
                line: vec![
                    UciMove::from_to(UciSquare::from('d', 1), UciSquare::from('h', 5)),
                    UciMove::from_to(UciSquare::from('g', 6), UciSquare::from('h', 5)),
                ],
            },
            UciInfoAttribute::CurrLine {
                cpu_nr: Some(2),
                line: vec![UciMove::from_to(
                    UciSquare::from('e', 2),
                    UciSquare::from('e', 4),
                )],
            },
            UciInfoAttribute::CurrLine {
                cpu_nr: Some(3),
                line: vec![
                    UciMove::from_to(UciSquare::from('d', 2), UciSquare::from('d', 4)),
                    UciMove::from_to(UciSquare::from('d', 7), UciSquare::from('d', 5)),
                ],
            }
        ]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![
            UciInfoAttribute::CurrLine {
                cpu_nr: Some(1),
                line: vec![
                    ChessMove::new(Square::D1, Square::H5, None),
                    ChessMove::new(Square::G6, Square::H5, None),
                ],
            },
            UciInfoAttribute::CurrLine {
                cpu_nr: Some(2),
                line: vec![ChessMove::new(Square::E2, Square::E4, None)],
            },
            UciInfoAttribute::CurrLine {
                cpu_nr: Some(3),
                line: vec![
                    ChessMove::new(Square::D2, Square::D4, None),
                    ChessMove::new(Square::D7, Square::D5, None),
                ],
            }
        ]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_info_score_cp() {
        let ml = parse_strict("info score cp 20\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::from_centipawns(20)
        ]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_info_score_mate() {
        let ml = parse_strict("info score mate -3\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::from_mate(-3)
        ]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_info_score_cp_lowerbound() {
        let ml = parse_strict("info score cp -75 lowerbound\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::Score {
            cp: Some(-75),
            mate: None,
            lower_bound: Some(true),
            upper_bound: None,
        }
        ]);

        assert_eq!(m, ml[0]);
    }

    #[test]
    fn test_info_score_cp_upperbound() {
        let ml = parse_strict("info score cp 404 upperbound\n").unwrap();

        let m = UciMessage::Info(vec![UciInfoAttribute::Score {
            cp: Some(404),
            mate: None,
            upper_bound: Some(true),
            lower_bound: None,
        }
        ]);

        assert_eq!(m, ml[0]);
    }

    // info score cp 13  depth 1 nodes 13 time 15 pv f1b5
    #[test]
    fn test_info_multi1() {
        let ml = parse_strict("info score cp 13  depth 1 nodes 13 time 15 pv f1b5\n").unwrap();
        println!("{}", ml[0].serialize());
        assert_eq!(1, ml.len());

        #[cfg(not(feature = "chess"))]
            let m = UciMessage::Info(vec![
            UciInfoAttribute::from_centipawns(13),
            UciInfoAttribute::Depth(1),
            UciInfoAttribute::Nodes(13),
            UciInfoAttribute::Time(Duration::from_millis(15)),
            UciInfoAttribute::Pv(vec![
                UciMove::from_to(UciSquare::from('f', 1), UciSquare::from('b', 5))
            ])
        ]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![
            UciInfoAttribute::from_centipawns(13),
            UciInfoAttribute::Depth(1),
            UciInfoAttribute::Nodes(13),
            UciInfoAttribute::Time(Duration::from_millis(15)),
            UciInfoAttribute::Pv(vec![
                ChessMove::new(Square::F1, Square::B5, None)
            ])
        ]);

        assert_eq!(m, ml[0]);

        assert_eq!(m.serialize(), "info score cp 13 depth 1 nodes 13 time 15 pv f1b5")
    }

    // info depth 2 seldepth 2
    #[test]
    fn test_info_multi2() {
        let ml = parse_strict("info depth 2 seldepth 2\n").unwrap();
        println!("{}", ml[0].serialize());
        assert_eq!(1, ml.len());

        let m = UciMessage::Info(vec![
            UciInfoAttribute::Depth(2),
            UciInfoAttribute::SelDepth(2),
        ]);

        assert_eq!(m, ml[0]);
        assert_eq!(m.serialize(), "info depth 2 seldepth 2")
    }

    // info score cp 20  depth 3 nodes 423 time 15 pv f1c4 g8f6 b1c3
    #[test]
    fn test_info_multi3() {
        let ml = parse_strict("info score cp 20  depth 3 nodes 423 time 15 pv f1c4 g8f6 b1c3 \n").unwrap();
        println!("{}", ml[0].serialize());
        assert_eq!(1, ml.len());

        #[cfg(not(feature = "chess"))]
        let m = UciMessage::Info(vec![
            UciInfoAttribute::from_centipawns(20),
            UciInfoAttribute::Depth(3),
            UciInfoAttribute::Nodes(423),
            UciInfoAttribute::Time(Duration::from_millis(15)),
            UciInfoAttribute::Pv(vec![
                UciMove::from_to(UciSquare::from('f', 1), UciSquare::from('c', 4)),
                UciMove::from_to(UciSquare::from('g', 8), UciSquare::from('f', 6)),
                UciMove::from_to(UciSquare::from('b', 1), UciSquare::from('c', 3))
            ])
        ]);

        #[cfg(feature = "chess")]
            let m = UciMessage::Info(vec![
            UciInfoAttribute::from_centipawns(20),
            UciInfoAttribute::Depth(3),
            UciInfoAttribute::Nodes(423),
            UciInfoAttribute::Time(Duration::from_millis(15)),
            UciInfoAttribute::Pv(vec![
                ChessMove::new(Square::F1, Square::C4, None),
                ChessMove::new(Square::G8, Square::F6, None),
                ChessMove::new(Square::B1, Square::C3, None),
            ])
        ]);

        assert_eq!(m, ml[0]);

        assert_eq!(m.serialize(), "info score cp 20 depth 3 nodes 423 time 15 pv f1c4 g8f6 b1c3")
    }

    #[test]
    fn test_parse_with_unknown() {
        let ml = parse_with_unknown("not really a message\n");
        println!("{}", ml[0].serialize());
        assert_eq!(1, ml.len());
        assert_eq!(ml[0].is_unknown(), true);

        match &ml[0] {
            UciMessage::Unknown(msg, err) => {
                assert_eq!(msg.as_str(), "not really a message");
                assert_eq!(err.is_some(), true);
                assert_eq!(err.clone().unwrap().to_string(), " --> 1:1\n  |\n1 | not really a message␊\n  | ^---\n  |\n  = expected uci, debug, isready, setoption, register, ucinewgame, stop, quit, ponderhit, position, go, id, uciok, readyok, bestmove, copyprotection, registration, option, or info");
            },
            _ => panic!("Expected a message of type UnknownMessage")
        }
    }

    #[test]
    fn test_parse_with_unknown_success() {
        let ml = parse_with_unknown("uci\nuciok\n");
        assert_eq!(2, ml.len());
        assert_eq!(ml[0].is_unknown(), false);
        assert_eq!(ml[1].is_unknown(), false);

        assert_eq!(ml, vec![UciMessage::Uci, UciMessage::UciOk]);
    }

    #[test]
    fn test_parse_go() {
        parse("go\n");
        let msgs = parse_strict("go\n").unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], UciMessage::go())
    }

    #[ignore]
    #[test]
    fn test_parse_stdin() {
        println!("Enter uci command: ");
        for line in stdin().lock().lines() {
            println!("got line: {:?}", line);
            let msgs: MessageList = parse(&line.unwrap());
            for msg in msgs {
                println!("parsed: {}", msg);
            }
        }
    }

    #[test]
    fn test_no_line_at_end_multi_parse() {
        let msgs = parse("uci\ndebug on\nucinewgame\nstop\nquit");
        assert_eq!(msgs.len(), 5);
        assert_eq!(msgs[0], UciMessage::Uci);
        assert_eq!(msgs[1], UciMessage::Debug(true));
        assert_eq!(msgs[2], UciMessage::UciNewGame);
        assert_eq!(msgs[3], UciMessage::Stop);
        assert_eq!(msgs[4], UciMessage::Quit);
    }

    #[test]
    fn test_no_line_at_end_single_parse() {
        let msgs = parse("uci\n");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], UciMessage::Uci);

        let msgs2 = parse("go ponder\n");
        assert_eq!(msgs2.len(), 1);
        assert_eq!(msgs2[0], UciMessage::go_ponder());
    }

    #[test]
    fn test_no_line_at_end_multi_parse_strict() {
        let msgs = parse("uci\ndebug on\nucinewgame\nstop\nquit");
        assert_eq!(msgs.len(), 5);
        assert_eq!(msgs[0], UciMessage::Uci);
        assert_eq!(msgs[1], UciMessage::Debug(true));
        assert_eq!(msgs[2], UciMessage::UciNewGame);
        assert_eq!(msgs[3], UciMessage::Stop);
        assert_eq!(msgs[4], UciMessage::Quit);
    }

    #[test]
    fn test_no_line_at_end_single_parse_strict() {
        let msgs = parse_strict("uci\n").unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], UciMessage::Uci);

        let msgs2 = parse_strict("info score cp 20").unwrap();
        assert_eq!(msgs2.len(), 1);
        assert_eq!(msgs2[0], UciMessage::Info(vec![UciInfoAttribute::Score {
            cp: Some(20),
            lower_bound: None,
            mate: None,
            upper_bound: None,
        }]));
    }

    // TODO parse_with_unknown should be improved to parse everything it knows and not die immediately
    // on error
    #[test]
    fn test_no_line_at_end_parse_with_unknown_with_unknown() {
        let msgs = parse_with_unknown("uci\ndebug on\nucinewgame\nabc\nstop\nquit");
        assert_eq!(msgs.len(), 1);
        // assert_eq!(msgs[0], UciMessage::Uci);
        // assert_eq!(msgs[1], UciMessage::Debug(true));
        // assert_eq!(msgs[2], UciMessage::UciNewGame);
        // assert_eq!(msgs[4], UciMessage::Stop);
        // assert_eq!(msgs[5], UciMessage::Quit);
    }

    #[test]
    fn test_no_line_at_end_parse_with_unknown() {
        let msgs = parse_with_unknown("uci\ndebug on\nucinewgame\nstop\nquit");
        assert_eq!(msgs.len(), 5);
        assert_eq!(msgs[0], UciMessage::Uci);
        assert_eq!(msgs[1], UciMessage::Debug(true));
        assert_eq!(msgs[2], UciMessage::UciNewGame);
        assert_eq!(msgs[3], UciMessage::Stop);
        assert_eq!(msgs[4], UciMessage::Quit);
    }

    #[test]
    fn test_no_line_at_end_parse_with_unknown_single() {
        let msgs = parse_with_unknown("uciok");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], UciMessage::UciOk);
    }

    #[test]
    fn test_empty_nl_parse() {
        let msgs = parse("\n");
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_empty_nl_parse_strict() {
        let msgs = parse_strict("\n").unwrap();
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_empty_nl_parse_with_unknown() {
        let msgs = parse_with_unknown("\n");
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_empty_parse() {
        let msgs = parse("");
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_empty_parse_strict() {
        let msgs = parse_strict("").unwrap();
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_empty_parse_with_unknown() {
        let msgs = parse_with_unknown("");
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_parse_one_uci() {
        let msg = parse_one("uci");
        assert_eq!(msg, UciMessage::Uci);
    }

    #[test]
    fn test_parse_one_go() {
        let msg = parse_one("go    infinite   \n");
        assert_eq!(msg, UciMessage::go_infinite());
    }

    #[test]
    fn test_parse_one_empty() {
        let msg = parse_one("");
        match msg {
            UciMessage::Unknown(s, _) => {
                assert_eq!(s, String::new());
            },
            _ => panic!("Expected UciMessage::Unknown")
        }
    }

    #[test]
    fn test_parse_one_unknown() {
        let msg = parse_one("ax34\n");
        match msg {
            UciMessage::Unknown(s, _) => {
                assert_eq!(s, String::from("ax34"));
            },
            _ => panic!("Expected UciMessage::Unknown")
        }
    }

    #[test]
    fn test_parse_one_multi_commands() {
        let msg = parse_one("uci\nuciok\n");
        assert_eq!(msg, UciMessage::Uci);
    }
}
