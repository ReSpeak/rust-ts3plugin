use *;

pub(crate) fn create() -> Vec<Struct<'static>> {
	// Map types to functions that will get that type
	let default_functions = {
		let mut m = Map::new();
		m.insert("i32", "ChannelData::get_property_as_int");
		m.insert("u64", "ChannelData::get_property_as_uint64");
		m.insert("String", "ChannelData::get_property_as_string");
		m
	};
	let transmutable = vec!["CodecType", "HostbannerMode"];

	let builder = PropertyBuilder::new()
		.functions(default_functions)
		.transmutable(transmutable)
		.default_args("server_id, id, ")
		.default_args_update("self.server_id, self.id, ")
		.enum_name("ChannelProperties");
	let builder_string = builder.type_s("String");
	let builder_i32 = builder.type_s("i32");
	let builder_bool = builder.type_s("bool");

	let channel = StructBuilder::new()
		.name("ChannelData")
		.api_name("Channel")
		.do_api_impl(true)
		.do_properties(true)
		.constructor_args("server_id: ServerId, id: ChannelId")
		.extra_property_list(vec![(
			"Option<Channel<'a>>".into(),
			"OptionChannel".into(),
			"ParentChannel,".into(),
		)])
		.extra_properties(
			"\
			ChannelProperty::OptionChannel {\n\tproperty: \
			 ChannelOptionChannelProperty::ParentChannel,\n\tdata: self.get_parent_channel(),\n},",
		)
		.properties(vec![
			builder.name("id").type_s("ChannelId").result(false).api_getter(false).finalize(),
			builder.name("server_id").type_s("ServerId").result(false).api_getter(false).finalize(),
			builder
				.name("parent_channel_id")
				.type_s("ChannelId")
				.update("Self::query_parent_channel_id(self.server_id, self.id)")
				.documentation("The id of the parent channel, 0 if there is no parent channel")
				.api_getter(false)
				.finalize(),
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
			builder_bool
				.name("max_clients_unlimited")
				.value_name("FlagMaxClientsUnlimited")
				.finalize(),
			builder_bool
				.name("max_family_clients_unlimited")
				.value_name("FlagMaxFamilyClientsUnlimited")
				.finalize(),
			// Clone so we can change the documentation
			builder_bool
				.name("subscribed")
				.value_name("FlagAreSubscribed")
				.documentation("If we are subscribed to this channel")
				.finalize(),
			builder_i32.name("needed_talk_power").finalize(),
			builder_i32.name("forced_silence").finalize(),
			builder_string.name("phonetic_name").value_name("NamePhonetic").finalize(),
			builder_i32.name("icon_id").finalize(),
			builder_string.name("banner_gfx_url").value_name("BannerGfxUrl").finalize(),
			builder
				.name("banner_mode")
				.value_name("BannerMode")
				.type_s("HostbannerMode")
				.finalize(),
			// Requested
			builder_string.name("description").requested(true).finalize(),
		])
		.finalize();

	vec![channel]
}
