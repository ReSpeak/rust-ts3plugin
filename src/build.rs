use std::env;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;

type Map<K, V> = BTreeMap<K, V>;

//TODO replace &'a str with Cow<'a, str>
#[derive(Default, Clone)]
struct Property<'a> {
    name: &'a str,
    type_s: &'a str,
    documentation: &'a str,
    initialize: bool,
    /// Use a fixed function
    method_name: Option<&'a str>,
    /// The name that is used to initialize this value: enum_name::value_name
    enum_name: &'a str,
    value_name: Option<&'a str>,
    /// Map type_s → used function
    functions: Map<&'a str, &'a str>,
    /// Types that are transmutable, the standard type that is taken is int
    transmutable: Vec<&'a str>,
    /// Arguments passed to the function
    default_args: &'a str,
}

impl<'a> Property<'a> {
    fn create_attribute(&self) -> String {
        let mut s = String::new();
        if !self.documentation.is_empty() {
            s.push_str(self.documentation.lines()
                .map(|l| format!("/// {}\n", l)).collect::<String>().as_str());
        }
        s.push_str(format!("{}: {},\n", self.name, self.type_s).as_str());
        s
    }

    fn create_getter(&self) -> String {
        let is_ref_type = ["String", "Permissions"].contains(&self.type_s) || self.type_s.starts_with("Option<");
        let mut s = String::new();
        s.push_str(format!("pub fn get_{}(&self) -> ", self.name).as_str());
        if is_ref_type {
            s.push('&');
        }
        s.push_str(format!("{} {{\n", self.type_s).as_str());
        let mut body = String::new();
        s.push_str(indent("", 1).as_str());
        if is_ref_type {
            body.push('&');
        }
        body.push_str(format!("self.{}\n", self.name).as_str());
        s.push_str(format!("{}}}\n", indent(body, 1)).as_str());
        s
    }

    fn create_initialisation(&self) -> String {
        if !self.initialize {
            return String::new();
        }
        let value_name = self.value_name.map(|s| s.to_string()).unwrap_or(to_pascal_case(self.name));
        let mut s = String::new();
        // Ignore unknown types
        if let Some(function) = self.method_name {
            // Special defined function
            s.push_str(format!("try!(Self::{}({}{}::{}))", function, self.default_args,
                self.enum_name, value_name).as_str());
        } else if let Some(function) = self.functions.get(self.type_s) {
            // From function list
            s.push_str(format!("try!(Self::{}({}{}::{}))", function, self.default_args,
                self.enum_name, value_name).as_str());
        } else if self.transmutable.contains(&self.type_s) {
            // Try to get an int
            for t in &["i32", "u64"] {
                if let Some(function) = self.functions.get(t) {
                    s.push_str(format!("unsafe {{ transmute(try!(Self::{}({}{}::{}))) }}", function, self.default_args,
                        self.enum_name, value_name).as_str());
                    break;
                }
            }
        } else {
            match self.type_s {
                "Duration" => {
                    // Try to get an u64
                    let function = if let Some(f) = self.functions.get("u64") {
                        f
                    } else if let Some(f) = self.functions.get("i32") {
                        f
                    } else {
                        "get_property_as_int"
                    };
                    s.push_str(format!("Duration::seconds(try!(Self::{}({}{}::{})) as i64)",
                        function, self.default_args, self.enum_name, value_name).as_str())
                },
                "bool" => {
                    for t in &["i32", "u64"] {
                        if let Some(function) = self.functions.get(t) {
                            s.push_str(format!("try!(Self::{}({}{}::{})) != 0", function,
                                self.default_args, self.enum_name, value_name).as_str());
                            break;
                        }
                    }
                }
                _ => if self.type_s.starts_with("Option<") {
                    s.push_str("None")
                }
            }
        }
        s
    }
}

#[derive(Default, Clone)]
struct PropertyBuilder<'a> {
    name: &'a str,
    type_s: &'a str,
    documentation: &'a str,
    initialize: bool,
    method_name: Option<&'a str>,
    enum_name: &'a str,
    value_name: Option<&'a str>,
    functions: Map<&'a str, &'a str>,
    transmutable: Vec<&'a str>,
    default_args: &'a str,
}

impl<'a> PropertyBuilder<'a> {
    fn new() -> PropertyBuilder<'a> {
        let mut result = Self::default();
        result.initialize = true;
        result
    }

    fn name(&self, name: &'a str) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.name = name;
        res
    }

    fn type_s(&self, type_s: &'a str) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.type_s = type_s;
        res
    }

    fn documentation(&self, documentation: &'a str) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.documentation = documentation;
        res
    }

    fn initialize(&self, initialize: bool) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.initialize = initialize;
        res
    }

    fn method_name(&self, method_name: &'a str) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.method_name = Some(method_name);
        res
    }

    fn enum_name(&self, enum_name: &'a str) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.enum_name = enum_name;
        res
    }

    fn value_name(&self, value_name: &'a str) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.value_name = Some(value_name);
        res
    }

    fn functions(&self, functions: Map<&'a str, &'a str>) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.functions = functions;
        res
    }

    fn transmutable(&self, transmutable: Vec<&'a str>) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.transmutable = transmutable;
        res
    }

    fn default_args(&self, default_args: &'a str) -> PropertyBuilder<'a> {
        let mut res = self.clone();
        res.default_args = default_args;
        res
    }

    fn finalize(&self) -> Property<'a> {
        Property {
            name: self.name,
            type_s: self.type_s,
            documentation: self.documentation,
            initialize: self.initialize,
            method_name: self.method_name,
            enum_name: self.enum_name,
            value_name: self.value_name,
            functions: self.functions.clone(),
            transmutable: self.transmutable.clone(),
            default_args: self.default_args,
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
    /// Arguments that are taken by the constructor
    constructor_args: &'a str,
}

#[derive(Default)]
struct StructBuilder<'a> {
    name: &'a str,
    documentation: &'a str,
    properties: Vec<Property<'a>>,
    extra_attributes: &'a str,
    extra_initialisation: &'a str,
    extra_creation: &'a str,
    constructor_args: &'a str,
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

    fn constructor_args(&mut self, constructor_args: &'a str) -> &mut StructBuilder<'a> {
        self.constructor_args = constructor_args;
        self
    }

    fn finalize(&self) -> Struct<'a> {
        Struct {
            name: self.name,
            documentation: self.documentation,
            // Move the contents of the properties
            properties: self.properties.clone(),
            extra_attributes: self.extra_attributes,
            extra_initialisation: self.extra_initialisation,
            extra_creation: self.extra_creation,
            constructor_args: self.constructor_args,
        }
    }
}

impl<'a> Struct<'a> {
    fn create_struct(&self) -> String {
        let mut s = String::new();
        for prop in &self.properties {
            s.push_str(prop.create_attribute().as_str());
        }
        let mut result = String::new();
        if !self.documentation.is_empty() {
            result.push_str(format!("/// {}\n", self.documentation).as_str());
        }
        result.push_str(format!("pub struct {} {{\n{}", self.name, indent(s, 1)).as_str());
        if !self.extra_attributes.is_empty() {
            result.push_str(format!("\n{}", indent(self.extra_attributes, 1)).as_str());
        }
        result.push_str("}\n\n");
        result
    }

    fn create_impl(&self) -> String {
        let mut s = String::new();
        for prop in &self.properties {
            s.push_str(prop.create_getter().as_str());
        }
        let mut result = String::new();
        write!(result, "impl {} {{\n{}}}\n\n", self.name, indent(s, 1)).unwrap();
        result
    }

    /// struct_name: Name of the struct
    /// properties_name: Name of the properties enum
    /// args: Base args (id) to get properties
    fn create_constructor(&self) -> String {
        let mut inits = String::new();
        // Initialisation
        if !self.extra_initialisation.is_empty() {
            inits.push_str(self.extra_initialisation);
            inits.push('\n');
        }
        for prop in &self.properties {
            let p = prop.create_initialisation();
            if !p.is_empty() {
                inits.push_str(format!("let {} = {};\n", prop.name, p).as_str());
            }
        }
        // Creation
        let mut creats = String::new();
        for prop in &self.properties {
            creats.push_str(format!("{0}: {0},\n", prop.name).as_str());
        }
        if !self.extra_creation.is_empty() {
            creats.push('\n');
            creats.push_str(self.extra_creation);
        }

        let mut result = String::new();
        write!(result, "impl {0} {{\n\
               \tfn new({1}) -> Result<{0}, Error> {{\n\
               {2}\n\
               \t\tOk({0} {{\n\
               {3}\
               \t\t}})\n\
               \t}}\n}}\n\n", self.name, self.constructor_args, indent(inits, 2), indent(creats, 3)).unwrap();
        result
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
    let transmutable = vec!["CodecEncryptionMode"];

    let builder = PropertyBuilder::new()
        .functions(default_functions)
        .transmutable(transmutable)
        .default_args("id, ")
        .enum_name("VirtualServerProperties");
    let builder_string = builder.type_s("String");
    let builder_i32 = builder.type_s("i32");
    // Optional server data
    let optional_server_data = StructBuilder::new().name("OptionalServerData")
        .documentation("Server properties that have to be fetched explicitely")
        .properties(vec![
            builder_string.name("welcome_message").finalize(),
            builder_i32.name("max_clients").finalize(),
            builder_i32.name("clients_online").finalize(),
            builder_i32.name("channels_online").finalize(),
            builder_i32.name("client_connections").finalize(),
            builder_i32.name("query_client_connections").finalize(),
            builder_i32.name("query_clients_online").finalize(),
            builder.name("uptime").type_s("Duration").finalize(),
            builder.name("password").type_s("bool").finalize(),
            builder_i32.name("max_download_total_bandwith").finalize(),
            builder_i32.name("max_upload_total_bandwith").finalize(),
            builder_i32.name("download_quota").finalize(),
            builder_i32.name("upload_quota").finalize(),
            builder_i32.name("month_bytes_downloaded").finalize(),
            builder_i32.name("month_bytes_uploaded").finalize(),
            builder_i32.name("total_bytes_downloaded").finalize(),
            builder_i32.name("total_bytes_uploaded").finalize(),
            builder_i32.name("complain_autoban_count").finalize(),
            builder.name("complain_autoban_time").type_s("Duration").finalize(),
            builder.name("complain_remove_time").type_s("Duration").finalize(),
            builder_i32.name("min_clients_in_channel_before_forced_silence").finalize(),
            builder_i32.name("antiflood_points_tick_reduce").finalize(),
            builder_i32.name("antiflood_points_needed_command_block").finalize(),
            builder_i32.name("antiflood_points_needed_ip_block").finalize(),
            builder_i32.name("port").finalize(),
            builder.name("autostart").type_s("bool").finalize(),
            builder_i32.name("machine_id").finalize(),
            builder_i32.name("needed_identity_security_level").finalize(),
            builder.name("log_client").type_s("bool").finalize(),
            builder.name("log_query").type_s("bool").finalize(),
            builder.name("log_channel").type_s("bool").finalize(),
            builder.name("log_permissions").type_s("bool").finalize(),
            builder.name("log_server").type_s("bool").finalize(),
            builder.name("log_filetransfer").type_s("bool").finalize(),
            builder_string.name("min_client_version").finalize(),
            builder_i32.name("total_packetloss_speech").finalize(),
            builder_i32.name("total_packetloss_keepalive").finalize(),
            builder_i32.name("total_packetloss_control").finalize(),
            builder_i32.name("total_packetloss_total").finalize(),
            builder_i32.name("total_ping").finalize(),
            builder.name("weblist_enabled").type_s("bool").finalize(),
        ]).finalize();
    // Outdated server data
    let outdated_server_data = StructBuilder::new().name("OutdatedServerData")
        .documentation("Server properties that are available at the start but not updated")
        .properties(vec![
            builder_string.name("hostmessage").finalize(),
            builder.name("hostmessage_mode").type_s("HostmessageMode").finalize(),
        ]).finalize();
    // The real server data
    let server = StructBuilder::new().name("Server")
        .constructor_args("id: ServerId")
        .extra_attributes("\
            visible_connections: Map<ConnectionId, Connection>,\n\
            channels: Map<ChannelId, Channel>,\n\
            outdated_data: OutdatedServerData,\n\
            optional_data: Option<OptionalServerData>,\n")
        .extra_initialisation("\
            let uid = try!(Self::get_property_as_string(id, VirtualServerProperties::UniqueIdentifier));\n\
            let own_connection_id = try!(Self::query_own_connection_id(id));\n\
            // These attributes are not in the main struct\n\
            let hostbanner_mode = unsafe { transmute(try!(Self::get_property_as_int(id, VirtualServerProperties::HostbannerMode))) };\n\
            let hostmessage_mode = unsafe { transmute(try!(Self::get_property_as_int(id, VirtualServerProperties::HostmessageMode))) };\n\
            let hostmessage = try!(Self::get_property_as_string(id, VirtualServerProperties::Hostmessage));\n\n\

            //TODO\n\
            let created = UTC::now();\n\
            let default_server_group = Permissions;\n\
            let default_channel_group = Permissions;\n\
            let default_channel_admin_group = Permissions;\n\
            // Query currently visible connections on this server\n\
            let visible_connections = Self::query_connections(id);\n\
            // Query channels on this server\n\
            let channels = Self::query_channels(id);\n")
        .extra_creation("\
            uid: uid,\n\
            visible_connections: visible_connections,\n\
            channels: channels,\n\
            outdated_data: OutdatedServerData {\n\
                \thostmessage: hostmessage,\n\
                \thostmessage_mode: hostmessage_mode,\n\
            },\n\
            optional_data: None,\n")
        .properties(vec![
            builder.name("id").type_s("ServerId").finalize(),
            builder_string.name("uid").finalize(),
            builder.name("own_connection_id").type_s("ConnectionId").finalize(),
            builder_string.name("name").finalize(),
            builder_string.name("phonetic_name").value_name("NamePhonetic").finalize(),
            builder_string.name("platform").finalize(),
            builder_string.name("version").finalize(),
            builder.name("created").type_s("DateTime<UTC>").finalize(),
            builder.name("codec_encryption_mode").type_s("CodecEncryptionMode").finalize(),
            builder.name("default_server_group").type_s("Permissions").finalize(),
            builder.name("default_channel_group").type_s("Permissions").finalize(),
            builder.name("default_channel_admin_group").type_s("Permissions").finalize(),
            builder_string.name("hostbanner_url").finalize(),
            builder_string.name("hostbanner_gfx_url").finalize(),
            builder.name("hostbanner_gfx_interval").type_s("Duration").finalize(),
            builder.name("hostbanner_mode").type_s("HostbannerMode").finalize(),
            builder_i32.name("priority_speaker_dimm_modificator").finalize(),
            builder_string.name("hostbutton_tooltip").finalize(),
            builder_string.name("hostbutton_url").finalize(),
            builder_string.name("hostbutton_gfx_url").finalize(),
            builder_i32.name("icon_id").finalize(),
            builder_i32.name("reserved_slots").finalize(),
            builder.name("ask_for_privilegekey").type_s("bool").finalize(),
            builder.name("channel_temp_delete_delay_default").type_s("Duration").finalize(),
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
}\n\n".as_bytes()).unwrap();

    // Constructors
    //f.write_all(optional_server_data.create_constructor("id: ServerId", &default_functions, "id, ", "VirtualServerProperties").as_bytes()).unwrap();

    // Initialize variables and ignore uid because it has another name
    {
        let mut ss = vec![server.properties[0].clone()];
        ss.extend_from_slice(&server.properties[2..]);
        let s = Struct {
            properties: ss,
            ..server
        };
        f.write_all(s.create_constructor().as_bytes()).unwrap();
    }
}

fn create_channel(f: &mut Write) {
    // Map types to functions that will get that type
    let default_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_property_as_int");
        m.insert("u64", "get_property_as_uint64");
        m.insert("String", "get_property_as_string");
        m
    };
    let transmutable = vec!["CodecType"];

    let builder = PropertyBuilder::new()
        .functions(default_functions)
        .transmutable(transmutable)
        .default_args("server_id, id, ")
        .enum_name("ChannelProperties");
    let builder_string = builder.type_s("String");
    let builder_i32 = builder.type_s("i32");
    let builder_bool = builder.type_s("bool");

    // Optional channel data
    let optional_channel_data = StructBuilder::new().name("OptionalChannelData")
        .documentation("Channel properties that have to be fetched explicitely")
        .properties(vec![
            builder_string.name("description").finalize(),
        ]).finalize();
    // The real channel data
    let channel = StructBuilder::new().name("Channel")
        .constructor_args("server_id: ServerId, id: ChannelId")
        .extra_initialisation("\
            let parent_channel_id = try!(Self::query_parent_channel_id(server_id, id));\n")
        .properties(vec![
            builder.name("id").type_s("ChannelId").finalize(),
            builder.name("server_id").type_s("ServerId").finalize(),
            builder.name("parent_channel_id").type_s("ChannelId")
                .documentation("The id of the parent channel, 0 if there is no parent channel").finalize(),
            builder_string.name("name").finalize(),
            builder_string.name("topic").finalize(),
            builder.name("codec").type_s("CodecType").finalize(),
            builder_i32.name("codec_quality").finalize(),
            builder_i32.name("max_clients").finalize(),
            builder_i32.name("max_family_clients").finalize(),
            builder_i32.name("order").finalize(),
            builder_bool.name("permanent").value_name("FlagPermanent").finalize(),
            builder_bool.name("semi_permanent").value_name("FlagSemiPermanent").finalize(),
            builder_bool.name("default").value_name("FlagDefault").finalize(),
            builder_bool.name("password").value_name("FlagPassword").finalize(),
            builder_i32.name("codec_latency_factor").finalize(),
            builder_bool.name("codec_is_unencrypted").finalize(),
            builder_i32.name("delete_delay").finalize(),
            builder_bool.name("max_clients_unlimited").value_name("FlagMaxClientsUnlimited").finalize(),
            builder_bool.name("max_family_clients_unlimited").value_name("FlagMaxFamilyClientsUnlimited").finalize(),
            // Clone so we can change the documentation
            builder_bool.name("subscribed").value_name("FlagAreSubscribed")
                .documentation("If we are subscribed to this channel").finalize(),
            builder_i32.name("needed_talk_power").finalize(),
            builder_i32.name("forced_silence").finalize(),
            builder_string.name("phonetic_name").value_name("NamePhonetic").finalize(),
            builder_i32.name("icon_id").finalize(),
            builder_bool.name("private").value_name("FlagPrivate").finalize(),

            builder.name("optional_data").type_s("Option<OptionalChannelData>").finalize(),
        ]).finalize();

    // Structs
    f.write_all(optional_channel_data.create_struct().as_bytes()).unwrap();
    f.write_all(channel.create_struct().as_bytes()).unwrap();

    // Implementations
    f.write_all(optional_channel_data.create_impl().as_bytes()).unwrap();
    f.write_all(channel.create_impl().as_bytes()).unwrap();
    f.write_all(channel.create_constructor().as_bytes()).unwrap();
}

fn create_connection(f: &mut Write) {
    // Map types to functions that will get that type
    let default_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_connection_property_as_uint64");
        m.insert("u64", "get_connection_property_as_uint64");
        m.insert("String", "get_connection_property_as_string");
        m
    };
    let client_functions = {
        let mut m = Map::new();
        m.insert("i32", "get_client_property_as_int");
        m.insert("String", "get_client_property_as_string");
        m
    };
    let transmutable = vec!["InputDeactivationStatus", "TalkStatus",
        "MuteInputStatus", "MuteOutputStatus", "HardwareInputStatus",
        "HardwareOutputStatus", "AwayStatus"];

    let builder = PropertyBuilder::new()
        .functions(default_functions)
        .transmutable(transmutable)
        .default_args("server_id, id, ")
        .enum_name("ConnectionProperties");
    let builder_string = builder.type_s("String");
    let builder_i32 = builder.type_s("i32");
    let builder_u64 = builder.type_s("u64");

    let client_b = builder.enum_name("ClientProperties")
        .functions(client_functions);
    let client_b_string = client_b.type_s("String");
    let client_b_i32 = client_b.type_s("i32");
    // Own connection data
    let own_connection_data = StructBuilder::new().name("OwnConnectionData")
        .properties(vec![
            builder_string.name("server_ip").finalize(),
            builder.name("server_port").type_s("u16").finalize(),
            builder.name("input_deactivated").type_s("InputDeactivationStatus").finalize(),
            builder.name("default_channel").type_s("ChannelId").finalize(),
            builder_string.name("default_token").finalize(),
        ]).finalize();
    // Serverquery connection data
    let serverquery_connection_data = StructBuilder::new().name("ServerqueryConnectionData")
        .properties(vec![
            builder_string.name("name").finalize(),
            builder_string.name("password").finalize(),
        ]).finalize();
    // Optional connection data
    let optional_connection_data = StructBuilder::new().name("OptionalConnectionData")
        .extra_initialisation("\
            //let client_port = try!(Self::get_connection_property_as_uint64(server_id, id, ConnectionProperties::ClientPort)) as u16;\n")
        .properties(vec![
            builder_string.name("version").finalize(),
            builder_string.name("platform").finalize(),
            builder.name("created").type_s("DateTime<UTC>").finalize(),
            builder.name("last_connected").type_s("DateTime<UTC>").finalize(),
            builder_i32.name("total_connection").finalize(),
            builder.name("ping").type_s("Duration").finalize(),
            builder.name("ping_deviation").type_s("Duration").finalize(),
            builder.name("connected_time").type_s("Duration").finalize(),
            builder.name("idle_time").type_s("Duration").finalize(),
            builder_string.name("client_ip").finalize(),
            builder.name("client_port").type_s("u16").finalize(),
            // Network
            builder_u64.name("packets_sent_speech").finalize(),
            builder_u64.name("packets_sent_keepalive").finalize(),
            builder_u64.name("packets_sent_control").finalize(),
            builder_u64.name("packets_sent_total").finalize(),
            builder_u64.name("bytes_sent_speech").finalize(),
            builder_u64.name("bytes_sent_keepalive").finalize(),
            builder_u64.name("bytes_sent_control").finalize(),
            builder_u64.name("bytes_sent_total").finalize(),
            builder_u64.name("packets_received_speech").finalize(),
            builder_u64.name("packets_received_keepalive").finalize(),
            builder_u64.name("packets_received_control").finalize(),
            builder_u64.name("packets_received_total").finalize(),
            builder_u64.name("bytes_received_speech").finalize(),
            builder_u64.name("bytes_received_keepalive").finalize(),
            builder_u64.name("bytes_received_control").finalize(),
            builder_u64.name("bytes_received_total").finalize(),
            builder_u64.name("packetloss_speech").finalize(),
            builder_u64.name("packetloss_keepalive").finalize(),
            builder_u64.name("packetloss_control").finalize(),
            builder_u64.name("packetloss_total").finalize(),
            //TODO much more...
            // End network
            builder_i32.name("month_bytes_uploaded").finalize(),
            builder_i32.name("month_bytes_downloaded").finalize(),
            builder_i32.name("total_bytes_uploaded").finalize(),
            builder_i32.name("total_bytes_downloaded").finalize(),

            client_b_string.name("default_channel_password").finalize(),
            client_b_string.name("server_password").finalize(),
            client_b.name("is_muted").type_s("bool")
                .documentation("If the client is locally muted.").finalize(),
            client_b.name("is_recording").type_s("bool").finalize(),
            client_b_i32.name("volume_modificator").finalize(),
            client_b.name("version_sign").type_s("bool").finalize(),
            client_b.name("away").type_s("AwayStatus").finalize(),
            client_b_string.name("away_message").finalize(),
            client_b.name("flag_avatar").type_s("bool").finalize(),
            client_b_string.name("description").finalize(),
            client_b.name("talker").type_s("bool").value_name("IsTalker").finalize(),
            client_b.name("priority_speaker").type_s("bool").value_name("IsPrioritySpeaker").finalize(),
            client_b.name("unread_messages").type_s("bool").finalize(),
            client_b_i32.name("needed_serverquery_view_power").finalize(),
            client_b_i32.name("icon_id").finalize(),
            client_b.name("is_channel_commander").type_s("bool").finalize(),
            client_b_string.name("country").finalize(),
            client_b_string.name("badges").finalize(),
        ]).finalize();
    // The real connection data
    let connection = StructBuilder::new().name("Connection")
        .constructor_args("server_id: ServerId, id: ConnectionId")
        .extra_initialisation("\
            let channel_id = try!(Self::query_channel_id(server_id, id));\n")
        .properties(vec![
            builder.name("id").type_s("ConnectionId").finalize(),
            builder.name("server_id").type_s("ServerId").finalize(),
            builder.name("channel_id").type_s("ChannelId").finalize(),
            // ClientProperties
            client_b_string.name("uid").value_name("UniqueIdentifier").finalize(),
            client_b_string.name("name").value_name("Nickname").finalize(),
            client_b.name("talking").type_s("TalkStatus").value_name("FlagTalking").finalize(),
            client_b.name("input_muted").type_s("MuteInputStatus").finalize(),
            client_b.name("output_muted").type_s("MuteOutputStatus").finalize(),
            client_b.name("output_only_muted").type_s("MuteOutputStatus").finalize(),
            client_b.name("input_hardware").type_s("HardwareInputStatus").finalize(),
            client_b.name("output_hardware").type_s("HardwareOutputStatus").finalize(),
            client_b_string.name("phonetic_name").value_name("NicknamePhonetic").finalize(),
            client_b.name("database_id").type_s("Option<Permissions>")
                .documentation("Only valid data if we have the appropriate permissions.").finalize(),
            client_b.name("channel_group_id").type_s("Option<Permissions>").finalize(),
            client_b.name("server_groups").type_s("Option<Vec<Permissions>>").finalize(),
            client_b.name("talk_power").type_s("Option<i32>").finalize(),
            // When this client requested to talk
            client_b.name("talk_request").type_s("Option<DateTime<UTC>>").finalize(),
            client_b.name("talk_request_message").type_s("Option<String>").finalize(),

            client_b.name("channel_group_inherited_channel_id").type_s("Option<ChannelId>")
                .documentation("The channel that sets the current channel id of this client.").finalize(),
            client_b.name("own_data").type_s("Option<OwnConnectionData>")
                .documentation("Only set for oneself").finalize(),
            client_b.name("serverquery_data").type_s("Option<ServerqueryConnectionData>")
                .documentation("Only available for serverqueries").finalize(),
            client_b.name("optional_data").type_s("Option<OptionalConnectionData>").finalize(),
    ]).finalize();

    // Structs
    f.write_all(own_connection_data.create_struct().as_bytes()).unwrap();
    f.write_all(serverquery_connection_data.create_struct().as_bytes()).unwrap();
    f.write_all(optional_connection_data.create_struct().as_bytes()).unwrap();
    f.write_all(connection.create_struct().as_bytes()).unwrap();

    // Implementations
    f.write_all(own_connection_data.create_impl().as_bytes()).unwrap();
    f.write_all(serverquery_connection_data.create_impl().as_bytes()).unwrap();
    f.write_all(optional_connection_data.create_impl().as_bytes()).unwrap();
    f.write_all(connection.create_impl().as_bytes()).unwrap();

    // Constructors
    //f.write_all(own_connection_data.create_constructor("id: ClientId", &default_functions, "id, ", "ConnectionProperties").as_bytes()).unwrap();
    f.write_all(connection.create_constructor().as_bytes()).unwrap();
}

/// Build parts of lib.rs as most of the structs are very repetitive
fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rerun-if-changed={}/src/build.rs", manifest_dir);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("structs.rs");
    let mut f = File::create(&dest_path).unwrap();

    create_server(&mut f);
    create_channel(&mut f);
    create_connection(&mut f);
}

fn to_pascal_case<S: AsRef<str>>(text: S) -> String {
    let sref = text.as_ref();
    let mut s = String::with_capacity(sref.len());
    let mut uppercase = true;
    for c in sref.chars() {
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
fn indent<S: AsRef<str>>(s: S, count: usize) -> String {
    let sref = s.as_ref();
    let line_count = sref.lines().count();
    let mut result = String::with_capacity(sref.len() + line_count * count * 4);
    for l in sref.lines() {
        result.push_str(std::iter::repeat("    ").take(count).collect::<String>().as_str());
        result.push_str(l);
        result.push('\n');
    }
    result
}
