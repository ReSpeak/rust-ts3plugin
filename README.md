TeamSpeak3 Plugin API &emsp; [![Build Status](https://travis-ci.org/Flakebi/rust-ts3plugin.svg?branch=master)](https://travis-ci.org/Flakebi/rust-ts3plugin) [![Latest version](https://img.shields.io/crates/v/ts3plugin.svg)](https://crates.io/crates/ts3plugin)
=====================
The documentation can be found here: [![At docs.rs](https://docs.rs/ts3plugin/badge.svg)](https://docs.rs/ts3plugin)

TeamSpeak 3.1 updates the plugin api version from 20 to 21.  
Version 0.2 and above are compatible with this version while version 0.1 is
compatible with the api version 20.

Breaking changes will happen from time to time, leading to a minor version bump.

At the moment, not all methods that are exposed by the TeamSpeak API are
available for plugins. If a method that you need is missing, please file an
issue or open a pull request.

Usage
-----
Add the following to your `Cargo.toml`:
```toml
[lib]
name = "<pluginname>"
crate-type = ["cdylib"]

[dependencies]
lazy_static = "0.2"
ts3plugin = "0.2"
```

This code can be used to make your library a TeamSpeak plugin:
```rust,no-run
#[macro_use]
extern crate ts3plugin;
#[macro_use]
extern crate lazy_static;

use ts3plugin::*;

struct MyTsPlugin;

impl Plugin for MyTsPlugin {
    fn new(api: &mut TsApi) -> Result<Box<MyTsPlugin>, InitError> {
        api.log_or_print("Inited", "MyTsPlugin", LogLevel::Info);
        Ok(Box::new(MyTsPlugin))
        // Or return Err(InitError::Failure) on failure
    }

    // Implement callbacks here

    fn shutdown(&mut self, api: &mut TsApi) {
        api.log_or_print("Shutdown", "MyTsPlugin", LogLevel::Info);
    }
}

create_plugin!(
    "My Ts Plugin", "0.1.0", "My name", "A wonderful tiny example plugin",
    ConfigureOffer::No, false, MyTsPlugin);
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


Template code that is needed to run the rust code in this file as a test:

```rust,skeptic-template
{}
fn main(){{}}
```
