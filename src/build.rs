#![feature(slice_concat_ext)]

use std::env;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::slice::SliceConcatExt;

type Map<K, V> = BTreeMap<K, V>;

#[derive(Default, Clone)]
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

    fn finalize(&self) -> Property<'a> {
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

    fn finalize(&mut self) -> Struct<'a> {
        Struct {
            name: self.name,
            documentation: self.documentation,
            // Move the contents of the properties
            properties: self.properties.drain(..).collect(),
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
                write!(s, "/// {}\n", prop.documentation).unwrap();
            }
            write!(s, "{}: {},\n", prop.name, prop.type_s).unwrap();
        }
        let mut result = String::new();
        if !self.documentation.is_empty() {
            write!(result, "/// {}\n", self.documentation).unwrap();
        }
        write!(result, "pub struct {} {{\n{}", self.name, indent(s.as_ref(), 1)).unwrap();
        if !self.extra_attributes.is_empty() {
            write!(result, "\n{}", indent(self.extra_attributes, 1)).unwrap();
        }
        result.push_str("}\n\n");
        result
    }

    fn create_impl(&self) -> String {
        let mut s = String::new();
        for prop in &self.properties {
            let is_ref_type = ["String", "Permissions"].contains(&prop.type_s);
            write!(s, "pub fn get_{}(&self) -> ", prop.name).unwrap();
            if is_ref_type {
                s.write_str("&").unwrap();
            }
            write!(s, "{} {{\n    ", prop.type_s).unwrap();
            if is_ref_type {
                s.write_str("&").unwrap();
            }
            write!(s, "self.{}\n}}\n", prop.name).unwrap();
        }
        let mut result = String::new();
        write!(result, "impl {} {{\n{}}}\n\n", self.name, indent(s.as_ref(), 1)).unwrap();
        result
    }

    fn constructor_variables(&self) -> String {
        let mut s = String::new();
        //TODO
        /*for prop in &self.properties() {
            let mut p = String::new();
            // Ignore unknown types
            if let Some(function) = functions.get(var_type) {
                write!(ps, "try!({}::{}({}{}::{}));", struct_name, function, args, properties_name, to_pascal_case(name)).unwrap();
            } else {
                match var_type {
                    "Duration" => { write!(p, "Duration::seconds(try!({}::{}({}{}::{})) as i64);", struct_name, "get_property_as_int", args, properties_name, to_pascal_case(name)).unwrap(); }
                    "bool" => { write!(p, "try!({}::{}({}{}::{})) != 0;", struct_name, "get_property_as_int", args, properties_name, to_pascal_case(name)).unwrap(); }
                    _ => {}
                }
            }
            if !p.is_empty() {
                write!(s, "\n        let {} = {}", name, p).unwrap();
            }
        }*/
        s
    }
}

fn create_server(f: &mut Write) {
    // Map types to functions that will get that type
    let default_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_property_as_int");
        m.insert("String", "get_property_as_string");
        m
    };

    // Optional server data
    let optional_server_data = StructBuilder::new().name("OptionalServerData")
        .documentation("Server properties that have to be fetched explicitely")
        .properties(vec![
            PropertyBuilder::new().name("welcome_message").type_s("String").finalize(),
            PropertyBuilder::new().name("max_clients").type_s("i32").finalize(),
            PropertyBuilder::new().name("clients_online").type_s("i32").finalize(),
            PropertyBuilder::new().name("channels_online").type_s("i32").finalize(),
            PropertyBuilder::new().name("client_connections").type_s("i32").finalize(),
            PropertyBuilder::new().name("query_client_connections").type_s("i32").finalize(),
            PropertyBuilder::new().name("query_clients_online").type_s("i32").finalize(),
            PropertyBuilder::new().name("uptime").type_s("Duration").finalize(),
            PropertyBuilder::new().name("password").type_s("bool").finalize(),
            PropertyBuilder::new().name("max_download_total_bandwith").type_s("i32").finalize(),
            PropertyBuilder::new().name("max_upload_total_bandwith").type_s("i32").finalize(),
            PropertyBuilder::new().name("download_quota").type_s("i32").finalize(),
            PropertyBuilder::new().name("upload_quota").type_s("i32").finalize(),
            PropertyBuilder::new().name("month_bytes_downloaded").type_s("i32").finalize(),
            PropertyBuilder::new().name("month_bytes_uploaded").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_bytes_downloaded").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_bytes_uploaded").type_s("i32").finalize(),
            PropertyBuilder::new().name("complain_autoban_count").type_s("i32").finalize(),
            PropertyBuilder::new().name("complain_autoban_time").type_s("Duration").finalize(),
            PropertyBuilder::new().name("complain_remove_time").type_s("Duration").finalize(),
            PropertyBuilder::new().name("min_clients_in_channel_before_forced_silence").type_s("i32").finalize(),
            PropertyBuilder::new().name("antiflood_points_tick_reduce").type_s("i32").finalize(),
            PropertyBuilder::new().name("antiflood_points_needed_command_block").type_s("i32").finalize(),
            PropertyBuilder::new().name("antiflood_points_needed_ip_block").type_s("i32").finalize(),
            PropertyBuilder::new().name("port").type_s("i32").finalize(),
            PropertyBuilder::new().name("autostart").type_s("bool").finalize(),
            PropertyBuilder::new().name("machine_id").type_s("i32").finalize(),
            PropertyBuilder::new().name("needed_identity_security_level").type_s("i32").finalize(),
            PropertyBuilder::new().name("log_client").type_s("bool").finalize(),
            PropertyBuilder::new().name("log_query").type_s("bool").finalize(),
            PropertyBuilder::new().name("log_channel").type_s("bool").finalize(),
            PropertyBuilder::new().name("log_permissions").type_s("bool").finalize(),
            PropertyBuilder::new().name("log_server").type_s("bool").finalize(),
            PropertyBuilder::new().name("log_filetransfer").type_s("bool").finalize(),
            PropertyBuilder::new().name("min_client_version").type_s("String").finalize(),
            PropertyBuilder::new().name("total_packetloss_speech").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_packetloss_keepalive").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_packetloss_control").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_packetloss_total").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_ping").type_s("i32").finalize(),
            PropertyBuilder::new().name("weblist_enabled").type_s("bool").finalize(),
        ]).finalize();
    // Outdated server data
    let outdated_server_data = StructBuilder::new().name("OutdatedServerData")
        .documentation("Server properties that are available at the start but not updated")
        .properties(vec![
            PropertyBuilder::new().name("hostmessage").type_s("String").finalize(),
            PropertyBuilder::new().name("hostmessage_mode").type_s("HostmessageMode").finalize(),
        ]).finalize();
    // The real server data
    let server = StructBuilder::new().name("Server")
        .extra_attributes("\
            visible_connections: Map<ConnectionId, Connection>,\n\
            outdated_data: OutdatedServerData,\n\
            optional_data: Option<OptionalServerData>,\n")
        .properties(vec![
            PropertyBuilder::new().name("id").type_s("ServerId").finalize(),
            PropertyBuilder::new().name("uid").type_s("String").finalize(),
            PropertyBuilder::new().name("name").type_s("String").finalize(),
            PropertyBuilder::new().name("name_phonetic").type_s("String").finalize(),
            PropertyBuilder::new().name("platform").type_s("String").finalize(),
            PropertyBuilder::new().name("version").type_s("String").finalize(),
            PropertyBuilder::new().name("created").type_s("DateTime<UTC>").finalize(),
            PropertyBuilder::new().name("codec_encryption_mode").type_s("CodecEncryptionMode").finalize(),
            PropertyBuilder::new().name("default_server_group").type_s("Permissions").finalize(),
            PropertyBuilder::new().name("default_channel_group").type_s("Permissions").finalize(),
            PropertyBuilder::new().name("default_channel_admin_group").type_s("Permissions").finalize(),
            PropertyBuilder::new().name("hostbanner_url").type_s("String").finalize(),
            PropertyBuilder::new().name("hostbanner_gfx_url").type_s("String").finalize(),
            PropertyBuilder::new().name("hostbanner_gfx_interval").type_s("Duration").finalize(),
            PropertyBuilder::new().name("hostbanner_mode").type_s("HostbannerMode").finalize(),
            PropertyBuilder::new().name("priority_speaker_dimm_modificator").type_s("i32").finalize(),
            PropertyBuilder::new().name("hostbutton_tooltip").type_s("String").finalize(),
            PropertyBuilder::new().name("hostbutton_url").type_s("String").finalize(),
            PropertyBuilder::new().name("hostbutton_gfx_url").type_s("String").finalize(),
            PropertyBuilder::new().name("icon_id").type_s("i32").finalize(),
            PropertyBuilder::new().name("reserved_slots").type_s("i32").finalize(),
            PropertyBuilder::new().name("ask_for_privilegekey").type_s("bool").finalize(),
            PropertyBuilder::new().name("channel_temp_delete_delay_default").type_s("Duration").finalize(),
        ]).finalize();

    // Structs
    f.write_all(optional_server_data.create_struct().as_bytes()).unwrap();
    f.write_all(outdated_server_data.create_struct().as_bytes()).unwrap();
    f.write_all(server.create_struct().as_bytes()).unwrap();

    // Implementations
    f.write_all(optional_server_data.create_impl().as_bytes()).unwrap();
    f.write_all(outdated_server_data.create_impl().as_bytes()).unwrap();
    f.write_all(server.create_impl().as_bytes()).unwrap();
    f.write_all("\
impl Server {
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
        let mut ss = vec![server.properties[0].clone()];
        ss.extend_from_slice(&server.properties[2..]);
        let s = Struct {
            properties: ss,
            ..server
        };
        //f.write_all(constructor_variables("Server", "VirtualServerProperties", &default_functions, "id, ", &ss).as_bytes()).unwrap();
    }

    f.write_all("

        Ok(Server {".as_bytes()).unwrap();
    //f.write_all(constructor_creation(&server).as_bytes()).unwrap();

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

fn create_connection(f: &mut Write) {
    // Map types to functions that will get that type
    let default_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_property_as_int");
        m.insert("String", "get_property_as_string");
        m
    };

    // Own connection data
    let own_connection_data = StructBuilder::new().name("OwnConnectionData")
        .properties(vec![
            PropertyBuilder::new().name("server_ip").type_s("String").finalize(),
            PropertyBuilder::new().name("server_port").type_s("u16").finalize(),
            PropertyBuilder::new().name("input_deactivated").type_s("InputDeactivationStatus").finalize(),
            PropertyBuilder::new().name("default_channel").type_s("ChannelId").finalize(),
            PropertyBuilder::new().name("default_token").type_s("String").finalize(),
        ]).finalize();
    // Serverquery connection data
    let serverquery_connection_data = StructBuilder::new().name("ServerqueryConnectionData")
        .properties(vec![
            PropertyBuilder::new().name("name").type_s("String").finalize(),
            PropertyBuilder::new().name("password").type_s("String").finalize(),
        ]).finalize();
    // Optional connection data
    let optional_connection_data = StructBuilder::new().name("OptionalConnectionData")
        .properties(vec![
            PropertyBuilder::new().name("version").type_s("String").finalize(),
            PropertyBuilder::new().name("platform").type_s("String").finalize(),
            PropertyBuilder::new().name("created").type_s("DateTime<UTC>").finalize(),
            PropertyBuilder::new().name("last_connected").type_s("DateTime<UTC>").finalize(),
            PropertyBuilder::new().name("total_connection").type_s("i32").finalize(),
            PropertyBuilder::new().name("month_bytes_uploaded").type_s("i32").finalize(),
            PropertyBuilder::new().name("month_bytes_downloaded").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_bytes_uploaded").type_s("i32").finalize(),
            PropertyBuilder::new().name("total_bytes_downloaded").type_s("i32").finalize(),
        ]).finalize();
    // The real connection data
    let connection = StructBuilder::new().name("Connection")
        .extra_attributes("\
            /// The channel that sets the current channel id of this client.\n\
            channel_group_inherited_channel_id: Option<ChannelId>,\n\
            /// Only set for oneself\n\
            own_data: Option<OwnConnectionData>,\n\
            serverquery_data: Option<ServerqueryConnectionData>,\n\
            optional_data: Option<OptionalConnectionData>,\n")
        .properties(vec![
            PropertyBuilder::new().name("id").type_s("ConnectionId").finalize(),
            PropertyBuilder::new().name("server_id").type_s("ServerId").finalize(),
            PropertyBuilder::new().name("ping").type_s("Duration").finalize(),
            PropertyBuilder::new().name("ping_deciation").type_s("Duration").finalize(),
            PropertyBuilder::new().name("connected_time").type_s("Duration").finalize(),
            PropertyBuilder::new().name("idle_time").type_s("Duration").finalize(),
            PropertyBuilder::new().name("client_ip").type_s("String").finalize(),
            PropertyBuilder::new().name("client_port").type_s("String").finalize(),
            // Network
            PropertyBuilder::new().name("packets_sent_speech").type_s("u64").finalize(),
            PropertyBuilder::new().name("packets_sent_keepalive").type_s("u64").finalize(),
            PropertyBuilder::new().name("packets_sent_control").type_s("u64").finalize(),
            PropertyBuilder::new().name("packets_sent_total").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_sent_speech").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_sent_keepalive").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_sent_control").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_sent_total").type_s("u64").finalize(),
            PropertyBuilder::new().name("packets_received_speech").type_s("u64").finalize(),
            PropertyBuilder::new().name("packets_received_keepalive").type_s("u64").finalize(),
            PropertyBuilder::new().name("packets_received_control").type_s("u64").finalize(),
            PropertyBuilder::new().name("packets_received_total").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_received_speech").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_received_keepalive").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_received_control").type_s("u64").finalize(),
            PropertyBuilder::new().name("bytes_received_total").type_s("u64").finalize(),
            PropertyBuilder::new().name("packetloss_speech").type_s("u64").finalize(),
            PropertyBuilder::new().name("packetloss_keepalive").type_s("u64").finalize(),
            PropertyBuilder::new().name("packetloss_control").type_s("u64").finalize(),
            PropertyBuilder::new().name("packetloss_total").type_s("u64").finalize(),
            //TODO much more...
            // End network

            // ClientProperties
            PropertyBuilder::new().name("uid").type_s("String").finalize(),
            PropertyBuilder::new().name("name").type_s("String").finalize(),
            PropertyBuilder::new().name("talking").type_s("TalkStatus").finalize(),
            PropertyBuilder::new().name("input_muted").type_s("MuteInputStatus").finalize(),
            PropertyBuilder::new().name("output_muted").type_s("MuteOutputStatus").finalize(),
            PropertyBuilder::new().name("output_only_muted").type_s("MuteOutputStatus").finalize(),
            PropertyBuilder::new().name("input_hardware").type_s("HardwareInputStatus").finalize(),
            PropertyBuilder::new().name("output_hardware").type_s("HardwareOutputStatus").finalize(),
            PropertyBuilder::new().name("default_channel_password").type_s("String").finalize(),
            PropertyBuilder::new().name("server_password").type_s("String").finalize(),
            PropertyBuilder::new().name("is_muted").type_s("bool")
                .documentation("If the client is locally muted.").finalize(),
            PropertyBuilder::new().name("is_recording").type_s("bool").finalize(),
            PropertyBuilder::new().name("volume_modificator").type_s("i32").finalize(),
            PropertyBuilder::new().name("version_sign").type_s("String").finalize(),
            PropertyBuilder::new().name("away").type_s("AwayStatus").finalize(),
            PropertyBuilder::new().name("away_message").type_s("String").finalize(),
            PropertyBuilder::new().name("flag_avatar").type_s("bool").finalize(),
            PropertyBuilder::new().name("description").type_s("String").finalize(),
            PropertyBuilder::new().name("is_talker").type_s("bool").finalize(),
            PropertyBuilder::new().name("is_priority_speaker").type_s("bool").finalize(),
            PropertyBuilder::new().name("has_unread_messages").type_s("bool").finalize(),
            PropertyBuilder::new().name("phonetic_name").type_s("String").finalize(),
            PropertyBuilder::new().name("needed_serverquery_view_power").type_s("i32").finalize(),
            PropertyBuilder::new().name("icon_id").type_s("i32").finalize(),
            PropertyBuilder::new().name("is_channel_commander").type_s("bool").finalize(),
            PropertyBuilder::new().name("country").type_s("String").finalize(),
            PropertyBuilder::new().name("badges").type_s("String").finalize(),
            PropertyBuilder::new().name("database_id").type_s("Option<Permissions>")
                .documentation("Only valid data if we have the appropriate permissions.").finalize(),
            PropertyBuilder::new().name("channel_group_id").type_s("Option<Permissions>").finalize(),
            PropertyBuilder::new().name("server_groups").type_s("Option<Vec<Permissions>>").finalize(),
            PropertyBuilder::new().name("talk_power").type_s("Option<i32>").finalize(),
            // When this client requested to talk
            PropertyBuilder::new().name("talk_request").type_s("Option<DateTime<UTC>>").finalize(),
            PropertyBuilder::new().name("talk_request_message").type_s("Option<String>").finalize(),
    ]).finalize();

    // Structs
    f.write_all(own_connection_data.create_struct().as_bytes()).unwrap();
    f.write_all(serverquery_connection_data.create_struct().as_bytes()).unwrap();
    f.write_all(optional_connection_data.create_struct().as_bytes()).unwrap();
    f.write_all(connection.create_struct().as_bytes()).unwrap();
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
            write!(result, "\n        let {} = {}", name, s).unwrap();
        }
    }
    result
}

fn constructor_creation(data: &Vec<(&str, &str)>) -> String {
    let mut s = String::new();
    for &(name, _) in data {
        write!(s, "\n            {0}: {0},", name).unwrap();
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
