//! TeamSpeak 3.6 updates the plugin api version to 26.  
//! Version 0.3 is compatible with this version.
//!
//! At the moment, not all methods that are exposed by the TeamSpeak API are
//! available for plugins. If a method that you need is missing, please file an
//! issue or open a pull request.
//!
//! # Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [package]
//! name = "<pluginname>"
//! version = "<version>"
//! authors = ["<your name>"]
//! description = "<description>"
//!
//! [lib]
//! name = "<pluginname>"
//! crate-type = ["cdylib"]
//!
//! [dependencies]
//! ts3plugin = "0.3"
//! ```
//!
//! # Example
//!
//! A fully working example, which creates a plugin that does nothing:
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
//!     // The default name is the crate name, but we can overwrite it here.
//!     fn name()        -> String { String::from("My Ts Plugin") }
//!     fn command() -> Option<String> { Some(String::from("myplugin")) }
//!     fn autoload() -> bool { false }
//!     fn configurable() -> ConfigureOffer { ConfigureOffer::No }
//!
//!     // The only required method
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
//! create_plugin!(MyTsPlugin);
//!
//! # fn main() { }
//! ```

// TODO This should be removed at some time, when more code is ready
#![allow(dead_code)]

extern crate chrono;
#[macro_use]
extern crate lazy_static;
extern crate ts3plugin_sys;

pub use ts3plugin_sys::plugin_definitions::*;
pub use ts3plugin_sys::public_definitions::*;
pub use ts3plugin_sys::public_errors::Error;
pub use ts3plugin_sys::ts3functions::Ts3Functions;

pub use crate::plugin::*;

use chrono::*;
use std::collections::HashMap as Map;
use std::ffi::{CStr, CString};
use std::fmt;
use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_char, c_int};
use std::sync::{MutexGuard, RwLock};

/// Converts a normal `String` to a `CString`.
macro_rules! to_cstring {
	($string: expr_2021) => {
		CString::new($string).unwrap_or(CString::new("String contains null character").unwrap())
	};
}

/// Converts a `CString` to a normal `String`.
macro_rules! to_string {
	($string: expr_2021) => {{ String::from_utf8_lossy(CStr::from_ptr($string).to_bytes()).into_owned() }};
}

// Declare modules here so the macros are visible in the modules
pub mod plugin;
pub mod ts3interface;

// Import automatically generated structs
include!(concat!(env!("OUT_DIR"), "/channel.rs"));
include!(concat!(env!("OUT_DIR"), "/connection.rs"));
include!(concat!(env!("OUT_DIR"), "/server.rs"));

/// The api functions provided by TeamSpeak
///
/// This is not part of the official api and is only public to permit dirty
/// hacks!
#[doc(hidden)]
pub static TS3_FUNCTIONS: RwLock<Option<Ts3Functions>> = RwLock::new(None);

// ******************** Structs ********************
/// The possible receivers of a message. A message can be sent to a specific
/// connection, to the current channel chat or to the server chat.
#[derive(Clone)]
pub enum MessageReceiver {
	Connection(ConnectionId),
	Channel,
	Server,
}

/// Permissions - TODO not yet implemented
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Permissions;

/// A wrapper for a server id.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ServerId(u64);

/// A wrapper for a channel id.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ChannelId(u64);

/// A wrapper for a connection id.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ConnectionId(u16);

#[derive(Debug, Clone)]
pub struct Permission {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct PermissionId(u32);

#[derive(Debug, Clone)]
pub struct ServerGroup {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ServerGroupId(u64);

#[derive(Debug, Clone)]
pub struct ChannelGroup {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ChannelGroupId(u64);

// ******************** Implementation ********************

// ********** Invoker **********
#[derive(Debug, Eq)]
pub struct InvokerData {
	id: ConnectionId,
	uid: String,
	name: String,
}

impl PartialEq<InvokerData> for InvokerData {
	fn eq(&self, other: &InvokerData) -> bool {
		self.id == other.id
	}
}

impl InvokerData {
	fn new(id: ConnectionId, uid: String, name: String) -> InvokerData {
		InvokerData { id, uid, name }
	}

	/// Get the connection id of this invoker.
	pub fn get_id(&self) -> ConnectionId {
		self.id
	}

	/// Get the unique id of this invoker.
	pub fn get_uid(&self) -> &String {
		&self.uid
	}

	/// Get the name of this invoker.
	pub fn get_name(&self) -> &String {
		&self.name
	}
}

/// The invoker is maybe not visible to the user, but we can get events caused
/// by him, so some information about him are passed along with his id.
#[derive(Debug, Eq)]
pub struct Invoker<'a> {
	server: Server<'a>,
	data: InvokerData,
}

impl<'a, 'b> PartialEq<Invoker<'b>> for Invoker<'a> {
	fn eq(&self, other: &Invoker) -> bool {
		self.server == other.server && self.data == other.data
	}
}
impl<'a> Deref for Invoker<'a> {
	type Target = InvokerData;
	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

impl<'a> Invoker<'a> {
	fn new(server: Server<'a>, data: InvokerData) -> Invoker<'a> {
		Invoker { server, data }
	}

	pub fn get_connection(&'_ self) -> Option<Connection<'_>> {
		self.server.get_connection(self.id)
	}
}

// ********** Server **********
#[derive(Clone)]
pub struct Server<'a> {
	api: &'a TsApi,
	data: Result<&'a ServerData, ServerId>,
}

impl<'a, 'b> PartialEq<Server<'b>> for Server<'a> {
	fn eq(&self, other: &Server<'b>) -> bool {
		self.get_id() == other.get_id()
	}
}
impl<'a> Eq for Server<'a> {}
impl<'a> fmt::Debug for Server<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Server({})", self.get_id().0)
	}
}

impl PartialEq<ServerData> for ServerData {
	fn eq(&self, other: &ServerData) -> bool {
		self.id == other.id
	}
}
impl Eq for ServerData {}

impl ServerData {
	/// Get a server property that is stored as a string.
	fn get_property_as_string(
		id: ServerId, property: VirtualServerProperties,
	) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error =
				transmute((TS3_FUNCTIONS
					.read()
					.unwrap()
					.as_ref()
					.expect("Functions should be loaded")
					.get_server_variable_as_string)(id.0, property as usize, &mut name));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res),
			}
		}
	}

	/// Get a server property that is stored as an int.
	fn get_property_as_int(id: ServerId, property: VirtualServerProperties) -> Result<i32, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error =
				transmute((TS3_FUNCTIONS
					.read()
					.unwrap()
					.as_ref()
					.expect("Functions should be loaded")
					.get_server_variable_as_int)(id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number as i32),
				_ => Err(res),
			}
		}
	}

	/// Get a server property that is stored as an int.
	fn get_property_as_uint64(
		id: ServerId, property: VirtualServerProperties,
	) -> Result<u64, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_server_variable_as_uint64)(
				id.0, property as usize, &mut number
			));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res),
			}
		}
	}

	/// Get the connection id of our own client.
	/// Called when a new Server is created.
	fn query_own_connection_id(id: ServerId) -> Result<ConnectionId, Error> {
		unsafe {
			let mut number: u16 = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_client_id)(id.0, &mut number));
			match res {
				Error::Ok => Ok(ConnectionId(number)),
				_ => Err(res),
			}
		}
	}

	/// Get all currently active connections on this server.
	/// Called when a new Server is created.
	/// When an error occurs, users are not inserted into the map.
	fn query_connections(id: ServerId) -> Map<ConnectionId, ConnectionData> {
		let mut map = Map::new();
		// Query connected connections
		let mut result: *mut u16 = std::ptr::null_mut();
		let res: Error = unsafe {
			transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_client_list)(id.0, &mut result))
		};
		if res == Error::Ok {
			unsafe {
				let mut counter = 0;
				while *result.offset(counter) != 0 {
					let connection_id = ConnectionId(*result.offset(counter));
					let mut connection = ConnectionData::new(id, connection_id);
					connection.update();
					map.insert(connection_id, connection);
					counter += 1;
				}
			}
		}
		map
	}

	/// Get all channels on this server.
	/// Called when a new Server is created.
	/// When an error occurs, channels are not inserted into the map.
	fn query_channels(id: ServerId) -> Result<Map<ChannelId, ChannelData>, Error> {
		let mut map = Map::new();
		// Query connected connections
		let mut result: *mut u64 = std::ptr::null_mut();
		let res: Error = unsafe {
			transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_channel_list)(id.0, &mut result))
		};
		if res == Error::Ok {
			unsafe {
				let mut counter = 0;
				while *result.offset(counter) != 0 {
					let channel_id = ChannelId(*result.offset(counter));
					let mut channel = ChannelData::new(id, channel_id);
					channel.update();
					map.insert(channel_id, channel);
					counter += 1;
				}
			}
			Ok(map)
		} else {
			Err(res)
		}
	}

	// ********** Private Interface **********

	fn add_connection(&mut self, connection_id: ConnectionId) -> &mut ConnectionData {
		let mut connection = ConnectionData::new(self.id, connection_id);
		connection.update();
		self.visible_connections.insert(connection_id, connection);
		self.visible_connections.get_mut(&connection_id).unwrap()
	}

	fn remove_connection(&mut self, connection_id: ConnectionId) -> Option<ConnectionData> {
		self.visible_connections.remove(&connection_id)
	}

	fn add_channel(&mut self, channel_id: ChannelId) -> Result<&mut ChannelData, Error> {
		match self.channels {
			Ok(ref mut cs) => {
				let mut channel = ChannelData::new(self.id, channel_id);
				channel.update();
				cs.insert(channel_id, channel);
				Ok(cs.get_mut(&channel_id).unwrap())
			}
			Err(error) => Err(error),
		}
	}

	fn remove_channel(&mut self, channel_id: ChannelId) -> Option<ChannelData> {
		self.channels.as_mut().ok().and_then(|cs| cs.remove(&channel_id))
	}

	/// Get the mutable connection on this server that has the specified id, returns
	/// `None` if there is no such connection.
	fn get_mut_connection(&mut self, connection_id: ConnectionId) -> Option<&mut ConnectionData> {
		self.visible_connections.get_mut(&connection_id)
	}

	/// Get the mutable channel on this server that has the specified id, returns
	/// `None` if there is no such channel.
	fn get_mut_channel(&mut self, channel_id: ChannelId) -> Option<&mut ChannelData> {
		self.channels.as_mut().ok().and_then(|cs| cs.get_mut(&channel_id))
	}
}

impl<'a> Server<'a> {
	fn new(api: &'a TsApi, data: &'a ServerData) -> Server<'a> {
		Server { api, data: Ok(data) }
	}

	fn new_err(api: &'a TsApi, server_id: ServerId) -> Server<'a> {
		Server { api, data: Err(server_id) }
	}

	pub fn get_id(&self) -> ServerId {
		match self.data {
			Ok(data) => data.get_id(),
			Err(id) => id,
		}
	}

	/// Get the connection on this server that has the specified id, returns
	/// `None` if there is no such connection.
	fn get_connection_unwrap(&self, connection_id: ConnectionId) -> Connection<'a> {
		self.get_connection(connection_id).unwrap_or_else(|| {
			self.api.log_or_print(
				format!("Can't find connection {:?}", connection_id),
				"rust-ts3plugin",
				crate::LogLevel::Warning,
			);
			Connection::new_err(&self.api, self.get_id(), connection_id)
		})
	}

	/// Get the channel on this server that has the specified id, returns
	/// `None` if there is no such channel.
	fn get_channel_unwrap(&self, channel_id: ChannelId) -> Channel<'a> {
		self.get_channel(channel_id).unwrap_or_else(|| {
			self.api.log_or_print(
				format!("Can't find channel {:?}", channel_id),
				"rust-ts3plugin",
				crate::LogLevel::Warning,
			);
			Channel::new_owned(&self.api, self.get_id(), channel_id)
		})
	}

	fn get_server_group_unwrap(&self, server_group_id: ServerGroupId) -> ServerGroup {
		self.get_server_group(server_group_id).unwrap_or_else(|| {
			/*self.api.log_or_print(
			format!("Can't find server group {:?}", server_group_id),
			"rust-ts3plugin", ::LogLevel::Warning);*/
			ServerGroup {}
		})
	}

	fn get_channel_group_unwrap(&self, channel_group_id: ChannelGroupId) -> ChannelGroup {
		self.get_channel_group(channel_group_id).unwrap_or_else(|| {
			//self.api.log_or_print(format!("Can't find channel group {:?}", channel_group_id),
			// "rust-ts3plugin", ::LogLevel::Warning);
			ChannelGroup {}
		})
	}

	// ********** Public Interface **********

	/*/// The server properties that are only available on request.
	pub fn get_optional_data(&self) -> Option<&OptionalServerData> {
		self.data.ok().map(|data| &data.optional_data)
	}*/

	/// Get the own connection to the server.
	pub fn get_own_connection(&self) -> Result<Connection<'a>, Error> {
		match self.data {
			Ok(data) => data.get_own_connection_id().map(|id| self.get_connection_unwrap(id)),
			Err(_) => Err(Error::Ok),
		}
	}

	/// Get the ids of all visible connections on this server.
	pub fn get_connections(&self) -> Vec<Connection<'a>> {
		match self.data {
			Ok(data) => {
				data.visible_connections.values().map(|c| Connection::new(self.api, &c)).collect()
			}
			Err(_) => Vec::new(),
		}
	}

	/// Get the ids of all channels on this server.
	pub fn get_channels(&self) -> Vec<Channel<'a>> {
		match self.data {
			Ok(data) => match data.channels {
				Ok(ref cs) => cs.values().map(|c| Channel::new(self.api, &c)).collect(),
				Err(_) => Vec::new(),
			},
			Err(_) => Vec::new(),
		}
	}

	/// Get the connection on this server that has the specified id, returns
	/// `None` if there is no such connection.
	pub fn get_connection(&self, connection_id: ConnectionId) -> Option<Connection<'a>> {
		self.data.ok().and_then(|data| {
			data.visible_connections.get(&connection_id).map(|c| Connection::new(&self.api, c))
		})
	}

	/// Get the channel on this server that has the specified id, returns
	/// `None` if there is no such channel.
	pub fn get_channel(&self, channel_id: ChannelId) -> Option<Channel<'a>> {
		self.data.ok().and_then(|data| {
			data.channels
				.as_ref()
				.ok()
				.and_then(|cs| cs.get(&channel_id))
				.map(|c| Channel::new(&self.api, c))
		})
	}

	pub fn get_server_group(&self, _server_group_id: ServerGroupId) -> Option<ServerGroup> {
		// TODO
		Some(ServerGroup {})
	}

	pub fn get_channel_group(&self, _channel_group_id: ChannelGroupId) -> Option<ChannelGroup> {
		// TODO
		Some(ChannelGroup {})
	}

	/// Send a message to the server chat.
	pub fn send_message<S: AsRef<str>>(&self, message: S) -> Result<(), Error> {
		unsafe {
			let text = to_cstring!(message.as_ref());
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.request_send_server_text_msg)(
				self.get_id().0, text.as_ptr(), std::ptr::null()
			));
			match res {
				Error::Ok => Ok(()),
				_ => Err(res),
			}
		}
	}

	/// Sends a plugin message to all connections on the server.
	///
	/// Messages can be received in [`Plugin::plugin_message`].
	/// This is refered to as `PluginCommand` in TeamSpeak.
	///
	/// [`Plugin::plugin_message`]: plugin/trait.Plugin.html#method.plugin_message
	pub fn send_plugin_message<S: AsRef<str>>(&self, message: S) {
		let text = to_cstring!(message.as_ref());
		(TS3_FUNCTIONS
			.read()
			.unwrap()
			.as_ref()
			.expect("Functions should be loaded")
			.send_plugin_command)(
			self.get_id().0,
			to_cstring!(self.api.get_plugin_id()).as_ptr(),
			text.as_ptr(),
			PluginTargetMode::Server as i32,
			std::ptr::null(),
			std::ptr::null(),
		);
	}

	/// Print a message into the server or channel tab of this server. This is only
	/// visible in the window of this client and will not be sent to the server.
	pub fn print_message<S: AsRef<str>>(&self, message: S, target: MessageTarget) {
		let text = to_cstring!(message.as_ref());
		(TS3_FUNCTIONS.read().unwrap().as_ref().expect("Functions should be loaded").print_message)(
			self.get_id().0,
			text.as_ptr(),
			target,
		);
	}
}

// ********** Channel **********
#[derive(Clone)]
pub struct Channel<'a> {
	api: &'a TsApi,
	data: Result<&'a ChannelData, (ServerId, ChannelId)>,
}

impl<'a, 'b> PartialEq<Channel<'b>> for Channel<'a> {
	fn eq(&self, other: &Channel<'b>) -> bool {
		self.get_server_id() == other.get_server_id() && self.get_id() == other.get_id()
	}
}
impl<'a> Eq for Channel<'a> {}
impl<'a> fmt::Debug for Channel<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Channel({})", self.get_id().0)
	}
}

impl PartialEq<ChannelData> for ChannelData {
	fn eq(&self, other: &ChannelData) -> bool {
		self.server_id == other.server_id && self.id == other.id
	}
}
impl Eq for ChannelData {}

impl ChannelData {
	/// Get a channel property that is stored as a string.
	fn get_property_as_string(
		server_id: ServerId, id: ChannelId, property: ChannelProperties,
	) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_channel_variable_as_string)(
				server_id.0, id.0, property as usize, &mut name
			));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res),
			}
		}
	}

	/// Get a channel property that is stored as an int.
	fn get_property_as_int(
		server_id: ServerId, id: ChannelId, property: ChannelProperties,
	) -> Result<i32, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_channel_variable_as_int)(
				server_id.0, id.0, property as usize, &mut number
			));
			match res {
				Error::Ok => Ok(number as i32),
				_ => Err(res),
			}
		}
	}

	/// Get a channel property that is stored as an uint64.
	fn get_property_as_uint64(
		server_id: ServerId, id: ChannelId, property: ChannelProperties,
	) -> Result<i32, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_channel_variable_as_uint64)(
				server_id.0, id.0, property as usize, &mut number
			));
			match res {
				Error::Ok => Ok(number as i32),
				_ => Err(res),
			}
		}
	}

	/// Ask the TeamSpeak api about the parent channel id of a channel.
	fn query_parent_channel_id(server_id: ServerId, id: ChannelId) -> Result<ChannelId, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error =
				transmute((TS3_FUNCTIONS
					.read()
					.unwrap()
					.as_ref()
					.expect("Functions should be loaded")
					.get_parent_channel_of_channel)(server_id.0, id.0, &mut number));
			match res {
				Error::Ok => Ok(ChannelId(number)),
				_ => Err(res),
			}
		}
	}
}

impl<'a> Channel<'a> {
	fn new(api: &'a TsApi, data: &'a ChannelData) -> Channel<'a> {
		Channel { api, data: Ok(data) }
	}

	fn new_owned(api: &'a TsApi, server_id: ServerId, channel_id: ChannelId) -> Channel<'a> {
		Channel { api, data: Err((server_id, channel_id)) }
	}

	fn get_server_id(&self) -> ServerId {
		match self.data {
			Ok(data) => data.get_server_id(),
			Err((server_id, _)) => server_id,
		}
	}

	pub fn get_id(&self) -> ChannelId {
		match self.data {
			Ok(data) => data.get_id(),
			Err((_, channel_id)) => channel_id,
		}
	}

	/// Get the server of this channel.
	pub fn get_server(&self) -> Server<'a> {
		self.api.get_server_unwrap(self.get_server_id())
	}

	pub fn get_parent_channel(&self) -> Result<Option<Channel<'a>>, Error> {
		match self.data {
			Ok(data) => data.get_parent_channel_id().map(|parent_channel_id| {
				if parent_channel_id.0 == 0 {
					None
				} else {
					Some(self.get_server().get_channel_unwrap(parent_channel_id))
				}
			}),
			Err(_) => Err(Error::Ok),
		}
	}

	/// Send a message to this channel chat.
	pub fn send_message<S: AsRef<str>>(&self, message: S) -> Result<(), Error> {
		unsafe {
			let text = to_cstring!(message.as_ref());
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.request_send_channel_text_msg)(
				self.data.unwrap().server_id.0,
				text.as_ptr(),
				self.data.unwrap().id.0,
				std::ptr::null(),
			));
			match res {
				Error::Ok => Ok(()),
				_ => Err(res),
			}
		}
	}
}

// ********** Connection **********
#[derive(Clone)]
pub struct Connection<'a> {
	api: &'a TsApi,
	data: Result<&'a ConnectionData, (ServerId, ConnectionId)>,
}

impl<'a, 'b> PartialEq<Connection<'b>> for Connection<'a> {
	fn eq(&self, other: &Connection<'b>) -> bool {
		self.get_server_id() == other.get_server_id() && self.get_id() == other.get_id()
	}
}
impl<'a> Eq for Connection<'a> {}
impl<'a> fmt::Debug for Connection<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Connection({})", self.get_id().0)
	}
}

impl PartialEq<ConnectionData> for ConnectionData {
	fn eq(&self, other: &ConnectionData) -> bool {
		self.server_id == other.server_id && self.id == other.id
	}
}
impl Eq for ConnectionData {}

impl ConnectionData {
	/// Get a connection property that is stored as a string.
	fn get_connection_property_as_string(
		server_id: ServerId, id: ConnectionId, property: ConnectionProperties,
	) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_connection_variable_as_string)(
				server_id.0, id.0, property as usize, &mut name
			));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res),
			}
		}
	}

	/// Get a connection property that is stored as a uint64.
	fn get_connection_property_as_uint64(
		server_id: ServerId, id: ConnectionId, property: ConnectionProperties,
	) -> Result<u64, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_connection_variable_as_uint64)(
				server_id.0, id.0, property as usize, &mut number
			));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res),
			}
		}
	}

	/// Get a connection property that is stored as a double.
	fn get_connection_property_as_double(
		server_id: ServerId, id: ConnectionId, property: ConnectionProperties,
	) -> Result<f64, Error> {
		unsafe {
			let mut number: f64 = 0.0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_connection_variable_as_double)(
				server_id.0, id.0, property as usize, &mut number
			));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res),
			}
		}
	}

	/// Get a client property that is stored as a string.
	fn get_client_property_as_string(
		server_id: ServerId, id: ConnectionId, property: ClientProperties,
	) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_client_variable_as_string)(
				server_id.0, id.0, property as usize, &mut name
			));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res),
			}
		}
	}

	/// Get a client property that is stored as an int.
	fn get_client_property_as_int(
		server_id: ServerId, id: ConnectionId, property: ClientProperties,
	) -> Result<c_int, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_client_variable_as_int)(
				server_id.0, id.0, property as usize, &mut number
			));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res),
			}
		}
	}

	/// Ask the TeamSpeak api about the current channel id of a connection.
	fn query_channel_id(server_id: ServerId, id: ConnectionId) -> Result<ChannelId, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_channel_of_client)(server_id.0, id.0, &mut number));
			match res {
				Error::Ok => Ok(ChannelId(number)),
				_ => Err(res),
			}
		}
	}

	/// Ask the TeamSpeak api, if the specified connection is currently whispering to our own
	/// client.
	fn query_whispering(server_id: ServerId, id: ConnectionId) -> Result<bool, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.is_whispering)(server_id.0, id.0, &mut number));
			match res {
				Error::Ok => Ok(number != 0),
				_ => Err(res),
			}
		}
	}
}

impl<'a> Connection<'a> {
	fn new(api: &'a TsApi, data: &'a ConnectionData) -> Connection<'a> {
		Connection { api, data: Ok(data) }
	}

	fn new_err(api: &'a TsApi, server_id: ServerId, connection_id: ConnectionId) -> Connection<'a> {
		Connection { api, data: Err((server_id, connection_id)) }
	}

	fn get_server_id(&self) -> ServerId {
		match self.data {
			Ok(data) => data.get_server_id(),
			Err((server_id, _)) => server_id,
		}
	}

	pub fn get_id(&self) -> ConnectionId {
		match self.data {
			Ok(data) => data.get_id(),
			Err((_, connection_id)) => connection_id,
		}
	}

	/// Get the server of this connection.
	pub fn get_server(&self) -> Server<'a> {
		self.api.get_server_unwrap(self.get_server_id())
	}

	/// Get the channel of this connection.
	pub fn get_channel(&self) -> Result<Channel<'a>, Error> {
		match self.data {
			Ok(data) => data.get_channel_id().map(|c| self.get_server().get_channel_unwrap(c)),
			Err(_) => Err(Error::Ok),
		}
	}

	pub fn get_channel_group_inherited_channel(&self) -> Result<Channel<'a>, Error> {
		match self.data {
			Ok(data) => data
				.get_channel_group_inherited_channel_id()
				.map(|c| self.get_server().get_channel_unwrap(c)),
			Err(_) => Err(Error::Ok),
		}
	}

	/*/// The connection properties that are only available for our own client.
	pub fn get_own_data(&self) -> Option<&OwnConnectionData> {
		self.data.ok().and_then(|data| data.own_data.as_ref())
	}

	/// The connection properties that are only available for server queries.
	pub fn get_serverquery_data(&self) -> Option<&ServerqueryConnectionData> {
		self.data.ok().and_then(|data| data.serverquery_data.as_ref())
	}

	/// The connection properties that are only available on request.
	pub fn get_optional_data(&self) -> Option<&OptionalConnectionData> {
		self.data.ok().map(|data| &data.optional_data)
	}*/

	/// Send a private message to this connection.
	pub fn send_message<S: AsRef<str>>(&self, message: S) -> Result<(), Error> {
		unsafe {
			let text = to_cstring!(message.as_ref());
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.request_send_private_text_msg)(
				self.data.unwrap().server_id.0,
				text.as_ptr(),
				self.data.unwrap().id.0,
				std::ptr::null(),
			));
			match res {
				Error::Ok => Ok(()),
				_ => Err(res),
			}
		}
	}
}

pub struct TsApiLock {
	guard: MutexGuard<'static, (Option<(TsApi, Box<dyn Plugin>)>, Option<String>)>,
}
impl Deref for TsApiLock {
	type Target = TsApi;
	fn deref(&self) -> &Self::Target {
		&self.guard.0.as_ref().unwrap().0
	}
}
impl DerefMut for TsApiLock {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.guard.0.as_mut().unwrap().0
	}
}

pub struct PluginLock {
	guard: MutexGuard<'static, (Option<(TsApi, Box<dyn Plugin>)>, Option<String>)>,
}
impl Deref for PluginLock {
	type Target = dyn Plugin;
	fn deref(&self) -> &Self::Target {
		&*self.guard.0.as_ref().unwrap().1
	}
}
impl DerefMut for PluginLock {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut *self.guard.0.as_mut().unwrap().1
	}
}

// ********** TsApi **********
/// The main struct that contains all permanently save data.
pub struct TsApi {
	/// All known servers.
	servers: Map<ServerId, ServerData>,
	/// The plugin id from TeamSpeak.
	plugin_id: String,
}

// Don't provide a default Implementation because we don't want the TsApi
// to be publicly constructable.
impl TsApi {
	/// Create a new TsApi instance without loading anything.
	fn new(plugin_id: String) -> TsApi {
		TsApi { servers: Map::new(), plugin_id: plugin_id }
	}

	/// Load all currently connected server and their data.
	/// This should normally be executed after `new()`.
	fn load(&mut self) -> Result<(), Error> {
		// Query available connections
		let mut result: *mut u64 = std::ptr::null_mut();
		let res: Error = unsafe {
			transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_server_connection_handler_list)(&mut result))
		};
		match res {
			Error::Ok => unsafe {
				let mut counter = 0;
				while *result.offset(counter) != 0 {
					// Test if we have a connection to this server.
					// We get open tabs, even if they are disconnected.
					let mut status: c_int = 0;
					let res: Error = transmute((TS3_FUNCTIONS
						.read()
						.unwrap()
						.as_ref()
						.expect("Functions should be loaded")
						.get_connection_status)(
						*result.offset(counter), &mut status
					));
					if res == Error::Ok
						&& transmute::<c_int, ConnectStatus>(status) != ConnectStatus::Disconnected
					{
						self.add_server(ServerId(*result.offset(counter)));
					}
					counter += 1;
				}
			},
			_ => return Err(res),
		}
		Ok(())
	}

	/// Lock the global `TsApi` object. This will be `None` when the plugin is
	/// constructed.
	pub fn lock_api() -> Option<TsApiLock> {
		let guard = ts3interface::DATA.lock().unwrap();
		if guard.0.is_none() { None } else { Some(TsApiLock { guard }) }
	}

	/// Lock the global `Plugin` object.
	pub fn lock_plugin() -> Option<PluginLock> {
		let guard = ts3interface::DATA.lock().unwrap();
		if guard.0.is_none() { None } else { Some(PluginLock { guard }) }
	}

	/// Please try to use the member method `log_message` instead of this static method.
	pub fn static_log_message<S1: AsRef<str>, S2: AsRef<str>>(
		message: S1, channel: S2, severity: LogLevel,
	) -> Result<(), Error> {
		unsafe {
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.log_message)(
				to_cstring!(message.as_ref()).as_ptr(),
				severity,
				to_cstring!(channel.as_ref()).as_ptr(),
				0,
			));
			match res {
				Error::Ok => Ok(()),
				_ => Err(res),
			}
		}
	}

	/// Please try to use the member method `log_or_print` instead of this static method.
	pub fn static_log_or_print<S1: AsRef<str>, S2: AsRef<str>>(
		message: S1, channel: S2, severity: LogLevel,
	) {
		if let Err(error) = TsApi::static_log_message(message.as_ref(), channel.as_ref(), severity)
		{
			println!(
				"Error {:?} while printing '{}' to '{}' ({:?})",
				error,
				message.as_ref(),
				channel.as_ref(),
				severity
			);
		}
	}

	/// Please try to use the member method `get_error_message` instead of this static method.
	pub fn static_get_error_message(error: Error) -> Result<String, Error> {
		unsafe {
			let mut message: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_error_message)(error as u32, &mut message));
			match res {
				Error::Ok => Ok(to_string!(message)),
				_ => Err(res),
			}
		}
	}

	// ********** Private Interface **********

	/// Add the server with the specified id to the server list.
	/// The currently available data of this server will be stored.
	fn add_server(&mut self, server_id: ServerId) -> &mut ServerData {
		self.servers.insert(server_id, ServerData::new(server_id));
		let server = self.servers.get_mut(&server_id).unwrap();
		server.update();
		server
	}

	/// Returns true if a server was removed
	fn remove_server(&mut self, server_id: ServerId) -> Option<ServerData> {
		self.servers.remove(&server_id)
	}

	/// Get the plugin id assigned by TeamSpeak.
	pub fn get_plugin_id(&self) -> &str {
		&self.plugin_id
	}

	/// Update the data of a connection with the data from the same connection
	/// as an invoker if possible.
	fn try_update_invoker(&mut self, server_id: ServerId, invoker: &InvokerData) {
		if let Some(server) = self.get_mut_server(server_id) {
			if let Some(connection) = server.get_mut_connection(invoker.get_id()) {
				if connection.get_uid() != Ok(invoker.get_uid()) {
					connection.uid = Ok(invoker.get_uid().clone());
				}
				if connection.get_name() != Ok(invoker.get_name()) {
					connection.name = Ok(invoker.get_name().clone())
				}
			}
		}
	}

	/// A reusable function that takes a TeamSpeak3 api function like
	/// `get_plugin_path` and returns the path.
	/// The buffer that holds the path will be automatically enlarged up to a
	/// limit.
	/// The function that is colled takes a pointer to a string buffer that will
	/// be filled and the max lenght of the buffer.
	fn get_path<F: Fn(*mut c_char, usize)>(fun: F) -> String {
		const START_SIZE: usize = 512;
		const MAX_SIZE: usize = 100_000;
		let mut size = START_SIZE;
		loop {
			let mut buf = vec![0 as u8; size];
			fun(buf.as_mut_ptr() as *mut c_char, size - 1);
			// Test if the allocated buffer was long enough
			if buf[size - 3] != 0 {
				size *= 2;
				if size > MAX_SIZE {
					return String::new();
				}
			} else {
				// Be sure that the string is terminated
				buf[size - 1] = 0;
				let s = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
				let result = s.to_string_lossy();
				return result.into_owned();
			}
		}
	}

	/// Get the mutable server that has the specified id, returns `None` if there is no
	/// such server.
	fn get_mut_server(&mut self, server_id: ServerId) -> Option<&mut ServerData> {
		self.servers.get_mut(&server_id)
	}

	fn get_server_unwrap<'a>(&'a self, server_id: ServerId) -> Server<'a> {
		self.servers.get(&server_id).map(|s| Server::<'a>::new(&self, s)).unwrap_or_else(|| {
			// Ignore here, there are too many messages when we are not yet
			// fully connected (or already disconnected), but sound is sent.
			// self.log_or_print(format!("Can't find server {:?}\n{:?}",
			//     server_id, backtrace::Backtrace::new()), "rust-ts3plugin", ::LogLevel::Warning);
			Server::new_err(&self, server_id)
		})
	}

	// ********** Public Interface **********

	/// Get the raw TeamSpeak api functions.
	/// These functions can be used to invoke actions that are not yet
	/// implemented by this library. You should file a bug report or make a pull
	/// request if you need to use this function.
	//pub unsafe fn get_raw_api() -> &'static Ts3Functions { unsafe { TS3_FUNCTIONS.lock().unwrap().as_ref().unwrap() }}

	/// Get all servers to which this client is currently connected.
	pub fn get_servers<'a>(&'a self) -> Vec<Server<'a>> {
		self.servers.values().map(|s| Server::new(&self, &s)).collect()
	}

	/// Log a message using the TeamSpeak logging API.
	pub fn log_message<S1: AsRef<str>, S2: AsRef<str>>(
		&self, message: S1, channel: S2, severity: LogLevel,
	) -> Result<(), Error> {
		TsApi::static_log_message(message, channel, severity)
	}

	/// Log a message using the TeamSpeak logging API.
	/// If that fails, print the message to stdout.
	pub fn log_or_print<S1: AsRef<str>, S2: AsRef<str>>(
		&self, message: S1, channel: S2, severity: LogLevel,
	) {
		TsApi::static_log_or_print(message, channel, severity)
	}

	/// Get the server that has the specified id, returns `None` if there is no
	/// such server.
	pub fn get_server(&'_ self, server_id: ServerId) -> Option<Server<'_>> {
		self.servers.get(&server_id).map(|s| Server::new(&self, s))
	}

	pub fn get_permission(&self, _permission_id: PermissionId) -> Option<&Permission> {
		// TODO
		Some(&Permission {})
	}

	/// Print a message to the currently selected tab. This is only
	/// visible in the window of this client and will not be sent to the server.
	pub fn print_message<S: AsRef<str>>(&self, message: S) {
		let text = to_cstring!(message.as_ref());
		(TS3_FUNCTIONS
			.read()
			.unwrap()
			.as_ref()
			.expect("Functions should be loaded")
			.print_message_to_current_tab)(text.as_ptr());
	}

	/// Get the application path of the TeamSpeak executable.
	pub fn get_app_path(&self) -> String {
		TsApi::get_path(|p, l| {
			(TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_app_path)(p, l)
		})
	}

	/// Get the resource path of TeamSpeak.
	pub fn get_resources_path(&self) -> String {
		TsApi::get_path(|p, l| {
			(TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_resources_path)(p, l)
		})
	}

	/// Get the path, where configuration files are stored.
	/// This is e.g. `~/.ts3client` on linux or `%AppData%/TS3Client` on Windows.
	pub fn get_config_path(&self) -> String {
		TsApi::get_path(|p, l| {
			(TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_config_path)(p, l)
		})
	}

	/// Get the path where TeamSpeak plugins are stored.
	pub fn get_plugin_path(&self) -> String {
		TsApi::get_path(|p, l| {
			(TS3_FUNCTIONS
				.read()
				.unwrap()
				.as_ref()
				.expect("Functions should be loaded")
				.get_plugin_path)(p, l, to_cstring!(self.plugin_id.as_str()).as_ptr())
		})
	}
}
