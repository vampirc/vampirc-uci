# vampirc-uci [![Build Status](https://travis-ci.org/vampirc/vampirc-uci.svg?branch=master)](https://travis-ci.org/vampirc/vampirc-uci) [![Documentation Status](https://docs.rs/vampirc-uci/badge.svg)](https://docs.rs/vampirc-uci)

Vampirc UCI is a [Universal Chess Interface (UCI) protocol](https://en.wikipedia.org/wiki/Universal_Chess_Interface) parser and
serializer. 

The UCI protocol is a way for a chess engine to communicate with a chessboard GUI, such as [Cute Chess](https://github.com/cutechess/cutechess).

The [Vampirc Project](https://vampirc.kejzar.si) is a chess engine and chess library suite, written in Rust. It is named for the
Slovenian grandmaster [Vasja Pirc](https://en.wikipedia.org/wiki/Vasja_Pirc), and, I guess, vampires? I dunno.

Vampirc UCI uses the [PEST parser](https://github.com/pest-parser/pest) to parse the UCI messages. If you want to build your own
abstractions of the protocol, the corresponding PEG grammar is available [here](https://github.com/vampirc/vampirc-uci/blob/master/res/uci.pest).

## Installing the library

To use the crate, declare a dependency on it in your Cargo.toml file:

```toml
[dependencies]
vampirc-uci = "0.7"
```

Then reference the `vampirc_uci` crate in your crate root:
```rust
extern crate vampirc_uci;
```

## Usage

1. Import either the `parse(..)` method or the `parse_strict(..)` method. The difference between them is that `parse_strict(..)`
will return a `pest::error::Error` if any of the input is unrecognized or violates the rules of the PEG grammar, whereas `parse`
will simply ignore any such input. The latter is the approach recommended by the protocol specification.

```rust
use vampirc_uci::parse;
``` 

2. Some other useful imports (for message representation):

```rust
use vampirc_uci::{UciMessage, MessageList, UciTimeControl, Serializable};
```

3. Parse some input:

```rust
let messages: MessageList = parse("uci\nposition startpos moves e2e4 e7e5\ngo ponder\n");
```

4. Do something with the parsed messages:

```rust
for m in messages {
    match m {
        UciMessage::Uci => {
            // Initialize the UCI mode of the chess engine.
        }
        UciMessage::Position { startpos, fen, moves } => {
            // Set up the starting position in the engine and play the moves e2-e4 and e7-e5
        }
        UciMessage::Go { time_control, search_control } {
            if let Some(tc) = time_control {
                match tc {
                    UciTimeControl::Ponder => {
                        // Put the engine into ponder mode ("think" on opponent's time)
                    }
                    _ => {...}
                }
            }
        }
        _ => {...}
    }
}
```

5. Outputting the messages

```rust
    let message = UciMessage::Option(UciOptionConfig::Spin {
                name: "Selectivity".to_string(),
                default: Some(2),
                min: Some(0),
                max: Some(4),
            });
    
    println!(message); // Outputs "option name Selectivity type spin default 2 min 0 max 4"
```

## API

The full API documentation is available at [docs.rs](https://docs.rs/vampirc-uci/).

## Limitations

The current version 0.6.x only supports the parsing of engine–bound messages. These include:
* `uci`
* `debug`
* `isready`
* `register`
* `position`
* `setoption`
* `ucinewgame`
* `stop`
* `ponderhit`
* `quit`
* `go`

Support for the rest is coming up.

Since 0.6.0, it does, however, support representation and serialization of all the GUI-bound messages:
* `id`
* `uciok`
* `readyok`
* `bestmove`
* `copyprotection`
* `registration`
* `option`
* `info`

