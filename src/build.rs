#![feature(slice_concat_ext)]

use std::env;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::slice::SliceConcatExt;

type Map<K, V> = BTreeMap<K, V>;

#[derive(Default)]
struct Property<'a> {
    name: &'a str,
    type_s: &'a str,
    documentation: &'a str,
}

impl<'a> Property<'a> {
    fn new() -> Property<'a> {
        Self::default()
    }
}

#[derive(Default)]
struct PropertyBuilder<'a> {
    name: &'a str,
    type_s: &'a str,
    documentation: &'a str,
}

impl<'a> PropertyBuilder<'a> {
    fn new() -> PropertyBuilder<'a> {
        Self::default()
    }

    fn name(&mut self, name: &'a str) -> &mut PropertyBuilder<'a> {
        self.name = name;
        self
    }

    fn type_s(&mut self, type_s: &'a str) -> &mut PropertyBuilder<'a> {
        self.type_s = type_s;
        self
    }

    fn documentation(&mut self, documentation: &'a str) -> &mut PropertyBuilder<'a> {
        self.documentation = documentation;
        self
    }

    fn finalize(self) -> Property<'a> {
        Property {
            name: self.name,
            type_s: self.type_s,
            documentation: self.documentation,
        }
    }
}

struct Struct<'a> {
    /// The name of this struct
    name: &'a str,
    /// The documentation of this struct
    documentation: &'a str,
    /// Members that will be generated for this struct
    properties: Vec<Property<'a>>,
    /// Code that will be put into the struct part
    extra_attributes: &'a str,
    /// Code that will be inserted into the constructor (::new method)
    extra_initialisation: &'a str,
    /// Code that will be inserted into the creation of the struct
    extra_creation: &'a str,
}

#[derive(Default)]
struct StructBuilder<'a> {
    name: &'a str,
    documentation: &'a str,
    properties: Vec<Property<'a>>,
    extra_attributes: &'a str,
    extra_initialisation: &'a str,
    extra_creation: &'a str,
}

impl<'a> StructBuilder<'a> {
    fn new() -> StructBuilder<'a> {
        Self::default()
    }

    fn name(&mut self, name: &'a str) -> &mut StructBuilder<'a> {
        self.name = name;
        self
    }

    fn documentation(&mut self, documentation: &'a str) -> &mut StructBuilder<'a> {
        self.documentation = documentation;
        self
    }

    fn properties(&mut self, properties: Vec<Property<'a>>) -> &mut StructBuilder<'a> {
        self.properties = properties;
        self
    }

    fn extra_attributes(&mut self, extra_attributes: &'a str) -> &mut StructBuilder<'a> {
        self.extra_attributes = extra_attributes;
        self
    }

    fn extra_initialisation(&mut self, extra_initialisation: &'a str) -> &mut StructBuilder<'a> {
        self.extra_initialisation = extra_initialisation;
        self
    }

    fn extra_creation(&mut self, extra_creation: &'a str) -> &mut StructBuilder<'a> {
        self.extra_creation = extra_creation;
        self
    }

    fn finalize(self) -> Struct<'a> {
        Struct {
            name: self.name,
            documentation: self.documentation,
            properties: self.properties,
            extra_attributes: self.extra_attributes,
            extra_initialisation: self.extra_initialisation,
            extra_creation: self.extra_creation,
        }
    }
}

impl<'a> Struct<'a> {
    fn create_struct(&self) -> String {
        let mut s = String::new();
        for prop in &self.properties {
            if !prop.documentation.is_empty() {
                s.push_str(prop.documentation);
                s.push_str("\n");
            }
            write!(s, "{}: {},\n", prop.name, prop.type_s).unwrap();
        }
        indent(s.as_ref(), 1)
    }

    fn create_impl(&self) -> String {
        let mut s = String::new();
        for prop in &self.properties {
            let is_ref_type = ["String", "Permissions"].contains(&prop.type_s);
            write!(s, "pub fn get_{}(&self) -> ", prop.name).unwrap();
            if is_ref_type {
                s.write_str("&").unwrap();
            }
            write!(s, "{} {{\n\t", prop.type_s).unwrap();
            if is_ref_type {
                s.write_str("&").unwrap();
            }
            write!(s, "self.{}\n", prop.name).unwrap();
        }
        indent(s.as_ref(), 1)
    }

    fn constructor_variables(&self) -> String {
        //TODO
        String::new()
    }
}

fn create_server(f: &mut File) {
    // Map types to functions that will get that type
    let default_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_property_as_int");
        m.insert("String", "get_property_as_string");
        m
    };

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


    // Optional server data
    f.write_all("/// Server properties that have to be fetched explicitely
pub struct OptionalServerData {".as_bytes()).unwrap();
    f.write_all(create_struct(&optional_server_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();
    // Outdated server data
    f.write_all("/// Server properties that are available at the start but not updated
pub struct OutdatedServerData {".as_bytes()).unwrap();
    f.write_all(create_struct(&outdated_server_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();
    // Server
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

fn create_connection(f: &mut File) {
    // Map types to functions that will get that type
    let default_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_property_as_int");
        m.insert("String", "get_property_as_string");
        m
    };

    // Own connection data
    let own_connection_data = vec![
        ("server_ip", "String"),
        ("server_port", "u16"),
        ("input_deactivated", "InputDeactivationStatus"),
        ("default_channel", "ChannelId"),
        ("default_token", "String"),
    ];
    // Serverquery connection data
    let serverquery_connection_data = vec![
        ("name", "String"),
        ("password", "String"),
    ];
    // Optional connection data
    let optional_connection_data = vec![
        ("version", "String"),
        ("platform", "String"),
        ("created", "DateTime<UTC>"),
        ("last_connected", "DateTime<UTC>"),
        ("total_connection", "i32"),
        ("month_bytes_uploaded", "i32"),
        ("month_bytes_downloaded", "i32"),
        ("total_bytes_uploaded", "i32"),
        ("total_bytes_downloaded", "i32"),
    ];
    // The real connection data
    let connection = vec![
        ("id", "ConnectionId"),
        ("server_id", "ServerId"),
        ("ping", "Duration"),
        ("ping_deciation", "Duration"),
        ("connected_time", "Duration"),
        ("idle_time", "Duration"),
        ("client_ip", "String"),
        ("client_port", "String"),
        // Network
        ("packets_sent_speech", "u64"),
        ("packets_sent_keepalive", "u64"),
        ("packets_sent_control", "u64"),
        ("packets_sent_total", "u64"),
        ("bytes_sent_speech", "u64"),
        ("bytes_sent_keepalive", "u64"),
        ("bytes_sent_control", "u64"),
        ("bytes_sent_total", "u64"),
        ("packets_received_speech", "u64"),
        ("packets_received_keepalive", "u64"),
        ("packets_received_control", "u64"),
        ("packets_received_total", "u64"),
        ("bytes_received_speech", "u64"),
        ("bytes_received_keepalive", "u64"),
        ("bytes_received_control", "u64"),
        ("bytes_received_total", "u64"),
        ("packetloss_speech", "u64"),
        ("packetloss_keepalive", "u64"),
        ("packetloss_control", "u64"),
        ("packetloss_total", "u64"),
        //TODO much more...
        // End network

        // ClientProperties
        ("uid", "String"),
        ("name", "String"),
        ("talking", "TalkStatus"),
        ("input_muted", "MuteInputStatus"),
        ("output_muted", "MuteOutputStatus"),
        ("output_only_muted", "MuteOutputStatus"),
        ("input_hardware", "HardwareInputStatus"),
        ("output_hardware", "HardwareOutputStatus"),
        ("default_channel_password", "String"),
        ("server_password", "String"),
        // If the client is locally muted.
        ("is_muted", "bool"),
        ("is_recording", "bool"),
        ("volume_modificator", "i32"),
        ("version_sign", "String"),
        ("away", "AwayStatus"),
        ("away_message", "String"),
        ("flag_avatar", "bool"),
        ("description", "String"),
        ("is_talker", "bool"),
        ("is_priority_speaker", "bool"),
        ("has_unread_messages", "bool"),
        ("phonetic_name", "String"),
        ("needed_serverquery_view_power", "i32"),
        ("icon_id", "i32"),
        ("is_channel_commander", "bool"),
        ("country", "String"),
        ("badges", "String"),
        // Only valid data if we have the appropriate permissions.
        ("database_id", "Option<Permissions>"),
        ("channel_group_id", "Option<Permissions>"),
        ("server_groups", "Option<Vec<Permissions>>"),
        ("talk_power", "Option<i32>"),
        // When this client requested to talk
        ("talk_request", "Option<DateTime<UTC>>"),
        ("talk_request_message", "Option<String>"),
    ];

    // Own connection data
    f.write_all("pub struct OwnConnectionData {".as_bytes()).unwrap();
    f.write_all(create_struct(&own_connection_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();
    // Serverquery connection data
    f.write_all("pub struct ServerqueryConnectionData {".as_bytes()).unwrap();
    f.write_all(create_struct(&serverquery_connection_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();
    // Optional connection data
    f.write_all("pub struct OptionalConnectionData {".as_bytes()).unwrap();
    f.write_all(create_struct(&optional_connection_data).as_bytes()).unwrap();
    f.write_all("\n}\n\n".as_bytes()).unwrap();
    // Connection
    f.write_all("pub struct Connection {".as_bytes()).unwrap();
    f.write_all(create_struct(&connection).as_bytes()).unwrap();
    f.write_all("
    /// The channel that sets the current channel id of this client.
    channel_group_inherited_channel_id: Option<ChannelId>,
    /// Only set for oneself
    own_data: Option<OwnConnectionData>,
    serverquery_data: Option<ServerqueryConnectionData>,
    optional_data: Option<OptionalConnectionData>,
}\n\n".as_bytes()).unwrap();
}

/// Build parts of lib.rs as most of the structs are very repetitive
fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rerun-if-changed={}/src/build.rs", manifest_dir);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("structs.rs");
    let mut f = File::create(&dest_path).unwrap();

    create_server(&mut f);
    create_connection(&mut f);
}

fn create_struct(data: &Vec<(&str, &str)>) -> String {
    let mut s = String::new();
    for &(name, var_type) in data {
        write!(s, "\n\t{}: {},", name, var_type).unwrap();
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
            write!(result, "\n\t\tlet {} = {}", name, s).unwrap();
        }
    }
    result
}

fn constructor_creation(data: &Vec<(&str, &str)>) -> String {
    let mut s = String::new();
    for &(name, _) in data {
        write!(s, "\n\t\t\t{0}: {0},", name).unwrap();
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

/// Indent a string by a given count using spaces.
fn indent(s: &str, count: usize) -> String {
    let line_count = s.lines().count();
    let mut result = String::with_capacity(s.len() + line_count * count * 4);
    for l in s.lines() {
        result.push_str(std::iter::repeat("    ").take(count).collect::<String>().as_ref());
        result.push_str(l);
        result.push('\n');
    }
    result
}
