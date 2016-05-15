//! A crate to create TeamSpeak3 plugins.
//!
//! # Example
//!
//! A fully working example that creates a plugin that does nothing:
//!
//! ```
//! #[macro_use]
//! extern crate ts3plugin;
//!
//! use ts3plugin::*;
//!
//! struct MyTsPlugin;
//!
//! impl Plugin for MyTsPlugin {
//!     fn new(api: &TsApi) -> Result<Box<MyTsPlugin>, InitError> {
//!         api.log_or_print("Inited", "MyTsPlugin", LogLevel::Info);
//!         Ok(Box::new(MyTsPlugin))
//!         // Or return Err(InitError::Failure) on failure
//!     }
//!
//!     // Implement callbacks here
//!
//!     fn shutdown(&mut self, api: &TsApi) {
//!         api.log_or_print("Shutdown", "MyTsPlugin", LogLevel::Info);
//!     }
//! }
//!
//! create_plugin!(
//!     "My Ts Plugin", "0.1.0", "My name", "A wonderful tiny example plugin",
//!     ConfigureOffer::No, false, MyTsPlugin);
//!
//! # fn main() { }
//! ```

#![allow(dead_code)]
#![feature(macro_reexport)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate libc;
extern crate chrono;
#[macro_use]
#[macro_reexport(lazy_static)]
extern crate lazy_static;
extern crate ts3plugin_sys;

pub use ts3plugin_sys::clientlib_publicdefinitions::*;
pub use ts3plugin_sys::plugin_definitions::*;
pub use ts3plugin_sys::public_definitions::*;
pub use ts3plugin_sys::public_errors::Error;
pub use ts3plugin_sys::ts3functions::Ts3Functions;

pub use plugin::*;

use libc::size_t;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::mem::transmute;
use chrono::*;

/// Converts a normal string to a CString
macro_rules! to_cstring {
    ($string: expr) => {
        CString::new($string).unwrap_or(
            CString::new("String contains null character").unwrap())
    };
}

/// Converts a CString to a normal string
macro_rules! to_string {
    ($string: expr) => {
        String::from_utf8_lossy(CStr::from_ptr($string).to_bytes()).into_owned()
    };
}

// Declare modules here so the macros are visible in the modules
pub mod ts3interface;
pub mod plugin;

type Map<K, V> = BTreeMap<K, V>;

// Import automatically generated structs
include!(concat!(env!("OUT_DIR"), "/structs.rs"));

// ******************** Structs ********************
pub struct TsApi {
    servers: Map<ServerId, Server>,
}

pub struct Permissions;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ServerId(u64);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ChannelId(u64);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ConnectionId(u16);


// ******************** Implementation ********************

// ********** Server **********
impl PartialEq<Server> for Server {
    fn eq(&self, other: &Server) -> bool {
        self.id == other.id
    }
}
impl Eq for Server {}

impl Server {
    fn get_property_as_string(id: ServerId, property: VirtualServerProperties) -> Result<String, Error> {
        unsafe {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_server_variable_as_string)
                    (id.0, property as size_t, &mut name));
            match res {
                Error::Ok => Ok(to_string!(name)),
                _ => Err(res)
            }
        }
    }

    fn get_property_as_int(id: ServerId, property: VirtualServerProperties) -> Result<i32, Error> {
        unsafe {
            let mut number: c_int = 0;
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_server_variable_as_int)
                    (id.0, property as size_t, &mut number));
            match res {
                Error::Ok => Ok(number as i32),
                _ => Err(res)
            }
        }
    }

    fn add_connection(&mut self, connection_id: ConnectionId) -> Result<(), Error> {
        self.visible_connections.insert(connection_id, try!(Connection::new(self.id, connection_id)));
        Ok(())
    }

    pub fn get_connection(&self, connection_id: ConnectionId) -> Option<&Connection> {
        self.visible_connections.get(&connection_id)
    }

    pub fn get_mut_connection(&mut self, connection_id: ConnectionId) -> Option<&mut Connection> {
        self.visible_connections.get_mut(&connection_id)
    }
}

// ********** Channel **********
impl PartialEq<Channel> for Channel {
    fn eq(&self, other: &Channel) -> bool {
        self.server_id == other.server_id && self.id == other.id
    }
}
impl Eq for Channel {}

impl Channel {
    fn get_property_as_string(server_id: ServerId, id: ChannelId, property: ChannelProperties) -> Result<String, Error> {
        unsafe {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_channel_variable_as_string)
                    (server_id.0, id.0, property as size_t, &mut name));
            match res {
                Error::Ok => Ok(to_string!(name)),
                _ => Err(res)
            }
        }
    }

    fn get_property_as_int(server_id: ServerId, id: ChannelId, property: ChannelProperties) -> Result<i32, Error> {
        unsafe {
            let mut number: c_int = 0;
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_channel_variable_as_int)
                    (server_id.0, id.0, property as size_t, &mut number));
            match res {
                Error::Ok => Ok(number as i32),
                _ => Err(res)
            }
        }
    }

    fn get_property_as_uint64(server_id: ServerId, id: ChannelId, property: ChannelProperties) -> Result<i32, Error> {
        unsafe {
            let mut number: u64 = 0;
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_channel_variable_as_uint64)
                    (server_id.0, id.0, property as size_t, &mut number));
            match res {
                Error::Ok => Ok(number as i32),
                _ => Err(res)
            }
        }
    }
}

// ********** Connection **********
impl PartialEq<Connection> for Connection {
    fn eq(&self, other: &Connection) -> bool {
        self.server_id == other.server_id && self.id == other.id
    }
}
impl Eq for Connection {}

impl Connection {
    fn get_connection_property_as_string(server_id: ServerId, id: ConnectionId, property: ConnectionProperties) -> Result<String, Error> {
        unsafe {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_connection_variable_as_string)
                    (server_id.0, id.0, property as size_t, &mut name));
            match res {
                Error::Ok => Ok(to_string!(name)),
                _ => Err(res)
            }
        }
    }

    fn get_connection_property_as_uint64(server_id: ServerId, id: ConnectionId, property: ConnectionProperties) -> Result<u64, Error> {
        unsafe {
            let mut number: u64 = 0;
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_connection_variable_as_uint64)
                    (server_id.0, id.0, property as size_t, &mut number));
            match res {
                Error::Ok => Ok(number),
                _ => Err(res)
            }
        }
    }

    fn get_connection_property_as_double(server_id: ServerId, id: ConnectionId, property: ConnectionProperties) -> Result<f64, Error> {
        unsafe {
            let mut number: f64 = 0.0;
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_connection_variable_as_double)
                    (server_id.0, id.0, property as size_t, &mut number));
            match res {
                Error::Ok => Ok(number),
                _ => Err(res)
            }
        }
    }

    fn get_client_property_as_string(server_id: ServerId, id: ConnectionId, property: ClientProperties) -> Result<String, Error> {
        unsafe {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_client_variable_as_string)
                    (server_id.0, id.0, property as size_t, &mut name));
            match res {
                Error::Ok => Ok(to_string!(name)),
                _ => Err(res)
            }
        }
    }

    fn get_client_property_as_int(server_id: ServerId, id: ConnectionId, property: ClientProperties) -> Result<c_int, Error> {
        unsafe {
            let mut number: c_int = 0;
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").get_client_variable_as_int)
                    (server_id.0, id.0, property as size_t, &mut number));
            match res {
                Error::Ok => Ok(number),
                _ => Err(res)
            }
        }
    }
}


// ********** TsApi **********
/// The api functions provided by TeamSpeak
static mut ts3functions: Option<Ts3Functions> = None;

impl Default for TsApi {
    fn default() -> TsApi {
        TsApi {
            servers: Map::new(),
        }
    }
}

impl TsApi {
    /// Create a new TsApi instance without loading anything.
    /// This will be called from the `create_plugin!` macro.
    /// This function is not meant for public use.
    pub fn new() -> TsApi {
        Self::default()
    }

    /// Load all currently connected server and there data.
    /// This should normally be executed after `new()`
    /// This will be called from the `create_plugin!` macro.
    /// This function is not meant for public use.
    pub fn load(&mut self) -> Result<(), Error> {
        // Query available connections
        let mut result: *mut u64 = std::ptr::null_mut();
        let res: Error = unsafe { transmute((ts3functions.as_ref()
            .expect("Functions should be loaded").get_server_connection_handler_list)
                (&mut result)) };
        match res {
            Error::Ok => unsafe {
                let mut counter = 0;
                while *result.offset(counter) != 0 {
                    match self.add_server(ServerId(*result.offset(counter))) {
                        // Ignore tabs without connected servers
                        Err(Error::NotConnected) | Ok(_) => {},
                        Err(error) => self.log_or_print(format!(
                            "Can't load server: {:?}", error),
                            "rust-ts3plugin", LogLevel::Error),
                    }
                    counter += 1;
                }
            },
            _ => return Err(res)
        }
        Ok(())
    }

    /// Please try to use the member method `log_message` instead of this static method.
    pub fn static_log_message<S1: AsRef<str>, S2: AsRef<str>>(message: S1, channel: S2, severity: LogLevel) -> Result<(), Error> {
        unsafe {
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").log_message)
                    (to_cstring!(message.as_ref()).as_ptr(),
                    severity, to_cstring!(channel.as_ref()).as_ptr(), 0));
            match res {
                Error::Ok => Ok(()),
                _ => Err(res)
            }
        }
    }

    /// Please try to use the member method `log_or_print` instead of this static method.
    pub fn static_log_or_print<S1: AsRef<str>, S2: AsRef<str>>(message: S1, channel: S2, severity: LogLevel) {
        if let Err(error) = TsApi::static_log_message(message.as_ref(), channel.as_ref(), severity) {
            println!("Error {:?} while printing '{}' to '{}' ({:?})", error,
                message.as_ref(), channel.as_ref(), severity);
        }
    }

    // ********** Private Interface **********

    fn add_server(&mut self, server_id: ServerId) -> Result<(), Error> {
        self.servers.insert(server_id, try!(Server::new(server_id)));
        Ok(())
    }

    /// Returns true if a server was removed
    fn remove_server(&mut self, server_id: ServerId) -> bool {
        self.servers.remove(&server_id).is_some()
    }

    // ********** Public Interface **********

    /// Log a message using the TeamSpeak logging API.
    pub fn log_message<S1: AsRef<str>, S2: AsRef<str>>(&self, message: S1, channel: S2, severity: LogLevel) -> Result<(), Error> {
        TsApi::static_log_message(message, channel, severity)
    }

    /// Log a message using the TeamSpeak logging API.
    /// If that fails, print the message.
    pub fn log_or_print<S1: AsRef<str>, S2: AsRef<str>>(&self, message: S1, channel: S2, severity: LogLevel) {
        TsApi::static_log_or_print(message, channel, severity)
    }

    pub fn get_server(&self, server_id: ServerId) -> Option<&Server> {
        self.servers.get(&server_id)
    }

    pub fn get_mut_server(&mut self, server_id: ServerId) -> Option<&mut Server> {
        self.servers.get_mut(&server_id)
    }
}
