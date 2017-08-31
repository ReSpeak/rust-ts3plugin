use ::*;

pub fn create(f: &mut Write, tera: &Tera) {
	// Map types to functions that will get that type
	let default_functions = {
		let mut m = Map::new();
		m.insert("i32", "ConnectionData::get_connection_property_as_uint64");
		m.insert("u64", "ConnectionData::get_connection_property_as_uint64");
		m.insert("String", "ConnectionData::get_connection_property_as_string");
		m
	};
	let client_functions = {
		let mut m = Map::new();
		m.insert("i32", "ConnectionData::get_client_property_as_int");
		m.insert("String", "ConnectionData::get_client_property_as_string");
		m
	};
	let transmutable = vec!["InputDeactivationStatus", "TalkStatus",
		"MuteInputStatus", "MuteOutputStatus", "HardwareInputStatus",
		"HardwareOutputStatus", "AwayStatus"];

	let builder = PropertyBuilder::new()
		.functions(default_functions)
		.transmutable(transmutable)
		.default_args("server_id, id, ")
		.default_args_update("self.server_id, self.id, ")
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
		.constructor_args("server_id: ServerId, id: ConnectionId")
		.do_update(false) //FIXME
		.properties(vec![
			builder.name("id").type_s("ConnectionId").result(false).api_getter(false).finalize(),
			builder.name("server_id").type_s("ServerId").result(false).api_getter(false).finalize(),
			builder_string.name("server_ip").finalize(),
			builder.name("server_port").type_s("u16").finalize(),
			builder.name("input_deactivated").type_s("InputDeactivationStatus").finalize(),
			builder.name("default_channel").type_s("ChannelId").finalize(),
			builder_string.name("default_token").finalize(),
		]).finalize();
	// Serverquery connection data
	let serverquery_connection_data = StructBuilder::new().name("ServerqueryConnectionData")
		.constructor_args("server_id: ServerId, id: ConnectionId")
		.do_update(false) //FIXME
		.properties(vec![
			builder.name("id").type_s("ConnectionId").result(false).api_getter(false).finalize(),
			builder.name("server_id").type_s("ServerId").result(false).api_getter(false).finalize(),
			builder_string.name("name").finalize(),
			builder_string.name("password").finalize(),
		]).finalize();
	// Optional connection data
	let optional_connection_data = StructBuilder::new().name("OptionalConnectionData")
		.constructor_args("server_id: ServerId, id: ConnectionId")
		.do_update(false) //FIXME
		.properties(vec![
			builder.name("id").type_s("ConnectionId").result(false).finalize(),
			builder.name("server_id").type_s("ServerId").result(false).api_getter(false).finalize(),
			builder_string.name("version").finalize(),
			builder_string.name("platform").finalize(),
			builder.name("created").type_s("DateTime<Utc>").finalize(),
			builder.name("last_connected").type_s("DateTime<Utc>").finalize(),
			builder_i32.name("total_connection").finalize(),
			builder.name("ping").type_s("Duration").finalize(),
			builder.name("ping_deviation").type_s("Duration").finalize(),
			builder.name("connected_time").type_s("Duration").finalize(),
			builder.name("idle_time").type_s("Duration").finalize(),
			builder_string.name("client_ip").finalize(),
			builder.name("client_port").type_s("u16").update("Self::get_connection_property_as_uint64(server_id, id, ConnectionProperties::ClientPort) as u16").finalize(),
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
			builder_u64.name("server_to_client_packetloss_speech").value_name("Server2ClientPacketlossSpeech").finalize(),
			builder_u64.name("server_to_client_packetloss_keepalive").value_name("Server2ClientPacketlossKeepalive").finalize(),
			builder_u64.name("server_to_client_packetloss_control").value_name("Server2ClientPacketlossControl").finalize(),
			builder_u64.name("server_to_client_packetloss_total").value_name("Server2ClientPacketlossTotal").finalize(),
			builder_u64.name("client_to_server_packetloss_speech").value_name("Client2ServerPacketlossSpeech").finalize(),
			builder_u64.name("client_to_server_packetloss_keepalive").value_name("Client2ServerPacketlossKeepalive").finalize(),
			builder_u64.name("client_to_server_packetloss_control").value_name("Client2ServerPacketlossControl").finalize(),
			builder_u64.name("client_to_server_packetloss_total").value_name("Client2ServerPacketlossTotal").finalize(),
			builder_u64.name("bandwidth_sent_last_second_speech").finalize(),
			builder_u64.name("bandwidth_sent_last_second_keepalive").finalize(),
			builder_u64.name("bandwidth_sent_last_second_control").finalize(),
			builder_u64.name("bandwidth_sent_last_second_total").finalize(),
			builder_u64.name("bandwidth_sent_last_minute_speech").finalize(),
			builder_u64.name("bandwidth_sent_last_minute_keepalive").finalize(),
			builder_u64.name("bandwidth_sent_last_minute_control").finalize(),
			builder_u64.name("bandwidth_sent_last_minute_total").finalize(),
			builder_u64.name("bandwidth_received_last_second_speech").finalize(),
			builder_u64.name("bandwidth_received_last_second_keepalive").finalize(),
			builder_u64.name("bandwidth_received_last_second_control").finalize(),
			builder_u64.name("bandwidth_received_last_second_total").finalize(),
			builder_u64.name("bandwidth_received_last_minute_speech").finalize(),
			builder_u64.name("bandwidth_received_last_minute_keepalive").finalize(),
			builder_u64.name("bandwidth_received_last_minute_control").finalize(),
			builder_u64.name("bandwidth_received_last_minute_total").finalize(),
			// End network
			builder_i32.name("month_bytes_uploaded").finalize(),
			builder_i32.name("month_bytes_downloaded").finalize(),
			builder_i32.name("total_bytes_uploaded").finalize(),
			builder_i32.name("total_bytes_downloaded").finalize(),

			client_b_string.name("default_channel_password").finalize(),
			client_b_string.name("server_password").finalize(),
			client_b.name("is_muted").type_s("bool")
				.documentation("If the client is locally muted.").finalize(),
			client_b_i32.name("volume_modificator").finalize(),
			client_b.name("version_sign").type_s("bool").finalize(),
			client_b.name("avatar").type_s("bool").value_name("FlagAvatar").finalize(),
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
	let builder = builder.public(false);
	let client_b = client_b.public(false);
	let client_b_string = client_b_string.public(false);
	let connection = StructBuilder::new()
		.name("ConnectionData")
		.api_name("Connection")
		.public(false)
		.do_api_impl(true)
		.constructor_args("server_id: ServerId, id: ConnectionId")
		.extra_initialisation("\
			let optional_data = OptionalConnectionData::new(server_id, id);\n\
			let own_data = None;\n\
			let serverquery_data = None;\n")
		.properties(vec![
			builder.name("id").type_s("ConnectionId").result(false).api_getter(false).finalize(),
			builder.name("server_id").type_s("ServerId").result(false).api_getter(false).finalize(),
			builder.name("channel_id").type_s("ChannelId").update("Self::query_channel_id(self.server_id, self.id)").api_getter(false).finalize(),
			// ClientProperties
			client_b_string.name("uid").value_name("UniqueIdentifier").finalize(),
			client_b_string.name("name").value_name("Nickname").finalize(),
			client_b.name("talking").type_s("TalkStatus").value_name("FlagTalking").finalize(),
			client_b.name("whispering").type_s("bool").update("Self::query_whispering(self.server_id, self.id)").finalize(),
			client_b.name("away").type_s("AwayStatus").finalize(),
			client_b_string.name("away_message").finalize(),
			client_b.name("input_muted").type_s("MuteInputStatus").finalize(),
			client_b.name("output_muted").type_s("MuteOutputStatus").finalize(),
			client_b.name("output_only_muted").type_s("MuteOutputStatus").finalize(),
			client_b.name("input_hardware").type_s("HardwareInputStatus").finalize(),
			client_b.name("output_hardware").type_s("HardwareOutputStatus").finalize(),
			client_b_string.name("phonetic_name").value_name("NicknamePhonetic").finalize(),
			client_b.name("recording").type_s("bool").value_name("IsRecording").finalize(),
			client_b.name("database_id").type_s("Permissions")
				.documentation("Only valid data if we have the appropriate permissions.").finalize(),
			client_b.name("channel_group_id").type_s("Permissions").finalize(),
			client_b.name("server_groups").type_s("Vec<Permissions>").finalize(),
			client_b.name("talk_power").type_s("i32").finalize(),
			// When this client requested to talk
			client_b.name("talk_request").type_s("DateTime<Utc>").finalize(),
			client_b.name("talk_request_message").type_s("String").value_name("TalkRequestMsg").finalize(),

			client_b.name("channel_group_inherited_channel_id").type_s("ChannelId").api_getter(false)
				.documentation("The channel that sets the current channel id of this client.").finalize(),
			client_b.name("own_data").type_s("Option<OwnConnectionData>").result(false).api_getter(false)
				.documentation("Only set for oneself").finalize(),
			client_b.name("serverquery_data").type_s("Option<ServerqueryConnectionData>").result(false).api_getter(false)
				.documentation("Only available for serverqueries").finalize(),
			client_b.name("optional_data").type_s("OptionalConnectionData").result(false).api_getter(false).finalize(),
	]).finalize();

	// Structs
	own_connection_data.create_struct(f, tera).unwrap();
	serverquery_connection_data.create_struct(f, tera).unwrap();
	optional_connection_data.create_struct(f, tera).unwrap();
	connection.create_struct(f, tera).unwrap();
}
