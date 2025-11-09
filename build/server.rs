use crate::{Map, PropertyBuilder, Struct, StructBuilder};

pub(crate) fn create() -> Vec<Struct<'static>> {
	// Map types to functions that will get that type
	let default_functions = {
		let mut m = Map::new();
		m.insert("i32", "ServerData::get_property_as_int");
		m.insert("u64", "ServerData::get_property_as_uint64");
		m.insert("String", "ServerData::get_property_as_string");
		m
	};
	let transmutable = vec!["CodecEncryptionMode", "HostbannerMode", "HostmessageMode"];

	let builder = PropertyBuilder::new()
		.functions(default_functions)
		.transmutable(transmutable)
		.default_args("id, ")
		.default_args_update("self.id, ")
		.enum_name("VirtualServerProperties");
	let builder_string = builder.type_s("String");
	let builder_i32 = builder.type_s("i32");

	let builder_r = builder.requested(true);
	let builder_string_r = builder_string.requested(true);
	let builder_i32_r = builder_i32.requested(true);

	let server = StructBuilder::new()
		.name("ServerData")
		.api_name("Server")
		.do_api_impl(true)
		.do_properties(true)
		.constructor_args("id: ServerId")
		.extra_property_list(vec![(
			"Connection<'a>".into(),
			"Connection".into(),
			"OwnConnection,".into(),
		)])
		.extra_properties(
			"\
			ServerProperty::Connection {\n\tproperty: ServerConnectionProperty::OwnConnection,\n\tdata: \
			 self.get_own_connection(),\n},",
		)
		.properties(vec![
			builder
				.name("id")
				.type_s("ServerId")
				.result(false)
				.initialisation("id")
				.should_update(false)
				.api_getter(false)
				.finalize(),
			builder_string.name("uid").value_name("UniqueIdentifier").finalize(),
			builder
				.name("own_connection_id")
				.type_s("ConnectionId")
				.update("Self::query_own_connection_id(self.id)")
				.api_getter(false)
				.finalize(),
			builder_string.name("name").finalize(),
			builder_string.name("phonetic_name").value_name("NamePhonetic").finalize(),
			builder_string.name("platform").finalize(),
			builder_string.name("version").finalize(),
			builder_string.name("nickname").finalize(),
			builder_string.name("accounting_token").finalize(),
			// TODO Always zero when queried as string, int or uint64
			builder.name("created").type_s("DateTime<Utc>").finalize(),
			builder.name("codec_encryption_mode").type_s("CodecEncryptionMode").finalize(),
			// TODO Update
			builder.name("default_server_group").type_s("ServerGroupId").finalize(),
			builder.name("default_channel_group").type_s("ChannelGroupId").finalize(),
			builder.name("default_channel_admin_group").type_s("ChannelGroupId").finalize(),
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
			builder
				.name("visible_connections")
				.type_s("Map<ConnectionId, ConnectionData>")
				.result(false)
				.initialisation("Map::new()")
				.update("Self::query_connections(self.id)")
				.api_getter(false)
				.finalize(),
			builder
				.name("channels")
				.type_s("Map<ChannelId, ChannelData>")
				.update("Self::query_channels(self.id)")
				.api_getter(false)
				.finalize(),
			// TODO requested
			builder_string_r.name("welcome_message").value_name("Welcomemessage").finalize(),
			builder_i32_r.name("max_clients").finalize(),
			builder_i32_r.name("clients_online").finalize(),
			builder_i32_r.name("channels_online").finalize(),
			builder_i32_r.name("client_connections").finalize(),
			builder_i32_r.name("query_client_connections").finalize(),
			builder_i32_r.name("query_clients_online").value_name("QueryclientsOnline").finalize(),
			builder_r.name("uptime").type_s("Duration").finalize(),
			builder_r.name("password").type_s("bool").finalize(),
			builder_i32_r.name("max_download_total_bandwidth").finalize(),
			builder_i32_r.name("max_upload_total_bandwidth").finalize(),
			builder_i32_r.name("download_quota").finalize(),
			builder_i32_r.name("upload_quota").finalize(),
			builder_i32_r.name("month_bytes_downloaded").finalize(),
			builder_i32_r.name("month_bytes_uploaded").finalize(),
			builder_i32_r.name("total_bytes_downloaded").finalize(),
			builder_i32_r.name("total_bytes_uploaded").finalize(),
			builder_i32_r.name("complain_autoban_count").finalize(),
			builder_r.name("complain_autoban_time").type_s("Duration").finalize(),
			builder_r.name("complain_remove_time").type_s("Duration").finalize(),
			builder_i32_r.name("min_clients_in_channel_before_forced_silence").finalize(),
			builder_i32_r.name("antiflood_points_tick_reduce").finalize(),
			builder_i32_r.name("antiflood_points_needed_command_block").finalize(),
			builder_i32_r.name("antiflood_points_needed_ip_block").finalize(),
			builder_i32_r.name("port").finalize(),
			builder_r.name("autostart").type_s("bool").finalize(),
			builder_i32_r.name("machine_id").finalize(),
			builder_i32_r.name("needed_identity_security_level").finalize(),
			builder_r.name("log_client").type_s("bool").finalize(),
			builder_r.name("log_query").type_s("bool").finalize(),
			builder_r.name("log_channel").type_s("bool").finalize(),
			builder_r.name("log_permissions").type_s("bool").finalize(),
			builder_r.name("log_server").type_s("bool").finalize(),
			builder_r.name("log_filetransfer").type_s("bool").finalize(),
			builder_string_r.name("min_client_version").finalize(),
			builder_i32_r.name("total_packetloss_speech").finalize(),
			builder_i32_r.name("total_packetloss_keepalive").finalize(),
			builder_i32_r.name("total_packetloss_control").finalize(),
			builder_i32_r.name("total_packetloss_total").finalize(),
			builder_i32_r.name("total_ping").finalize(),
			builder_r.name("weblist_enabled").type_s("bool").finalize(),
			builder_string
				.name("hostmessage")
				.documentation("Only set on connect and not updated")
				.finalize(),
			builder
				.name("hostmessage_mode")
				.type_s("HostmessageMode")
				.documentation("Only set on connect and not updated")
				.finalize(),
			builder_i32.name("antiflood_points_needed_plugin_block").finalize(),
		])
		.finalize();

	vec![server]
}
