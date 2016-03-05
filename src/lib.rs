#![allow(dead_code)]
#![feature(macro_reexport)]

extern crate libc;
extern crate chrono;
#[macro_use]
#[macro_reexport(lazy_static)]
extern crate lazy_static;
extern crate ts3plugin_sys;

pub mod ts3interface;
pub mod plugin;

use libc::size_t;
use std::ffi::CStr;
use chrono::*;

pub use ts3plugin_sys::clientlib_publicdefinitions::*;
pub use ts3plugin_sys::plugin_definitions::*;
pub use ts3plugin_sys::public_definitions::*;
pub use ts3plugin_sys::public_errors::Error;
pub use ts3plugin_sys::ts3functions::Ts3Functions;

pub use plugin::*;

/// Converts a normal string to a CString
macro_rules! to_cstring
{
    ($string: expr) =>
    {
        CString::new($string).unwrap_or(
            CString::new("String contains null character").unwrap())
    };
}

// ******************** Structs ********************
pub struct TsApi;

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

pub struct Server {
	id: u64,
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
	outdated_data: OutdatedServerData,
	optional_data: Option<OptionalServerData>,
}

pub struct Channel<'a> {
	id: u64,
	server: &'a Server,
}

pub struct Connection<'a, 'b> {
	id: u16,
	server: &'a Server,
	client: &'b Client,
	talking: bool,
}

pub struct Client {
	uid: String,
	name: String,
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
    fn get_property_as_string(id: u64, property: VirtualServerProperties) -> Result<String, Error> {
        unsafe {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_server_variable_as_string)
                    (id, property as size_t, &mut name));
            match res {
                Error::Ok => Ok(String::from_utf8_lossy(CStr::from_ptr(name).to_bytes()).into_owned()),
                _ => Err(res)
            }
        }
    }

    fn get_property_as_int(id: u64, property: VirtualServerProperties) -> Result<i32, Error> {
        unsafe {
            let mut number: c_int = 0;
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_server_variable_as_int)
                    (id, property as size_t, &mut number));
            match res {
                Error::Ok => Ok(number as i32),
                _ => Err(res)
            }
        }
    }

	fn new(id: u64) -> Result<Server, Error> {
		let uid = try!(Server::get_property_as_string(id, VirtualServerProperties::UniqueIdentifier));
		let name = try!(Server::get_property_as_string(id, VirtualServerProperties::Name));
		let name_phonetic = try!(Server::get_property_as_string(id, VirtualServerProperties::NamePhonetic));
		let platform = try!(Server::get_property_as_string(id, VirtualServerProperties::Platform));
		let version = try!(Server::get_property_as_string(id, VirtualServerProperties::Version));

		//TODO
		let created = UTC::now();
		let codec_encryption_mode = unsafe { std::mem::transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::CodecEncryptionMode))) };
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
		let hostbanner_mode = unsafe { std::mem::transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::HostbannerMode))) };
		let channel_temp_delete_delay_default = Duration::seconds(try!(Server::get_property_as_int(id, VirtualServerProperties::AskForPrivilegekey)) as i64);
		let hostmessage = try!(Server::get_property_as_string(id, VirtualServerProperties::Hostmessage));
		let hostmessage_mode = unsafe { std::mem::transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::HostmessageMode))) };

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
			outdated_data: OutdatedServerData {
				hostmessage: hostmessage,
				hostmessage_mode: hostmessage_mode,
			},
			optional_data: None,
		})
	}
}

// ********** Channel **********
impl<'a> PartialEq<Channel<'a>> for Channel<'a> {
	fn eq(&self, other: &Channel) -> bool {
		self.server == other.server && self.id == other.id
	}
}
impl<'a> Eq for Channel<'a> {}

// ********** Connection **********
impl<'a, 'b> PartialEq<Connection<'a, 'b>> for Connection<'a, 'b> {
	fn eq(&self, other: &Connection) -> bool {
		self.server == other.server && self.id == other.id
	}
}
impl<'a, 'b> Eq for Connection<'a, 'b> {}

// ********** Client **********
impl PartialEq<Client> for Client {
	fn eq(&self, other: &Client) -> bool {
		self.uid == other.uid
	}
}
impl Eq for Client {}


// ********** TsApi **********

impl TsApi
{
    unsafe fn get_raw_functions<'a>() -> &'a Ts3Functions
    {
        ts3interface::ts3functions.as_ref().expect("Functions should be loaded")
    }

    pub fn log_message(message: &str, channel: &str, severity: LogLevel) -> Result<(), Error>
    {
        unsafe
        {
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").log_message)
                    (to_cstring!(message).as_ptr(),
                    severity, to_cstring!(channel).as_ptr(), 0));
            match res
            {
                Error::Ok => Ok(()),
                _ => Err(res)
            }
        }
    }

    pub fn log_or_print(message: &str, channel: &str, severity: LogLevel)
    {
        if let Err(error) = TsApi::log_message(message, channel, severity)
        {
            println!("Error {0:?} while printing '{1}' to {2} ({3:?})", error,
                message, channel, severity);
        }
    }
}
