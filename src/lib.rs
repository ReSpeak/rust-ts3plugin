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
//! struct MyTsPlugin {
//! 	api: TsApi
//! }
//!
//! impl MyTsPlugin {
//!     fn new(api: TsApi) -> Result<Box<MyTsPlugin>, InitError> {
//!         api.log_or_print("Inited", "MyTsPlugin", LogLevel::Info);
//!         Ok(Box::new(MyTsPlugin {
//!         	api: api
//!         }))
//!         // Or return Err(InitError::Failure) on failure
//!     }
//! }
//!
//! impl Plugin for MyTsPlugin {
//!     fn get_api(&self) -> &TsApi {
//!     	&self.api
//!     }
//!
//!     fn get_mut_api(&mut self) -> &mut TsApi {
//!     	&mut self.api
//!     }
//! }
//!
//! impl Drop for MyTsPlugin {
//!     fn drop(&mut self) {
//!         self.api.log_or_print("Shutdown", "MyTsPlugin", LogLevel::Info);
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

	/// TeamSpeak waits for a function result so the api will panic if
	/// someone trys to wait for a result coming from TeamSpeak.
	no_wait: bool,
}

pub struct Permissions;

/// Server properties that have to be fetched explicitely
pub struct OptionalServerData {
	welcome_message: String,
	max_clients: i32,
	clients_online: i32,
	channels_online: i32,
	client_connections: i32,
	query_client_connections: i32,
	query_clients_online: i32,
	uptime: Duration,
	password: bool,
	max_download_total_bandwith: i32,
	max_upload_total_bandwith: i32,
	download_quota: i32,
	upload_quota: i32,
	month_bytes_downloaded: i32,
	month_bytes_uploaded: i32,
	total_bytes_downloaded: i32,
	total_bytes_uploaded: i32,
	complain_autoban_count: i32,
	complain_autoban_time: Duration,
	complain_remove_time: Duration,
	min_clients_in_channel_before_forced_silence: i32,
	antiflood_points_tick_reduce: i32,
	antiflood_points_needed_command_block: i32,
	antiflood_points_needed_ip_block: i32,
	port: i32,
	autostart: bool,
	machine_id: i32,
	needed_identity_security_level: i32,
	log_client: bool,
	log_query: bool,
	log_channel: bool,
	log_permissions: bool,
	log_server: bool,
	log_filetransfer: bool,
	min_client_version: String,
	total_packetloss_speech: i32,
	total_packetloss_keepalive: i32,
	total_packetloss_control: i32,
	total_packetloss_total: i32,
	total_ping: i32,
	weblist_enabled: bool,
}

/// Server properties that are available at the start but not updated
pub struct OutdatedServerData {
	hostmessage: String,
	hostmessage_mode: HostmessageMode,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ServerId(u64);

pub struct Server {
	id: ServerId,
	uid: String,
	name: String,
	name_phonetic: String,
	platform: String,
	version: String,
	created: DateTime<UTC>,
	codec_encryption_mode: CodecEncryptionMode,
	default_server_group: Permissions,
	default_channel_group: Permissions,
	default_channel_admin_group: Permissions,
	hostbanner_url: String,
	hostbanner_gfx_url: String,
	hostbanner_gfx_interval: Duration,
	priority_speaker_dimm_modificator: i32,
	hostbutton_tooltip: String,
	hostbutton_url: String,
	hostbutton_gfx_url: String,
	icon_id: i32,
	reserved_slots: i32,
	ask_for_privilegekey: bool,
	hostbanner_mode: HostbannerMode,
	channel_temp_delete_delay_default: Duration,
	visible_connections: Map<ConnectionId, Connection>,
	outdated_data: OutdatedServerData,
	optional_data: Option<OptionalServerData>,

	no_wait: bool,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ChannelId(u64);

pub struct Channel {
	id: ChannelId,
	server_id: ServerId,

	no_wait: bool,
}

pub struct OwnConnectionData {
	/// ConnectionProperties
	server_ip: String,
	server_port: u16,

	// ClientProperties
	input_deactivated: InputDeactivationStatus,
	default_channel: ChannelId,
	default_token: String,
}

pub struct ServerqueryConnectionData {
	name: String,
	password: String,
}

pub struct OptionalConnectionData {
	version: String,
	platform: String,
	created: DateTime<UTC>,
	last_connected: DateTime<UTC>,
	total_connection: i32,
	month_bytes_uploaded: i32,
	month_bytes_downloaded: i32,
	total_bytes_uploaded: i32,
	total_bytes_downloaded: i32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ConnectionId(u16);

pub struct Connection {
	/// ConnectionProperties
	id: ConnectionId,
	server_id: ServerId,
	ping: Duration,
	ping_deciation: Duration,
	connected_time: Duration,
	idle_time: Duration,
	client_ip: String,
	client_port: String,
	/// Network
	packets_sent_speech: u64,
	packets_sent_keepalive: u64,
	packets_sent_control: u64,
	packets_sent_total: u64,
	bytes_sent_speech: u64,
	bytes_sent_keepalive: u64,
	bytes_sent_control: u64,
	bytes_sent_total: u64,
	packets_received_speech: u64,
	packets_received_keepalive: u64,
	packets_received_control: u64,
	packets_received_total: u64,
	bytes_received_speech: u64,
	bytes_received_keepalive: u64,
	bytes_received_control: u64,
	bytes_received_total: u64,
	packetloss_speech: u64,
	packetloss_keepalive: u64,
	packetloss_control: u64,
	packetloss_total: u64,
	//TODO much more...
	/// End network


	/// ClientProperties
	uid: String,
	name: String,
	talking: TalkStatus,
	input_muted: MuteInputStatus,
	output_muted: MuteOutputStatus,
	output_only_muted: MuteOutputStatus,
	input_hardware: HardwareInputStatus,
	output_hardware: HardwareOutputStatus,
	default_channel_password: String,
	server_password: String,
	/// If the client is locally muted.
	is_muted: bool,
	is_recording: bool,
	volume_modificator: i32,
	version_sign: String,
	away: AwayStatus,
	away_message: String,
	flag_avatar: bool,
	description: String,
	is_talker: bool,
	is_priority_speaker: bool,
	has_unread_messages: bool,
	phonetic_name: String,
	needed_serverquery_view_power: i32,
	icon_id: i32,
	is_channel_commander: bool,
	country: String,
	badges: String,
	/// Only valid data if we have the appropriate permissions.
	database_id: Option<Permissions>,
	channel_group_id: Option<Permissions>,
	server_groups: Option<Vec<Permissions>>,
	talk_power: Option<i32>,
	/// When this client requested to talk
	talk_request: Option<DateTime<UTC>>,
	talk_request_message: Option<String>,
	/// The channel that sets the current channel id of this client.
	channel_group_inherited_channel_id: Option<ChannelId>,
	/// Only set for oneself
	own_data: Option<OwnConnectionData>,
	serverquery_data: Option<ServerqueryConnectionData>,
	optional_data: Option<OptionalConnectionData>,

	no_wait: bool,
}


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

	fn new(id: ServerId) -> Result<Server, Error> {
		let uid = try!(Server::get_property_as_string(id, VirtualServerProperties::UniqueIdentifier));
		let name = try!(Server::get_property_as_string(id, VirtualServerProperties::Name));
		let name_phonetic = try!(Server::get_property_as_string(id, VirtualServerProperties::NamePhonetic));
		let platform = try!(Server::get_property_as_string(id, VirtualServerProperties::Platform));
		let version = try!(Server::get_property_as_string(id, VirtualServerProperties::Version));

		//TODO
		let created = UTC::now();

		let codec_encryption_mode = unsafe { transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::CodecEncryptionMode))) };

		//TODO
		let default_server_group = Permissions;
		let default_channel_group = Permissions;
		let default_channel_admin_group = Permissions;

		let hostbanner_url = try!(Server::get_property_as_string(id, VirtualServerProperties::HostbannerUrl));
		let hostbanner_gfx_url = try!(Server::get_property_as_string(id, VirtualServerProperties::HostbannerGfxUrl));
		let hostbanner_gfx_interval = Duration::seconds(try!(Server::get_property_as_int(id, VirtualServerProperties::PrioritySpeakerDimmModificator)) as i64);
		let priority_speaker_dimm_modificator = try!(Server::get_property_as_int(id, VirtualServerProperties::PrioritySpeakerDimmModificator));
		let hostbutton_tooltip = try!(Server::get_property_as_string(id, VirtualServerProperties::HostbuttonTooltip));
		let hostbutton_url = try!(Server::get_property_as_string(id, VirtualServerProperties::HostbuttonUrl));
		let hostbutton_gfx_url = try!(Server::get_property_as_string(id, VirtualServerProperties::HostbuttonGfxUrl));
		let icon_id = try!(Server::get_property_as_int(id, VirtualServerProperties::IconId));
		let reserved_slots = try!(Server::get_property_as_int(id, VirtualServerProperties::ReservedSlots));
		let ask_for_privilegekey = try!(Server::get_property_as_int(id, VirtualServerProperties::AskForPrivilegekey)) != 0;
		let hostbanner_mode = unsafe { transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::HostbannerMode))) };
		let channel_temp_delete_delay_default = Duration::seconds(try!(Server::get_property_as_int(id, VirtualServerProperties::AskForPrivilegekey)) as i64);
		let hostmessage = try!(Server::get_property_as_string(id, VirtualServerProperties::Hostmessage));
		let hostmessage_mode = unsafe { transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::HostmessageMode))) };

		//TODO Query currently visible connections on this server
		let visible_connections = Map::new();

		Ok(Server {
			id: id,
			uid: uid,
			name: name,
			name_phonetic: name_phonetic,
			platform: platform,
			version: version,
			created: created,
			codec_encryption_mode: codec_encryption_mode,
			default_server_group: default_server_group,
			default_channel_group: default_channel_group,
			default_channel_admin_group: default_channel_admin_group,
			hostbanner_url: hostbanner_url,
			hostbanner_gfx_url: hostbanner_gfx_url,
			hostbanner_gfx_interval: hostbanner_gfx_interval,
			priority_speaker_dimm_modificator: priority_speaker_dimm_modificator,
			hostbutton_tooltip: hostbutton_tooltip,
			hostbutton_url: hostbutton_url,
			hostbutton_gfx_url: hostbutton_gfx_url,
			icon_id: icon_id,
			reserved_slots: reserved_slots,
			ask_for_privilegekey: ask_for_privilegekey,
			hostbanner_mode: hostbanner_mode,
			channel_temp_delete_delay_default: channel_temp_delete_delay_default,
			visible_connections: visible_connections,
			outdated_data: OutdatedServerData {
				hostmessage: hostmessage,
				hostmessage_mode: hostmessage_mode,
			},
			optional_data: None,

			no_wait: false,
		})
	}

	fn add_connection(&mut self, connection_id: ConnectionId) -> Result<(), Error> {
		self.visible_connections.insert(connection_id, try!(Connection::new(self.id, connection_id)));
		Ok(())
	}

	pub fn get_name(&self) -> &String {
		&self.name
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

	fn new(server_id: ServerId, id: ConnectionId) -> Result<Connection, Error> {
		// ConnectionProperties
		let ping = Duration::milliseconds(try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::Ping)) as i64);
		let ping_deciation = Duration::milliseconds(try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PingDeciation)) as i64);
		let connected_time = Duration::seconds(try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::ConnectedTime)) as i64);
		let idle_time = Duration::seconds(try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::IdleTime)) as i64);
		let client_ip = try!(Connection::get_connection_property_as_string(server_id, id, ConnectionProperties::ClientIp));
		let client_port = try!(Connection::get_connection_property_as_string(server_id, id, ConnectionProperties::ClientPort));
		// Network
		let packets_sent_speech = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsSentSpeech));
		let packets_sent_keepalive = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsSentKeepalive));
		let packets_sent_control = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsSentControl));
		let packets_sent_total = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsSentTotal));
		let bytes_sent_speech = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesSentSpeech));
		let bytes_sent_keepalive = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesSentKeepalive));
		let bytes_sent_control = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesSentControl));
		let bytes_sent_total = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesSentTotal));
		let packets_received_speech = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsReceivedSpeech));
		let packets_received_keepalive = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsReceivedKeepalive));
		let packets_received_control = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsReceivedControl));
		let packets_received_total = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketsReceivedTotal));
		let bytes_received_speech = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesReceivedSpeech));
		let bytes_received_keepalive = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesReceivedKeepalive));
		let bytes_received_control = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesReceivedControl));
		let bytes_received_total = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::BytesReceivedTotal));
		let packetloss_speech = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketlossSpeech));
		let packetloss_keepalive = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketlossKeepalive));
		let packetloss_control = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketlossControl));
		let packetloss_total = try!(Connection::get_connection_property_as_uint64(server_id, id, ConnectionProperties::PacketlossTotal));
		// End network

		// ClientProperties
		let uid = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::UniqueIdentifier));
		let name = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::Nickname));
		let talking = unsafe { transmute(try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::FlagTalking))) };
		let input_muted = unsafe { transmute(try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::InputMuted))) };
		let output_muted = unsafe { transmute(try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::OutputMuted))) };
		let output_only_muted = unsafe { transmute(try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::OutputOnlyMuted))) };
		let input_hardware = unsafe { transmute(try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::InputHardware))) };
		let output_hardware = unsafe { transmute(try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::OutputHardware))) };
		let default_channel_password = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::DefaultChannelPassword));
		let server_password = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::ServerPassword));
		let is_muted = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::IsMuted)) != 0;
		let is_recording = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::IsRecording)) != 0;
		let volume_modificator = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::VolumeModificator));
		let version_sign = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::VersionSign));
		let away = unsafe { transmute(try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::Away))) };
		let away_message = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::AwayMessage));
		let flag_avatar = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::FlagAvatar)) != 0;
		let description = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::Description));
		let is_talker = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::IsTalker)) != 0;
		let is_priority_speaker = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::IsPrioritySpeaker)) != 0;
		let has_unread_messages = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::UnreadMessages)) != 0;
		let phonetic_name = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::NicknamePhonetic));
		let needed_serverquery_view_power = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::NeededServerqueryViewPower));
		let icon_id = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::IconId));
		let is_channel_commander = try!(Connection::get_client_property_as_int(server_id, id, ClientProperties::IsChannelCommander)) != 0;
		let country = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::Country));
		let badges = try!(Connection::get_client_property_as_string(server_id, id, ClientProperties::Badges));

		Ok(Connection {
			id: id,
			server_id: server_id,
			// ConnectionProperties
			ping: ping,
			ping_deciation: ping_deciation,
			connected_time: connected_time,
			idle_time: idle_time,
			client_ip: client_ip,
			client_port: client_port,
			// Network
			packets_sent_speech: packets_sent_speech,
			packets_sent_keepalive: packets_sent_keepalive,
			packets_sent_control: packets_sent_control,
			packets_sent_total: packets_sent_total,
			bytes_sent_speech: bytes_sent_speech,
			bytes_sent_keepalive: bytes_sent_keepalive,
			bytes_sent_control: bytes_sent_control,
			bytes_sent_total: bytes_sent_total,
			packets_received_speech: packets_received_speech,
			packets_received_keepalive: packets_received_keepalive,
			packets_received_control: packets_received_control,
			packets_received_total: packets_received_total,
			bytes_received_speech: bytes_received_speech,
			bytes_received_keepalive: bytes_received_keepalive,
			bytes_received_control: bytes_received_control,
			bytes_received_total: bytes_received_total,
			packetloss_speech: packetloss_speech,
			packetloss_keepalive: packetloss_keepalive,
			packetloss_control: packetloss_control,
			packetloss_total: packetloss_total,
			// End network

			// ClientProperties
			uid: uid,
			name: name,
			talking: talking,
			input_muted: input_muted,
			output_muted: output_muted,
			output_only_muted: output_only_muted,
			input_hardware: input_hardware,
			output_hardware: output_hardware,
			default_channel_password: default_channel_password,
			server_password: server_password,
			is_muted: is_muted,
			is_recording: is_recording,
			volume_modificator: volume_modificator,
			version_sign: version_sign,
			away: away,
			away_message: away_message,
			flag_avatar: flag_avatar,
			description: description,
			is_talker: is_talker,
			is_priority_speaker: is_priority_speaker,
			has_unread_messages: has_unread_messages,
			phonetic_name: phonetic_name,
			needed_serverquery_view_power: needed_serverquery_view_power,
			icon_id: icon_id,
			is_channel_commander: is_channel_commander,
			country: country,
			badges: badges,
			//TODO
			database_id: None,
			channel_group_id: None,
			server_groups: None,
			talk_power: None,
			talk_request: None,
			talk_request_message: None,
			channel_group_inherited_channel_id: None,
			own_data: None,
			serverquery_data: None,
			optional_data: None,

			no_wait: false,
		})
	}

	pub fn get_name(&self) -> &String {
		&self.name
	}
}


// ********** TsApi **********
/// The api functions provided by TeamSpeak
static mut ts3functions: Option<Ts3Functions> = None;

impl TsApi {
	/// Create a new TsApi instance without loading anything.
	/// This will be called from the `create_plugin!` macro.
	/// This function is not meant for public use.
	pub fn new() -> TsApi {
		TsApi {
			servers: Map::new(),

			no_wait: false,
		}
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
                	try!(self.add_server(ServerId(*result.offset(counter))));
                    counter += 1;
                }
	        },
	        _ => return Err(res)
	    }
		Ok(())
	}

    fn static_log_message(message: &str, channel: &str, severity: LogLevel) -> Result<(), Error> {
        unsafe {
            let res: Error = transmute((ts3functions.as_ref()
                .expect("Functions should be loaded").log_message)
                    (to_cstring!(message).as_ptr(),
                    severity, to_cstring!(channel).as_ptr(), 0));
            match res {
                Error::Ok => Ok(()),
                _ => Err(res)
            }
        }
    }

    fn static_log_or_print(message: &str, channel: &str, severity: LogLevel) {
        if let Err(error) = TsApi::static_log_message(message, channel, severity) {
            println!("Error {:?} while printing '{}' to '{}' ({:?})", error,
                message, channel, severity);
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
    pub fn log_message(&self, message: &str, channel: &str, severity: LogLevel) -> Result<(), Error> {
    	TsApi::static_log_message(message, channel, severity)
    }

    /// Log a message using the TeamSpeak logging API.
	/// If that fails, print the message.
    pub fn log_or_print(&self, message: &str, channel: &str, severity: LogLevel) {
    	TsApi::static_log_or_print(message, channel, severity)
    }

    pub fn get_server(&self, server_id: ServerId) -> Option<&Server> {
    	self.servers.get(&server_id)
    }

    pub fn get_mut_server(&mut self, server_id: ServerId) -> Option<&mut Server> {
    	self.servers.get_mut(&server_id)
    }
}
