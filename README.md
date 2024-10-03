TeamSpeak3 Plugin API &emsp; [![Latest version](https://img.shields.io/crates/v/ts3plugin.svg)](https://crates.io/crates/ts3plugin)
=====================
The documentation can be found here: [![At docs.rs](https://docs.rs/ts3plugin/badge.svg)](https://docs.rs/ts3plugin)

TeamSpeak 3.6 updates the plugin api version to 26.
Version 0.3 is compatible with this version.

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
 - [TeamSpeak3 Text to Speech](https://github.com/ReSpeak/ts3tts)
 - [TsPressor](https://github.com/ReSpeak/TsPressor)

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
