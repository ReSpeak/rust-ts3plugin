TS3Plugin API
=============
The documentation can be found [on GitHub-Pages](https://flakebi.github.io/rust-ts3plugin/doc/ts3plugin/).

Usage
-----
Add the following to your `Cargo.toml`:
```
[lib]
name = "<pluginname>"
crate-type = ["dylib"]

[dependencies.ts3plugin]
git = "https://github.com/Flakebi/rust-ts3plugin"
```

This code can be used to make your library a TeamSpeak plugin:
```
#[macro_use]
extern crate ts3plugin;

use ts3plugin::*;

struct MyTsPlugin;

impl Plugin for MyTsPlugin {
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

create_plugin!(
    "My Ts Plugin", "0.1.0", "My name", "A wonderful tiny example plugin",
    ConfigureOffer::No, false, MyTsPlugin);
```

License
-------
This project is licensed under the MIT license. The full license can be found in the `LICENSE` file.
