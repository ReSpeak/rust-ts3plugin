TeamSpeak3 Plugin API &emsp; [![Build Status](https://travis-ci.org/Flakebi/rust-ts3plugin.svg?branch=master)](https://travis-ci.org/Flakebi/rust-ts3plugin) [![Latest version](https://img.shields.io/crates/v/ts3plugin.svg)](https://crates.io/crates/ts3plugin)
=====================
The documentation can be found here: [![At docs.rs](https://docs.rs/ts3plugin/badge.svg)](https://docs.rs/ts3plugin)

TeamSpeak 3.3 updates the plugin api version from 22 to 23.
Version 0.4 and above are compatible with this version.

TeamSpeak 3.1.1 updates the plugin api version from 21 to 22 but stays
backwards compatible so plugins with version 21 can still be loaded.

TeamSpeak 3.1 updates the plugin api version from 20 to 21.
Version 0.2 and 0.3 are compatible with this version while version 0.1 is
compatible with the api version 20.

At the moment, not all methods that are exposed by the TeamSpeak API are
available for plugins. If a method that you need is missing, please file an
issue or open a pull request.

## Usage

Add the following to your `Cargo.toml`:

```toml
[package]
name = "<pluginname>"
version = "<version>"
authors = ["<your name>"]
description = "<description>"

[lib]
name = "<pluginname>"
crate-type = ["cdylib"]

[dependencies]
ts3plugin = "0.3"
```

## Example

A fully working example, which creates a plugin that does nothing:

```rust
#[macro_use]
extern crate ts3plugin;

use ts3plugin::*;

struct MyTsPlugin;

impl Plugin for MyTsPlugin {
    // The default name is the crate name, but we can overwrite it here.
    fn name()        -> String { String::from("My Ts Plugin") }
    fn command() -> Option<String> { Some(String::from("myplugin")) }
    fn autoload() -> bool { false }
    fn configurable() -> ConfigureOffer { ConfigureOffer::No }

    // The only required method
    fn new(api: &TsApi) -> Result<Box<MyTsPlugin>, InitError> {
        api.log_or_print("Inited", "MyTsPlugin", LogLevel::Info);
        Ok(Box::new(MyTsPlugin))
        // Or return Err(InitError::Failure) on failure
    }

    // Implement callbacks here

    fn shutdown(&mut self, api: &TsApi) {
        api.log_or_print("Shutdown", "MyTsPlugin", LogLevel::Info);
    }
}

create_plugin!(MyTsPlugin);

```

Projects using this library
---------------------------
 - [TeamSpeak3 Text to Speech](https://github.com/Flakebi/ts3tts)
 - [TsPressor](https://github.com/Splamy/TsPressor)

License
-------
Licensed under either of

 * [Apache License, Version 2.0](LICENSE-APACHE)
 * [MIT license](LICENSE-MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
