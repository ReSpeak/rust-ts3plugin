TS3Plugin API
=============

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
#![feature(box_raw)]
#[macro_use]
extern crate ts3plugin;

use ts3plugin::*;

struct MyTsPlugin;

impl Plugin for MyTsPlugin
{
    fn init(&mut self) -> InitResult
    {
        println!("Inited");
        InitResult::Success
    }

    fn shutdown(&mut self)
    {
        println!("Shutdown");
    }
}

create_plugin!("My Ts Plugin\0", "0.1.0\0", "My Name\0",
    "A wonderful tiny example plugin\0", MyTsPlugin);
```

License
-------
This project is licensed under the MIT license. The full license can be found in the `LICENSE` file.
