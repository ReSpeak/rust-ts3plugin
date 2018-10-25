//! A crate to create TeamSpeak3 plugins.
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
//!     fn name()        -> String { String::from("My Ts Plugin") }
//!     fn version()     -> String { String::from("0.1.0") }
//!     fn author()      -> String { String::from("My Name") }
//!     fn description() -> String { String::from("A wonderful tiny example plugin") }
//!     // Optional
//!     fn command() -> Option<String> { Some(String::from("myplugin")) }
//!     fn autoload() -> bool { false }
//!     fn configurable() -> ConfigureOffer { ConfigureOffer::No }
//!
//!     fn new(api: &mut TsApi) -> Result<Box<MyTsPlugin>, InitError> {
//!         api.log_or_print("Inited", "MyTsPlugin", LogLevel::Info);
//!         Ok(Box::new(MyTsPlugin))
//!         // Or return Err(InitError::Failure) on failure
//!     }
//!
//!     // Implement callbacks here
//!
//!     fn shutdown(&mut self, api: &mut TsApi) {
//!         api.log_or_print("Shutdown", "MyTsPlugin", LogLevel::Info);
//!     }
//! }
//!
//! create_plugin!(MyTsPlugin);
//!
//! # fn main() { }
//! ```

#![allow(dead_code)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate chrono;
#[macro_use]
extern crate lazy_static;
extern crate ts3plugin_sys;

pub use ts3plugin_sys::clientlib_publicdefinitions::*;
pub use ts3plugin_sys::plugin_definitions::*;
pub use ts3plugin_sys::public_definitions::*;
pub use ts3plugin_sys::public_errors::Error;
pub use ts3plugin_sys::ts3functions::Ts3Functions;

pub use plugin::*;

use std::str::Utf8Error;
use std::collections::HashMap as Map;
use std::ffi::{CStr, CString};
use std::mem::transmute;
use std::os::raw::{c_void,c_char, c_int};
use chrono::*;

/// Converts a normal string to a CString.
macro_rules! to_cstring {
	($string: expr) => {
		CString::new($string).unwrap_or(
			CString::new("String contains null character").unwrap())
	};
}

/// Converts a CString to a normal string.
macro_rules! to_string {
	($string: expr) => {
		String::from_utf8_lossy(CStr::from_ptr($string).to_bytes()).into_owned()
	};
}

// Declare modules here so the macros are visible in the modules
pub mod ts3interface;
pub mod plugin;

// Import automatically generated structs
include!(concat!(env!("OUT_DIR"), "/structs.rs"));

// ******************** Structs ********************
/// The main struct that contains all permanently save data.
pub struct TsApi {
	/// All known servers.
	servers: Map<ServerId, Server>,
	/// The plugin id from TeamSpeak.
	plugin_id: String,
}

/// A struct for convenience. The invoker is maybe not visible to the user,
/// but we can get events caused by him, so some information about him
/// are passed along with his id.
#[derive(Eq, Clone)]
pub struct Invoker {
	id: ConnectionId,
	uid: String,
	name: String,
}

/// The possible receivers of a message. A message can be sent to a specific
/// connection, to the current channel chat or to the server chat.
#[derive(Clone)]
pub enum MessageReceiver {
	Connection(ConnectionId),
	Channel,
	Server,
}

/// Permissions - TODO not yet implemented
#[derive(Clone)]
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct PermissionId(u32);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ServerGroupId(u64);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct ChannelGroupId(u64);


// ******************** Implementation ********************

// ********** Invoker **********
impl PartialEq<Invoker> for Invoker {
	fn eq(&self, other: &Invoker) -> bool {
		self.id == other.id
	}
}

impl Invoker {
	fn new(id: ConnectionId, uid: String, name: String) -> Invoker {
		Invoker {
			id: id,
			uid: uid,
			name: name,
		}
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

// ********** Server **********
impl PartialEq<Server> for Server {
	fn eq(&self, other: &Server) -> bool {
		self.id == other.id
	}
}
impl Eq for Server {}

impl Server {
	/// Get a server property that is stored as a string.
	fn get_property_as_string(id: ServerId, property: VirtualServerProperties) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_server_variable_as_string)
					(id.0, property as usize, &mut name));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res)
			}
		}
	}

	/// Get a server property that is stored as an int.
	fn get_property_as_int(id: ServerId, property: VirtualServerProperties) -> Result<i32, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_server_variable_as_int)
					(id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number as i32),
				_ => Err(res)
			}
		}
	}

	/// Get a server property that is stored as an int.
	fn get_property_as_uint64(id: ServerId, property: VirtualServerProperties) -> Result<u64, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_server_variable_as_uint64)
					(id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res)
			}
		}
	}

	/// Get the connection id of our own client.
	/// Called when a new Server is created.
	fn query_own_connection_id(id: ServerId) -> Result<ConnectionId, Error> {
		unsafe {
			let mut number: u16 = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_client_id)
					(id.0, &mut number));
			match res {
				Error::Ok => Ok(ConnectionId(number)),
				_ => Err(res)
			}
		}
	}

	/// Get all currently active connections on this server.
	/// Called when a new Server is created.
	/// When an error occurs, users are not inserted into the map.
	fn query_connections(id: ServerId) -> Map<ConnectionId, Connection> {
		let mut map = Map::new();
		// Query connected connections
		let mut result: *mut u16 = std::ptr::null_mut();
		let res: Error = unsafe { transmute((TS3_FUNCTIONS.as_ref()
			.expect("Functions should be loaded").get_client_list)
				(id.0, &mut result)) };
		if res == Error::Ok {
			unsafe {
				let mut counter = 0;
				while *result.offset(counter) != 0 {
					let connection_id = ConnectionId(*result.offset(counter));
					let mut connection = Connection::new(id, connection_id);
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
	fn query_channels(id: ServerId) -> Result<Map<ChannelId, Channel>, Error> {
		let mut map = Map::new();
		// Query connected connections
		let mut result: *mut u64 = std::ptr::null_mut();
		let res: Error = unsafe { transmute((TS3_FUNCTIONS.as_ref()
			.expect("Functions should be loaded").get_channel_list)
				(id.0, &mut result)) };
		if res == Error::Ok {
			unsafe {
				let mut counter = 0;
				while *result.offset(counter) != 0 {
					let channel_id = ChannelId(*result.offset(counter));
					let mut channel = Channel::new(id, channel_id);
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

	fn add_connection(&mut self, connection_id: ConnectionId) -> &mut Connection {
		let mut connection = Connection::new(self.id, connection_id);
		connection.update();
		self.visible_connections.insert(connection_id, connection);
		self.visible_connections.get_mut(&connection_id).unwrap()
	}

	fn remove_connection(&mut self, connection_id: ConnectionId) -> Option<Connection> {
		self.visible_connections.remove(&connection_id)
	}

	fn add_channel(&mut self, channel_id: ChannelId) -> Result<&mut Channel, Error> {
		match self.channels {
			Ok(ref mut cs) => {
				let mut channel = Channel::new(self.id, channel_id);
				channel.update();
				cs.insert(channel_id, channel);
				Ok(cs.get_mut(&channel_id).unwrap())
			}
			Err(error) => Err(error),
		}
	}

	fn remove_channel(&mut self, channel_id: ChannelId) -> Option<Channel> {
		self.channels.as_mut().ok().and_then(|mut cs| cs.remove(&channel_id))
	}

	// ********** Public Interface **********

	/// Get the ids of all visible connections on this server.
	pub fn get_connection_ids(&self) -> Vec<ConnectionId> {
		self.visible_connections.keys().cloned().collect()
	}

	/// Get the ids of all channels on this server.
	pub fn get_channel_ids(&self) -> Vec<ChannelId> {
		match self.channels {
			Ok(ref cs) => cs.keys().cloned().collect(),
			Err(_) => Vec::new(),
		}
	}

	/// Get the connection on this server that has the specified id, returns
	/// `None` if there is no such connection.
	pub fn get_connection(&self, connection_id: ConnectionId) -> Option<&Connection> {
		self.visible_connections.get(&connection_id)
	}

	/// Get the mutable connection on this server that has the specified id, returns
	/// `None` if there is no such connection.
	pub fn get_mut_connection(&mut self, connection_id: ConnectionId) -> Option<&mut Connection> {
		self.visible_connections.get_mut(&connection_id)
	}

	/// Get the channel on this server that has the specified id, returns
	/// `None` if there is no such channel.
	pub fn get_channel(&self, channel_id: ChannelId) -> Option<&Channel> {
		self.channels.as_ref().ok().and_then(|cs| cs.get(&channel_id))
	}

	/// Get the mutable channel on this server that has the specified id, returns
	/// `None` if there is no such channel.
	pub fn get_mut_channel(&mut self, channel_id: ChannelId) -> Option<&mut Channel> {
		self.channels.as_mut().ok().and_then(|cs| cs.get_mut(&channel_id))
	}

	/// Send a message to the server chat.
	pub fn send_message<S: AsRef<str>>(&self, message: S) -> Result<(), Error> {
		unsafe {
			let text = to_cstring!(message.as_ref());
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").request_send_server_text_msg)
					(self.id.0, text.as_ptr(), std::ptr::null()));
			match res {
				Error::Ok => Ok(()),
				_ => Err(res)
			}
		}
	}

	/// Sends a plugin message to all connections on the server.
	///
	/// Messages can be received in [`Plugin::plugin_message`].
	/// This is refered to as `PluginCommand` in TeamSpeak.
	///
	/// [`Plugin::plugin_message`]: plugin/trait.Plugin.html#method.plugin_message
	pub fn send_plugin_message<S: AsRef<str>>(&self, plugin_id: &str, message: S) {
		// FIXME: Needing the plugin id as argument is weird
		unsafe {
			let text = to_cstring!(message.as_ref());
			(TS3_FUNCTIONS.as_ref().expect("Functions should be loaded").send_plugin_command)
					(self.id.0, to_cstring!(plugin_id).as_ptr(), text.as_ptr(),
					PluginTargetMode::Server as i32, std::ptr::null(), std::ptr::null());
		}
	}

	/// Print a message into the server or channel tab of this server. This is only
	/// visible in the window of this client and will not be sent to the server.
	pub fn print_message<S: AsRef<str>>(&self, message: S, target: MessageTarget) {
		unsafe {
			let text = to_cstring!(message.as_ref());
			(TS3_FUNCTIONS.as_ref().expect("Functions should be loaded").print_message)
					(self.id.0, text.as_ptr(), target);
		}
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
	/// Get a channel property that is stored as a string.
	fn get_property_as_string(server_id: ServerId, id: ChannelId, property: ChannelProperties) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_channel_variable_as_string)
					(server_id.0, id.0, property as usize, &mut name));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res)
			}
		}
	}

	/// Get a channel property that is stored as an int.
	fn get_property_as_int(server_id: ServerId, id: ChannelId, property: ChannelProperties) -> Result<i32, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_channel_variable_as_int)
					(server_id.0, id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number as i32),
				_ => Err(res)
			}
		}
	}

	/// Get a channel property that is stored as an uint64.
	fn get_property_as_uint64(server_id: ServerId, id: ChannelId, property: ChannelProperties) -> Result<i32, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_channel_variable_as_uint64)
					(server_id.0, id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number as i32),
				_ => Err(res)
			}
		}
	}

	/// Ask the TeamSpeak api about the parent channel id of a channel.
	fn query_parent_channel_id(server_id: ServerId, id: ChannelId) -> Result<ChannelId, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_parent_channel_of_channel)
					(server_id.0, id.0, &mut number));
			match res {
				Error::Ok => Ok(ChannelId(number)),
				_ => Err(res)
			}
		}
	}

	/// Send a message to this channel chat.
	pub fn send_message<S: AsRef<str>>(&self, message: S) -> Result<(), Error> {
		unsafe {
			let text = to_cstring!(message.as_ref());
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").request_send_channel_text_msg)
					(self.server_id.0, text.as_ptr(), self.id.0, std::ptr::null()));
			match res {
				Error::Ok => Ok(()),
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
	/// Get a connection property that is stored as a string.
	fn get_connection_property_as_string(server_id: ServerId, id: ConnectionId, property: ConnectionProperties) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_connection_variable_as_string)
					(server_id.0, id.0, property as usize, &mut name));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res)
			}
		}
	}

	/// Get a connection property that is stored as a uint64.
	fn get_connection_property_as_uint64(server_id: ServerId, id: ConnectionId, property: ConnectionProperties) -> Result<u64, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_connection_variable_as_uint64)
					(server_id.0, id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res)
			}
		}
	}

	/// Get a connection property that is stored as a double.
	fn get_connection_property_as_double(server_id: ServerId, id: ConnectionId, property: ConnectionProperties) -> Result<f64, Error> {
		unsafe {
			let mut number: f64 = 0.0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_connection_variable_as_double)
					(server_id.0, id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res)
			}
		}
	}

	/// Get a client property that is stored as a string.
	fn get_client_property_as_string(server_id: ServerId, id: ConnectionId, property: ClientProperties) -> Result<String, Error> {
		unsafe {
			let mut name: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_client_variable_as_string)
					(server_id.0, id.0, property as usize, &mut name));
			match res {
				Error::Ok => Ok(to_string!(name)),
				_ => Err(res)
			}
		}
	}

	/// Get a client property that is stored as an int.
	fn get_client_property_as_int(server_id: ServerId, id: ConnectionId, property: ClientProperties) -> Result<c_int, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_client_variable_as_int)
					(server_id.0, id.0, property as usize, &mut number));
			match res {
				Error::Ok => Ok(number),
				_ => Err(res)
			}
		}
	}

	/// Ask the TeamSpeak api about the current channel id of a connection.
	fn query_channel_id(server_id: ServerId, id: ConnectionId) -> Result<ChannelId, Error> {
		unsafe {
			let mut number: u64 = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_channel_of_client)
					(server_id.0, id.0, &mut number));
			match res {
				Error::Ok => Ok(ChannelId(number)),
				_ => Err(res)
			}
		}
	}

	/// Ask the TeamSpeak api, if the specified connection is currently whispering to our own client.
	fn query_whispering(server_id: ServerId, id: ConnectionId) -> Result<bool, Error> {
		unsafe {
			let mut number: c_int = 0;
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").is_whispering)
					(server_id.0, id.0, &mut number));
			match res {
				Error::Ok => Ok(number != 0),
				_ => Err(res)
			}
		}
	}

	/// Send a private message to this connection.
	pub fn send_message<S: AsRef<str>>(&self, message: S) -> Result<(), Error> {
		unsafe {
			let text = to_cstring!(message.as_ref());
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").request_send_private_text_msg)
					(self.server_id.0, text.as_ptr(), self.id.0, std::ptr::null()));
			match res {
				Error::Ok => Ok(()),
				_ => Err(res)
			}
		}
	}
}

// Client property value which can be converted to a UTF8 String
pub struct ClientStringVariable {
	pub value: *mut c_char
}

impl ClientStringVariable {
	/// Get a &str from property value or return an Utf8Error on conversion fail
	pub fn to_str<'a>(&'a self) -> Result<&'a str, Utf8Error> {
		unsafe {CStr::from_ptr(self.value).to_str()}
	}

	/// Get owned String from lossy conversion of value
	pub fn to_owned_string_lossy(&self) -> String {
		unsafe {CStr::from_ptr(self.value).to_string_lossy().into_owned()}
	}
}

impl Drop for ClientStringVariable {
	fn drop(&mut self) {
		use std::os::raw::c_void;
		unsafe {
			(TS3_FUNCTIONS.as_ref()
			.expect("Functions should be loaded").free_memory)
				(self.value as *mut c_void);
		}
	}
}


// ********** TsApi **********
/// The api functions provided by TeamSpeak
static mut TS3_FUNCTIONS: Option<Ts3Functions> = None;

// Don't provide a default Implementation because we don't want the TsApi
// to be publicly constructable.
impl TsApi {
	/// Create a new TsApi instance without loading anything.
	fn new(plugin_id: String) -> TsApi {
		TsApi {
			servers: Map::new(),
			plugin_id: plugin_id,
		}
	}

	/// Load all currently connected server and their data.
	/// This should normally be executed after `new()`.
	fn load(&mut self) -> Result<(), Error> {
		// Query available connections
		let mut result: *mut u64 = std::ptr::null_mut();
		let res: Error = unsafe { transmute((TS3_FUNCTIONS.as_ref()
			.expect("Functions should be loaded").get_server_connection_handler_list)
				(&mut result)) };
		match res {
			Error::Ok => unsafe {
				let mut counter = 0;
				while *result.offset(counter) != 0 {
					// Test if we have a connection to this server.
					// We get open tabs, even if they are disconnected.
					let mut status: c_int = 0;
					let res: Error = transmute((TS3_FUNCTIONS.as_ref()
						.expect("Functions should be loaded").get_connection_status)
							(*result.offset(counter), &mut status));
					if res == Error::Ok && transmute::<c_int, ConnectStatus>(status) != ConnectStatus::Disconnected {
						self.add_server(ServerId(*result.offset(counter)));
					}
					counter += 1;
				}
			},
			_ => return Err(res)
		}
		Ok(())
	}

	/// Get client properties as string
	pub fn get_string_client_properties(&self,properties: ClientProperties, client: &ConnectionId, server: &ServerId) -> Result<ClientStringVariable,Error> {
		let ServerId(s_id) = server;
		let ConnectionId(c_id) = client;
		unsafe {
			let mut result: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_client_variable_as_string)
					(*s_id,*c_id,properties as usize,&mut result));
			match res {
				Error::Ok => {
					Ok(ClientStringVariable{value: result})
				},
				_ => Err(res)
			}
		}
	}

	/// Please try to use the member method `log_message` instead of this static method.
	pub fn static_log_message<S1: AsRef<str>, S2: AsRef<str>>(message: S1, channel: S2, severity: LogLevel) -> Result<(), Error> {
		unsafe {
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
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

	/// Please try to use the member method `get_error_message` instead of this static method.
	pub fn static_get_error_message(error: Error) -> Result<String, Error> {
		unsafe {
			let mut message: *mut c_char = std::ptr::null_mut();
			let res: Error = transmute((TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_error_message)
				(error as u32, &mut message));
			match res {
				Error::Ok => Ok(to_string!(message)),
				_ => Err(res)
			}
		}
	}

	// ********** Private Interface **********

	/// Add the server with the specified id to the server list.
	/// The currently available data of this server will be stored.
	fn add_server(&mut self, server_id: ServerId) -> &mut Server {
		self.servers.insert(server_id, Server::new(server_id));
		let mut server = self.servers.get_mut(&server_id).unwrap();
		server.update();
		server
	}

	/// Returns true if a server was removed
	fn remove_server(&mut self, server_id: ServerId) -> bool {
		self.servers.remove(&server_id).is_some()
	}

	/// Update the data of a connection with the data from the same connection
	/// as an invoker if possible.
	fn try_update_invoker(&mut self, server_id: ServerId, invoker: &Invoker) {
		if let Some(server) = self.get_mut_server(server_id) {
			if let Some(mut connection) = server.get_mut_connection(invoker.get_id()) {
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
		const MAX_SIZE: usize = 1000000;
		let mut size = START_SIZE;
		loop {
			let mut buf = vec![0 as u8; size];
			fun(buf.as_mut_ptr() as *mut c_char, size - 1);
			// Test if the allocated buffer was long enough
			if buf[size - 3] != 0 {
				size *= 2;
			} else {
				// Be sure that the string is terminated
				buf[size - 1] = 0;
				let s = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
				let result = s.to_string_lossy();
				return result.into_owned();
			}
		}
	}

	// ********** Public Interface **********

	/// Get the raw TeamSpeak api functions.
	/// These functions can be used to invoke actions that are not yet
	/// implemented by this library. You should file a bug report or make a pull
	/// request if you need to use this function.
	pub unsafe fn get_raw_api(&self) -> &Ts3Functions {
		TS3_FUNCTIONS.as_ref().unwrap()
	}

	/// Get the plugin id assigned by TeamSpeak.
	pub fn get_plugin_id(&self) -> &str {
		&self.plugin_id
	}

	/// Get all server ids to which this client is currently connected.
	pub fn get_server_ids(&self) -> Vec<ServerId> {
		self.servers.keys().cloned().collect()
	}

	/// Log a message using the TeamSpeak logging API.
	pub fn log_message<S1: AsRef<str>, S2: AsRef<str>>(&self, message: S1, channel: S2, severity: LogLevel) -> Result<(), Error> {
		TsApi::static_log_message(message, channel, severity)
	}

	/// Log a message using the TeamSpeak logging API.
	/// If that fails, print the message to stdout.
	pub fn log_or_print<S1: AsRef<str>, S2: AsRef<str>>(&self, message: S1, channel: S2, severity: LogLevel) {
		TsApi::static_log_or_print(message, channel, severity)
	}

	/// Get the server that has the specified id, returns `None` if there is no
	/// such server.
	pub fn get_server(&self, server_id: ServerId) -> Option<&Server> {
		self.servers.get(&server_id)
	}

	/// Get the mutable server that has the specified id, returns `None` if there is no
	/// such server.
	pub fn get_mut_server(&mut self, server_id: ServerId) -> Option<&mut Server> {
		self.servers.get_mut(&server_id)
	}

	/// Print a message to the currently selected tab. This is only
	/// visible in the window of this client and will not be sent to the server.
	pub fn print_message<S: AsRef<str>>(&self, message: S) {
		unsafe {
			let text = to_cstring!(message.as_ref());
			(TS3_FUNCTIONS.as_ref().expect("Functions should be loaded").print_message_to_current_tab)
					(text.as_ptr());
		}
	}

	/// Get the application path of the TeamSpeak executable.
	pub fn get_app_path(&self) -> String {
		unsafe {
			TsApi::get_path(|p, l| (TS3_FUNCTIONS.as_ref().expect("Functions should be loaded").get_app_path)(p, l))
		}
	}

	/// Get the resource path of TeamSpeak.
	pub fn get_resources_path(&self) -> String {
		unsafe {
			TsApi::get_path(|p, l| (TS3_FUNCTIONS.as_ref().expect("Functions should be loaded").get_resources_path)(p, l))
		}
	}

	/// Get the path, where configuration files are stored.
	/// This is e.g. `~/.ts3client` on linux or `%AppData%/TS3Client` on Windows.
	pub fn get_config_path(&self) -> String {
		unsafe {
			TsApi::get_path(|p, l| (TS3_FUNCTIONS.as_ref().expect("Functions should be loaded").get_config_path)(p, l))
		}
	}

	/// Get the path where TeamSpeak plugins are stored.
	pub fn get_plugin_path(&self) -> String {
		unsafe {
			TsApi::get_path(|p, l| (TS3_FUNCTIONS.as_ref()
				.expect("Functions should be loaded").get_plugin_path)(p, l,
				to_cstring!(self.plugin_id.as_str()).as_ptr()))
		}
	}
}
