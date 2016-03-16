#![feature(slice_concat_ext)]

use std::env;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::slice::SliceConcatExt;

/// Build parts of lib.rs as most of the structs are very repetitive
fn main() {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
	println!("cargo:rerun-if-changed={}/src/build.rs", manifest_dir);

	let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("structs.rs");
    let mut f = File::create(&dest_path).unwrap();

	// Server
	// Optional server data
	let optional_server_data = vec![
		("welcome_message", "String"),
		("max_clients", "i32"),
		("clients_online", "i32"),
		("channels_online", "i32"),
		("client_connections", "i32"),
		("query_client_connections", "i32"),
		("query_clients_online", "i32"),
		("uptime", "Duration"),
		("password", "bool"),
		("max_download_total_bandwith", "i32"),
		("max_upload_total_bandwith", "i32"),
		("download_quota", "i32"),
		("upload_quota", "i32"),
		("month_bytes_downloaded", "i32"),
		("month_bytes_uploaded", "i32"),
		("total_bytes_downloaded", "i32"),
		("total_bytes_uploaded", "i32"),
		("complain_autoban_count", "i32"),
		("complain_autoban_time", "Duration"),
		("complain_remove_time", "Duration"),
		("min_clients_in_channel_before_forced_silence", "i32"),
		("antiflood_points_tick_reduce", "i32"),
		("antiflood_points_needed_command_block", "i32"),
		("antiflood_points_needed_ip_block", "i32"),
		("port", "i32"),
		("autostart", "bool"),
		("machine_id", "i32"),
		("needed_identity_security_level", "i32"),
		("log_client", "bool"),
		("log_query", "bool"),
		("log_channel", "bool"),
		("log_permissions", "bool"),
		("log_server", "bool"),
		("log_filetransfer", "bool"),
		("min_client_version", "String"),
		("total_packetloss_speech", "i32"),
		("total_packetloss_keepalive", "i32"),
		("total_packetloss_control", "i32"),
		("total_packetloss_total", "i32"),
		("total_ping", "i32"),
		("weblist_enabled", "bool"),
	];
	// Outdated server data
	let outdated_server_data = vec![
		("hostmessage", "String"),
		("hostmessage_mode", "HostmessageMode"),
	];
	// The real server data
	let server = vec![
		("id", "ServerId"),
		("uid", "String"),
		("name", "String"),
		("name_phonetic", "String"),
		("platform", "String"),
		("version", "String"),
		("created", "DateTime<UTC>"),
		("codec_encryption_mode", "CodecEncryptionMode"),
		("default_server_group", "Permissions"),
		("default_channel_group", "Permissions"),
		("default_channel_admin_group", "Permissions"),
		("hostbanner_url", "String"),
		("hostbanner_gfx_url", "String"),
		("hostbanner_gfx_interval", "Duration"),
		("hostbanner_mode", "HostbannerMode"),
		("priority_speaker_dimm_modificator", "i32"),
		("hostbutton_tooltip", "String"),
		("hostbutton_url", "String"),
		("hostbutton_gfx_url", "String"),
		("icon_id", "i32"),
		("reserved_slots", "i32"),
		("ask_for_privilegekey", "bool"),
		("channel_temp_delete_delay_default", "Duration"),
	];

	f.write_all("/// Server properties that are available at the start but not updated
pub struct OutdatedServerData {".as_bytes()).unwrap();
	f.write_all(create_struct(&outdated_server_data).as_bytes()).unwrap();
	f.write_all("\n}".as_bytes()).unwrap();

	f.write_all("/// Server properties that have to be fetched explicitely
pub struct OptionalServerData {".as_bytes()).unwrap();
	f.write_all(create_struct(&optional_server_data).as_bytes()).unwrap();
	f.write_all("\n}".as_bytes()).unwrap();

	f.write_all("pub struct Server {".as_bytes()).unwrap();
	f.write_all(create_struct(&server).as_bytes()).unwrap();
	f.write_all("
	visible_connections: Map<ConnectionId, Connection>,
	outdated_data: OutdatedServerData,
	optional_data: Option<OptionalServerData>,
}".as_bytes()).unwrap();
}

fn create_struct(data: &Vec<(&str, &str)>) -> String {
	let mut s = String::new();
	for &(name, var_type) in data {
		s.write_str("\n\t").unwrap();
		s.write_str(name).unwrap();
		s.write_str(": ").unwrap();
		s.write_str(var_type).unwrap();
		s.write_str(",").unwrap();
	}
	s
}
