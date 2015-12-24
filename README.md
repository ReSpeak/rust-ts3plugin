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

impl MyTsPlugin
{
    fn new() -> MyTsPlugin
    {
        MyTsPlugin
    }
}

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
    "A wonderful tiny example plugin\0", ConfigureOffer::No, MyTsPlugin);
```

License
-------
This project is licensed under the MIT license. The full license can be found in the `LICENSE` file.
