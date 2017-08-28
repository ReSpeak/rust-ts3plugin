use ::*;

pub fn create(f: &mut Write) {
	// Map types to functions that will get that type
	let default_functions = {
		let mut m = Map::new();
		m.insert("i32", "ServerData::get_property_as_int");
		m.insert("String", "ServerData::get_property_as_string");
		m
	};
	let transmutable = vec!["CodecEncryptionMode", "HostbannerMode"];

	let builder = PropertyBuilder::new()
		.functions(default_functions)
		.transmutable(transmutable)
		.default_args("id, ")
		.default_args_update("self.id, ")
		.enum_name("VirtualServerProperties");
	let builder_string = builder.type_s("String");
	let builder_i32 = builder.type_s("i32");
	// Optional server data
	let optional_server_data = StructBuilder::new().name("OptionalServerData")
		.documentation("Server properties that have to be fetched explicitely")
		.constructor_args("id: ServerId")
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
	let builder = builder.public(false);
	let builder_string = builder_string.public(false);
	let builder_i32 = builder_i32.public(false);
	let server = StructBuilder::new()
		.name("ServerData")
		.api_name("Server")
		.public(false)
		.constructor_args("id: ServerId")
		.extra_attributes("\
			outdated_data: OutdatedServerData,\n")
		.extra_initialisation("\
			// These attributes are not in the main struct\n\
			let hostmessage_mode = Self::get_property_as_int(id, VirtualServerProperties::HostmessageMode).map(|p| unsafe { transmute(p) });\n\
			let hostmessage = Self::get_property_as_string(id, VirtualServerProperties::Hostmessage);\n")
		.extra_creation("\
			outdated_data: OutdatedServerData {\n\
				\thostmessage: hostmessage,\n\
				\thostmessage_mode: hostmessage_mode,\n\
			},\n")
		.properties(vec![
			builder.name("id").type_s("ServerId").result(false).initialisation("id").should_update(false).api_getter(false).finalize(),
			builder_string.name("uid").value_name("UniqueIdentifier").finalize(),
			builder.name("own_connection_id").type_s("ConnectionId").update("Self::query_own_connection_id(self.id)").api_getter(false).finalize(),
			builder_string.name("name").finalize(),
			builder_string.name("phonetic_name").value_name("NamePhonetic").finalize(),
			builder_string.name("platform").finalize(),
			builder_string.name("version").finalize(),
			// FIXME Always zero when queried as string, int or uint64
			builder.name("created").type_s("DateTime<Utc>").finalize(),
			builder.name("codec_encryption_mode").type_s("CodecEncryptionMode").finalize(),
			// TODO Update
			builder.name("default_server_group").type_s("Permissions").update("Ok(Permissions)").finalize(),
			builder.name("default_channel_group").type_s("Permissions").update("Ok(Permissions)").finalize(),
			builder.name("default_channel_admin_group").type_s("Permissions").update("Ok(Permissions)").finalize(),
			// End TODO Update
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
			builder.name("visible_connections").type_s("Map<ConnectionId, ConnectionData>").result(false).initialisation("Map::new()").update("Self::query_connections(self.id)").api_getter(false).finalize(),
			builder.name("channels").type_s("Map<ChannelId, ChannelData>").update("Self::query_channels(self.id)").api_getter(false).finalize(),
			builder.name("optional_data").type_s("OptionalServerData").result(false).initialisation("OptionalServerData::new(id)").update("OptionalServerData::new(self.id)").api_getter(false).finalize(),
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
impl ServerData {
	fn get_outdated_data(&self) -> &OutdatedServerData {
		&self.outdated_data
	}
}\n\n".as_bytes()).unwrap();
	f.write_all(server.create_update().as_bytes()).unwrap();
	f.write_all(server.create_api_impl().as_bytes()).unwrap();

	// Initialize variables
	f.write_all(server.create_constructor().as_bytes()).unwrap();
	f.write_all(optional_server_data.create_constructor().as_bytes()).unwrap();
}
