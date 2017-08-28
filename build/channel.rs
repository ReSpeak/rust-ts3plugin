use ::*;

pub fn create(f: &mut Write) {
	// Map types to functions that will get that type
	let default_functions = {
		let mut m = Map::new();
		m.insert("i32", "ChannelData::get_property_as_int");
		m.insert("u64", "ChannelData::get_property_as_uint64");
		m.insert("String", "ChannelData::get_property_as_string");
		m
	};
	let transmutable = vec!["CodecType"];

	let builder = PropertyBuilder::new()
		.functions(default_functions)
		.transmutable(transmutable)
		.default_args("server_id, id, ")
		.default_args_update("self.server_id, self.id, ")
		.enum_name("ChannelProperties");
	let builder_string = builder.type_s("String");
	let builder_i32 = builder.type_s("i32");
	let builder_bool = builder.type_s("bool");
	let builder_optional_data = builder
		.default_args("server_id, channel_id, ")
		.default_args_update("self.server_id, self.channel_id, ");

	// Optional channel data
	let optional_channel_data = StructBuilder::new().name("OptionalChannelData")
		.documentation("Channel properties that have to be fetched explicitely")
		.constructor_args("server_id: ServerId, channel_id: ChannelId")
		.properties(vec![
			builder_optional_data.name("channel_id").type_s("ChannelId").result(false).finalize(),
			builder_optional_data.name("server_id").type_s("ServerId").result(false).finalize(),
			builder_optional_data.name("description").type_s("String").finalize(),
		]).finalize();
	// The real channel data
	let builder = builder.public(false);
	let builder_string = builder_string.public(false);
	let builder_i32 = builder_i32.public(false);
	let builder_bool = builder_bool.public(false);
	let channel = StructBuilder::new()
		.name("ChannelData")
		.api_name("Channel")
		.public(false)
		.constructor_args("server_id: ServerId, id: ChannelId")
		.properties(vec![
			builder.name("id").type_s("ChannelId").result(false).api_getter(false).finalize(),
			builder.name("server_id").type_s("ServerId").result(false).api_getter(false).finalize(),
			builder.name("parent_channel_id").type_s("ChannelId").update("Self::query_parent_channel_id(self.server_id, self.id)")
				.documentation("The id of the parent channel, 0 if there is no parent channel").api_getter(false).finalize(),
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

			builder.name("optional_data").type_s("OptionalChannelData").initialisation("OptionalChannelData::new(server_id, id)").update("OptionalChannelData::new(self.server_id, self.id)").result(false).api_getter(false).finalize(),
		]).finalize();

	// Structs
	f.write_all(optional_channel_data.create_struct().as_bytes()).unwrap();
	f.write_all(channel.create_struct().as_bytes()).unwrap();

	// Implementations
	f.write_all(optional_channel_data.create_impl().as_bytes()).unwrap();
	f.write_all(optional_channel_data.create_update().as_bytes()).unwrap();
	f.write_all(optional_channel_data.create_constructor().as_bytes()).unwrap();
	f.write_all(channel.create_impl().as_bytes()).unwrap();
	f.write_all(channel.create_update().as_bytes()).unwrap();
	f.write_all(channel.create_constructor().as_bytes()).unwrap();
	f.write_all(channel.create_api_impl().as_bytes()).unwrap();
}
