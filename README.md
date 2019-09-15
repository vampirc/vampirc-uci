# vampirc-uci [![Build Status](https://travis-ci.org/vampirc/vampirc-uci.svg?branch=master)](https://travis-ci.org/vampirc/vampirc-uci) [![Documentation Status](https://docs.rs/vampirc-uci/badge.svg)](https://docs.rs/vampirc-uci)

Vampirc UCI is a [Universal Chess Interface (UCI) protocol](https://en.wikipedia.org/wiki/Universal_Chess_Interface) parser and
serializer. 

The UCI protocol is a way for a chess engine to communicate with a chessboard GUI, such as [Scid vs. PC](http://scidvspc.sourceforge.net/).

The [Vampirc Project](https://vampirc.kejzar.si) is a chess engine and chess library suite, written in Rust. It is named for the
Slovenian grandmaster [Vasja Pirc](https://en.wikipedia.org/wiki/Vasja_Pirc), and, I guess, vampires? I dunno.

Vampirc UCI uses the [PEST parser](https://github.com/pest-parser/pest) to parse the UCI messages. If you want to build your own
abstractions of the protocol, the corresponding PEG grammar is available [here](https://github.com/vampirc/vampirc-uci/blob/master/res/uci.pest).

## Installing the library

To use the crate, declare a dependency on it in your Cargo.toml file:

```toml
[dependencies]
vampirc-uci = "0.8"
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

### New in 0.8.2

* Added `ByteVecUciMessage` as a `UciMessage` wrapper that keeps the serialized form of the message in the struct as a byte Vector. Useful if
you need to serialize the same message multiple types or support `AsRef<[u8]>` trait for funnelling the messages into a `futures::Sink` or
something.
* Modifications for integration with async [async-std](https://github.com/async-rs/async-std) based [vampirc-io](https://github.com/vampirc/vampirc-io).

### New in 0.8.1

* Added `parse_with_unknown()` method that instead of ignoring unknown messages (like `parse`) or throwing an error (like `parse_strict`) returns
them as a `UciMessage::Unknown` variant.

### New in 0.8.0

* Support for parsing of the `info` message, with the [UciAttributeInfo](https://docs.rs/vampirc-uci/0.8/vampirc_uci/uci/enum.UciInfoAttribute.html) 
enum representing all 17 types of messages described by the UCI documentation, as well as any other info message via the
[Any variant](https://docs.rs/vampirc-uci/0.8/vampirc_uci/uci/enum.UciInfoAttribute.html#variant.Any).

### New in 0.7.5

* Support for parsing of the `option` message.
* Proper support for `<empty>` strings in `option` and `setoption`.

## vampirc-io

This crate goes together well with the [vampirc-io](https://github.com/vampirc/vampirc-io) crate, a library for 
non-blocking communication over standard input and output (which is how UCI communication is usually conducted), 
based on the [async-std framework](https://github.com/async-rs/async-std).

## Limitations and 1.0

The library is functionally complete – it supports the parsing and serialization to string of all the messages
described by the UCI specification. Before the 1.0 version can be released, though, this library needs to be battle
tested more, especially in the upcoming [Vampirc chess engine](https://vampirc.kejzar.si).

Furthermore, as I am fairly new to Rust, I want to make sure the implementation of this protocol parser is Rust-idiomatic
before releasing 1.0. For this reason, the API should not be considered completely stable until 1.0 is released. 

Additionally, some performance testing would also not go amiss.

### Supported engine-bound messages (100%)

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

### Supported GUI-bound messages (100%)

* `id`
* `uciok`
* `readyok`
* `bestmove`
* `copyprotection`
* `registration`
* `option`
* `info`

