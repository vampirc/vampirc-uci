# vampirc-uci [![Documentation Status](https://docs.rs/vampirc-uci/badge.svg)](https://docs.rs/vampirc-uci)

Vampirc UCI is a [Universal Chess Interface (UCI) protocol](https://en.wikipedia.org/wiki/Universal_Chess_Interface) parser and
serializer. 

The UCI protocol is a way for a chess engine to communicate with a chessboard GUI, such as [Cute Chess](https://cutechess.com/).

The [Vampirc Project](https://vampirc.kejzar.si) is a chess engine and chess library suite, written in Rust. It is named for the
Slovenian grandmaster [Vasja Pirc](https://en.wikipedia.org/wiki/Vasja_Pirc), and, I guess, vampires? I dunno.

Vampirc UCI uses the [PEST parser](https://github.com/pest-parser/pest) to parse the UCI messages. If you want to build your own
abstractions of the protocol, the corresponding PEG grammar is available [here](https://github.com/vampirc/vampirc-uci/blob/master/res/uci.pest).

## Installing the library

To use the crate, declare a dependency on it in your Cargo.toml file:

```toml
[dependencies]
vampirc-uci = "0.11"
```

Then reference the `vampirc_uci` crate in your crate root:
```rust
extern crate vampirc_uci;
```

## Usage

1. Choose and import one of the `parse..` functions. See [Choosing the parsing function](#choosing-the-parsing-function).

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

6. Or, parse and handle input line by line, from, for example, `stdin`:
```rust
use std::io::{self, BufRead};
use vampirc_uci::{UciMessage, parse_one};

for line in io::stdin().lock().lines() {
     let msg: UciMessage = parse_one(&line.unwrap());
     println!("Received message: {}", msg);
}
```

## Choosing the parsing function

There are several parsing functions available, depending on your need and use case. They differ in what
they return and how they handle unrecognized input. The following table may be of assistance in selecting the
parsing function:

| Function             | Returns                                 | Can skip terminating newline | On unrecognised input...                    | 
| -------------------- | ----------------------------------------|------------------------------|---------------------------------------------|
| `parse`              | `MessageList` (a `Vec` of `UciMessage`) | On last command              | Ignores it                                  |
| `parse_strict`       | `MessageList` (a `Vec` of `UciMessage`) | On last command              | Throws a `pest::ParseError`                 |
| `parse_with_unknown` | `MessageList` (a `Vec` of `UciMessage`) | On last command              | Wraps it in a `UciMessage::Unknown` variant |
| `parse_one`          | `UciMessage`                            | Yes                          | Wraps it in a `UciMessage::Unknown` variant |

From my own experience, I recommend using either `parse_with_unknown` if your string can contain multiple commands, or
else `parse_one` if you're doing line by line parsing. That way, your chess engine or tooling can at least log 
unrecognised input, available from `UciMessage::Unknown(String, Error)` variant.  

## Integration with the chess crate (since 0.9.0)

This library (optionally) integrates with the [chess crate](https://crates.io/crates/chess). First, include the 
`vampirc-uci` crate into your project with the `chess` feature:

```toml
[dependencies]
vampirc-uci = {version = "0.11", features = ["chess"]}
```

This will cause the vampirc_uci's internal representation of moves, squares and pieces to be replaced with `chess` 
crate's representation of those concepts. Full table below:

| vampirc_uci 's representation | chess' representation |
| ----------------------------- | --------------------- |
| `vampirc_uci::UciSquare`      | `chess::Square`       |
| `vampirc_uci::UciPiece`       | `chess::Piece`        |
| `vampirc_uci::UciMove`        | `chess::ChessMove`    |

---
**WARNING**

`chess` is a fairly heavy create with some heavy dependencies, so probably only use the integration feature if you're 
building your own chess engine or tooling with it. 

---


## API

The full API documentation is available at [docs.rs](https://docs.rs/vampirc-uci/).

### New in 0.11.0
* Support for negative times, such as negative time left and time increment, as discussed in 
[vampirc-uci doesn't recognize negative times #16](https://github.com/vampirc/vampirc-uci/issues/16).
To support negative durations, the representation of millisecond-based time quantities has been switched
from Rust standard library's `std::time::Duration` to the [chrono crate's](https://crates.io/crates/chrono)
`chrono::Duration` ([doc](https://docs.rs/chrono/0.4.15/chrono/struct.Duration.html)). This is an API-breaking change, hence the version increase.

### New in 0.10.1
* Republish as 0.10.1 due to improper publish. 

### New in 0.10.0
* Added the `parse_one(&str)` method that parses and returns a single command, to be used in a loop
that reads from `stdin` or other `BufReader`. See example above.
* Changed the internal representation of time parameters from `u64` into `std::time::Duration` (breaking 
change).
* Relaxed grammar rules now allow that the last command sent to `parse()` or friends doesn't need to
have a newline terminator. This allows for parsing of, among others, a single command read in a loop from
`stdin::io::stdin().lock().lines()`, which strips the newline characters from the end -
see [vampirc-uci-14](https://github.com/vampirc/vampirc-uci/issues/14).
* Marked the `UciMessage::direction(&self)` method as public.

### New in 0.9.0
* (Optional) integration with [chess crate](https://crates.io/crates/chess) (see above).
* Removed the explicit Safe and Sync implementations.

### New in 0.8.3

* Added the `UciMessage::info_string()` utility function.
* Allowed the empty `go` command (see [Parser cannot parse "go\n"](https://github.com/vampirc/vampirc-uci/issues/9)).

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

This section used to recommend using the [vampirc-io](https://github.com/vampirc/vampirc-io) crate to connect your
UCI-based chess engine to the GUI, but honestly, with recent advances to Rust's async stack support, it is probably
just easier if you do it yourself using, for example, the [async-std library](https://github.com/async-rs/async-std).

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

