#![feature(slice_concat_ext)]

use std::env;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::slice::SliceConcatExt;

type Map<K, V> = BTreeMap<K, V>;

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

    // Map types to functions that will get that type
    let default_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_property_as_int");
        m.insert("String", "get_property_as_string");
        m
    };

    // Structs
    f.write_all("/// Server properties that have to be fetched explicitely
pub struct OptionalServerData {".as_bytes()).unwrap();
    f.write_all(create_struct(&optional_server_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();

    f.write_all("/// Server properties that are available at the start but not updated
pub struct OutdatedServerData {".as_bytes()).unwrap();
    f.write_all(create_struct(&outdated_server_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();

    f.write_all("pub struct Server {".as_bytes()).unwrap();
    f.write_all(create_struct(&server).as_bytes()).unwrap();
    f.write_all("
    visible_connections: Map<ConnectionId, Connection>,
    outdated_data: OutdatedServerData,
    optional_data: Option<OptionalServerData>,
}\n\n".as_bytes()).unwrap();

    // Implementations
    f.write_all("impl OptionalServerData {".as_bytes()).unwrap();
    f.write_all(create_impl(&optional_server_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();

    f.write_all("impl OutdatedServerData {".as_bytes()).unwrap();
    f.write_all(create_impl(&outdated_server_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();

    f.write_all("impl Server {".as_bytes()).unwrap();
    f.write_all(create_impl(&server).as_bytes()).unwrap();
    f.write_all("
    fn get_outdated_data(&self) -> &OutdatedServerData {
        &self.outdated_data
    }
    fn get_optional_data(&self) -> &Option<OptionalServerData> {
        &self.optional_data
    }

    fn new(id: ServerId) -> Result<Server, Error> {
        let uid = try!(Server::get_property_as_string(id, VirtualServerProperties::UniqueIdentifier));
        // Enums have to be transmuted
        let codec_encryption_mode = unsafe { transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::CodecEncryptionMode))) };
        let hostbanner_mode = unsafe { transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::HostbannerMode))) };
        let hostmessage_mode = unsafe { transmute(try!(Server::get_property_as_int(id, VirtualServerProperties::HostmessageMode))) };
        let hostmessage = try!(Server::get_property_as_string(id, VirtualServerProperties::Hostmessage));

        //TODO
        let created = UTC::now();
        let default_server_group = Permissions;
        let default_channel_group = Permissions;
        let default_channel_admin_group = Permissions;
        //TODO Query currently visible connections on this server
        let visible_connections = Map::new();".as_bytes()).unwrap();

    // Initialize variables and ignore uid because it has another name
    {
        let mut ss = vec![server[0]];
        ss.extend_from_slice(&server[2..]);
        f.write_all(constructor_variables("Server", "VirtualServerProperties", &default_functions, "id, ", &ss).as_bytes()).unwrap();
    }

    f.write_all("

        Ok(Server {".as_bytes()).unwrap();
    f.write_all(constructor_creation(&server).as_bytes()).unwrap();

    f.write_all("
            visible_connections: visible_connections,
            outdated_data: OutdatedServerData {
                hostmessage: hostmessage,
                hostmessage_mode: hostmessage_mode,
            },
            optional_data: None,
        })
    }
}\n\n".as_bytes()).unwrap();
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

fn create_impl(data: &Vec<(&str, &str)>) -> String {
    let mut s = String::new();
    for &(name, var_type) in data {
        let is_ref_type = ["String", "Permissions"].contains(&var_type);
        s.write_str("\n\tpub fn get_").unwrap();
        s.write_str(name).unwrap();
        s.write_str("(&self) -> ").unwrap();
        if is_ref_type {
            s.write_str("&").unwrap();
        }
        s.write_str(var_type).unwrap();
        s.write_str(" {\n\t\t").unwrap();
        if is_ref_type {
            s.write_str("&").unwrap();
        }
        s.write_str("self.").unwrap();
        s.write_str(name).unwrap();
        s.write_str("\n\t}").unwrap();
    }
    s
}

/// struct_name: Name of the struct
/// properties_name: Name of the properties enum
/// args: Base args (id) to get properties
fn constructor_variables(struct_name: &str, properties_name: &str, functions: &Map<&str, &str>, args: &str, data: &Vec<(&str, &str)>) -> String {
    let mut result = String::new();
    for &(name, var_type) in data {
        let mut s = String::new();
        // Ignore unknown types
        if let Some(function) = functions.get(var_type) {
            write!(s, "try!({}::{}({}{}::{}));", struct_name, function, args, properties_name, to_pascal_case(name)).unwrap();
        } else {
            match var_type {
                "Duration" => { write!(s, "Duration::seconds(try!({}::{}({}{}::{})) as i64);", struct_name, "get_property_as_int", args, properties_name, to_pascal_case(name)).unwrap(); }
                "bool" => { write!(s, "try!({}::{}({}{}::{})) != 0;", struct_name, "get_property_as_int", args, properties_name, to_pascal_case(name)).unwrap(); }
                _ => {}
            }
        }
        if !s.is_empty() {
            result.write_str("\n\t\tlet ").unwrap();
            result.write_str(name).unwrap();
            result.write_str(" = ").unwrap();
            result.write_str(&s).unwrap();
        }
    }
    result
}

fn constructor_creation(data: &Vec<(&str, &str)>) -> String {
    let mut s = String::new();
    for &(name, _) in data {
        s.write_str("\n\t\t\t").unwrap();
        s.write_str(name).unwrap();
        s.write_str(": ").unwrap();
        s.write_str(name).unwrap();
        s.write_str(",").unwrap();
    }
    s
}

fn to_pascal_case(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    let mut uppercase = true;
    for c in text.chars() {
        if c == '_' {
            uppercase = true;
        } else {
            if uppercase {
                s.push(c.to_uppercase().next().unwrap());
                uppercase = false;
            } else {
                s.push(c);
            }
        }
    }
    s
}
