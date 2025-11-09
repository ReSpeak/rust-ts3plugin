use std::ffi::CStr;
use std::mem::transmute;
use std::os::raw::{c_char, c_int, c_short, c_uint};
use std::slice;
use std::sync::Mutex;

use ts3plugin_sys::public_definitions::*;
use ts3plugin_sys::ts3functions::Ts3Functions;

use crate::plugin::Plugin;

lazy_static! {
	/// The api, plugin and plugin id
	pub(crate) static ref DATA: Mutex<(Option<(crate::TsApi, Box<dyn Plugin>)>, Option<String>)> =
		Mutex::new((None, None));
}

/// Get the current file without the preceding path
macro_rules! filename {
	() => {{
		let f = file!();
		&f[f.rfind(|c| c == '/' || c == '\\').map_or(0, |i| i + 1)..]
	}};
}

/// Log an error with a description and the current line and file
macro_rules! error {
	($api: ident, $description: expr, $error: expr) => {
		$api.log_or_print(
			format!("Error {:?} ({}) in in {}:L{}", $error, $description, filename!(), line!()),
			"rust-ts3plugin",
			crate::LogLevel::Error,
		);
	};
}

/// Initialises the internal data.
/// T is the plugin type.
/// This function will be called from `create_plugin!`, please don't call it manually.
#[doc(hidden)]
pub unsafe fn private_init<T: Plugin>() -> Result<(), crate::InitError> {
	// Create the TsApi
	let plugin_id = {
		let mut data = DATA.lock().unwrap();
		data.1.take().unwrap()
	};
	let mut api = crate::TsApi::new(plugin_id);
	if let Err(error) = api.load() {
		error!(api, "Can't create TsApi", error);
		return Err(crate::InitError::Failure);
	}

	// Create the plugin
	match T::new(&mut api) {
		Ok(plugin) => {
			let mut data = DATA.lock().unwrap();
			data.0 = Some((api, plugin));
			Ok(())
		}
		Err(error) => Err(error),
	}
}

// ************************** Interface for TeamSpeak **************************

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn ts3plugin_apiVersion() -> c_int { 26 }

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_setFunctionPointers(funs: Ts3Functions) {
	crate::TS3_FUNCTIONS = Some(funs);
}

/// Called when the plugin should be unloaded.
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_shutdown() {
	let mut data = DATA.lock().unwrap();
	if let Some(data) = data.0.as_mut() {
		let api = &mut data.0;
		let plugin = &mut data.1;
		plugin.shutdown(api);
	}
	// Drop the api and the plugin
	*data = (None, None);
}

/// Called when settings is opened and QtThread or NewThread is set as configurable offer.
/// https://github.com/teamspeak/ts3client-pluginsdk/blob/4aa90a53aa150cbf81e13bc97e68c0431b26499f/src/plugin.h#L32
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn ts3plugin_configure(_handle: *mut std::os::raw::c_void, _qParentWidget: *mut std::os::raw::c_void) {
	// !TODO
	// The handle parameter and qParentWidget parameter is not really needed to show a settings window using egui or whatever framework is chosen.
	// Maybe expose the pointers anyway later if needed.
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	plugin.configure(api);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_registerPluginID(plugin_id: *const c_char) {
	let mut data = DATA.lock().unwrap();
	data.1 = Some(to_string!(plugin_id));
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onConnectStatusChangeEvent(
	server_id: u64, status: c_int, error: c_uint,
) {
	let server_id = crate::ServerId(server_id);
	let status = transmute(status);
	let error = transmute(error);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	// Add the server if we can get information about it
	// and don't have that server cached already.
	if status != ConnectStatus::Connecting && api.get_server(server_id).is_none() {
		api.add_server(server_id);
	}
	{
		let server = api.get_server_unwrap(server_id);
		// Execute plugin callback
		plugin.connect_status_change(api, &server, status, error);
	}
	// Remove server if we disconnected
	if status == ConnectStatus::Disconnected {
		api.remove_server(server_id);
	}
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerStopEvent(server_id: u64, message: *const c_char) {
	let server_id = crate::ServerId(server_id);
	let message = to_string!(message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	plugin.server_stop(api, &server, message);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerErrorEvent(
	server_id: u64, message: *const c_char, error: c_uint, return_code: *const c_char,
	extra_message: *const c_char,
) -> c_int {
	let server_id = crate::ServerId(server_id);
	let message = to_string!(message);
	let error = transmute(error);
	let return_code = to_string!(return_code);
	let extra_message = to_string!(extra_message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	let b = plugin.server_error(api, &server, error, message, return_code, extra_message);
	if b { 1 } else { 0 }
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerEditedEvent(
	server_id: u64, invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let invoker = if invoker_id == 0 {
		None
	} else {
		Some(crate::InvokerData::new(
			crate::ConnectionId(invoker_id),
			to_string!(invoker_uid),
			to_string!(invoker_name),
		))
	};
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	if let Some(ref invoker) = invoker {
		api.try_update_invoker(server_id, invoker);
	}
	if let Some(ref mut server) = api.get_mut_server(server_id) {
		server.update();
	}
	let server = api.get_server_unwrap(server_id);
	plugin.server_edited(api, &server, invoker.map(|i| crate::Invoker::new(server.clone(), i)).as_ref());
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerConnectionInfoEvent(server_id: u64) {
	let server_id = crate::ServerId(server_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	plugin.server_connection_info(api, &server);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onConnectionInfoEvent(server_id: u64, connection_id: u16) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	let connection = server.get_connection_unwrap(connection_id);
	plugin.connection_info(api, &server, &connection);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateClientEvent(
	server_id: u64, connection_id: u16, invoker_id: u16, invoker_name: *const c_char,
	invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);

	// Save the old connection
	let old_connection;
	{
		let server = api.get_mut_server(server_id).unwrap();
		// Try to get the old channel
		old_connection = server
			.remove_connection(connection_id)
			.unwrap_or(crate::ConnectionData::new(server_id, connection_id));
		let connection = server.add_connection(connection_id);
		// Copy optional data from old connection
		connection.update_from(&old_connection);
	}
	let server = api.get_server_unwrap(server_id);
	let connection = server.get_connection_unwrap(connection_id);
	let old_connection = crate::Connection::new(api, &old_connection);
	plugin.connection_properties_changed(
		api,
		&server,
		&connection,
		&old_connection,
		crate::get_connection_changes(old_connection.properties(), connection.properties()),
		&crate::Invoker::new(server.clone(), invoker),
	);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveEvent(
	server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64,
	visibility: c_int, move_message: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let old_channel_id = crate::ChannelId(old_channel_id);
	let new_channel_id = crate::ChannelId(new_channel_id);
	let visibility = transmute(visibility);
	let move_message = to_string!(move_message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	if old_channel_id == crate::ChannelId(0) {
		// Connection connected, this will also be called for ourselves
		api.get_mut_server(server_id).unwrap().add_connection(connection_id);
		let server = api.get_server_unwrap(server_id);
		let connection = server.get_connection_unwrap(connection_id);
		plugin.connection_changed(api, &server, &connection, true, move_message)
	} else if new_channel_id == crate::ChannelId(0) {
		// Connection disconnected
		{
			let server = api.get_server_unwrap(server_id);
			let connection = server.get_connection_unwrap(connection_id);
			plugin.connection_changed(api, &server, &connection, false, move_message);
		}
		api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
	} else if old_channel_id == new_channel_id {
		// Connection announced
		match visibility {
			Visibility::Enter => {
				api.get_mut_server(server_id).unwrap().add_connection(connection_id);
				let server = api.get_server_unwrap(server_id);
				let connection = server.get_connection_unwrap(connection_id);
				plugin.connection_announced(api, &server, &connection, true);
			}
			Visibility::Leave => {
				{
					let server = api.get_server_unwrap(server_id);
					let connection = server.get_connection_unwrap(connection_id);
					plugin.connection_announced(api, &server, &connection, false);
				}
				api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
			}
			Visibility::Retain => {}
		}
	} else {
		// Connection switched channel
		// Add the connection if it entered visibility
		if visibility == Visibility::Enter {
			api.get_mut_server(server_id).unwrap().add_connection(connection_id);
		}
		// Update the channel
		{
			if let Some(connection) =
				api.get_mut_server(server_id).and_then(|s| s.get_mut_connection(connection_id))
			{
				connection.channel_id = Ok(new_channel_id);
			}
		}
		{
			let server = api.get_server_unwrap(server_id);
			let connection = server.get_connection_unwrap(connection_id);
			let old_channel = server.get_channel_unwrap(old_channel_id);
			let new_channel = server.get_channel_unwrap(new_channel_id);
			plugin.connection_move(
				api,
				&server,
				&connection,
				&old_channel,
				&new_channel,
				visibility,
			);
		}
		// Remove the connection if it left visibility
		if visibility == Visibility::Leave {
			api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
		}
	}
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveMovedEvent(
	server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64,
	visibility: c_int, invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
	move_message: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let old_channel_id = crate::ChannelId(old_channel_id);
	let new_channel_id = crate::ChannelId(new_channel_id);
	let visibility = transmute(visibility);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let move_message = to_string!(move_message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	// Appart from the invoker, the same code as for ClientMove
	api.try_update_invoker(server_id, &invoker);
	if old_channel_id == crate::ChannelId(0) {
		// Connection connected, this will also be called for ourselves
		api.get_mut_server(server_id).unwrap().add_connection(connection_id);
		let server = api.get_server_unwrap(server_id);
		let connection = server.get_connection_unwrap(connection_id);
		plugin.connection_changed(api, &server, &connection, true, move_message)
	} else if new_channel_id == crate::ChannelId(0) {
		// Connection disconnected
		{
			let server = api.get_server_unwrap(server_id);
			let connection = server.get_connection_unwrap(connection_id);
			plugin.connection_changed(api, &server, &connection, false, move_message);
		}
		api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
	} else if old_channel_id == new_channel_id {
		// Connection announced
		match visibility {
			Visibility::Enter => {
				api.get_mut_server(server_id).unwrap().add_connection(connection_id);
				let server = api.get_server_unwrap(server_id);
				let connection = server.get_connection_unwrap(connection_id);
				plugin.connection_announced(api, &server, &connection, true);
			}
			Visibility::Leave => {
				{
					let server = api.get_server_unwrap(server_id);
					let connection = server.get_connection_unwrap(connection_id);
					plugin.connection_announced(api, &server, &connection, false);
				}
				api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
			}
			Visibility::Retain => {}
		}
	} else {
		// Connection switched channel
		// Add the connection if it entered visibility
		if visibility == Visibility::Enter {
			api.get_mut_server(server_id).unwrap().add_connection(connection_id);
		}
		// Update the channel
		{
			if let Some(connection) =
				api.get_mut_server(server_id).and_then(|s| s.get_mut_connection(connection_id))
			{
				connection.channel_id = Ok(new_channel_id);
			}
		}
		{
			let server = api.get_server_unwrap(server_id);
			let connection = server.get_connection_unwrap(connection_id);
			let old_channel = server.get_channel_unwrap(old_channel_id);
			let new_channel = server.get_channel_unwrap(new_channel_id);
			plugin.connection_moved(
				api,
				&server,
				&connection,
				&old_channel,
				&new_channel,
				visibility,
				&crate::Invoker::new(server.clone(), invoker),
			);
		}
		// Remove the connection if it left visibility
		if visibility == Visibility::Leave {
			api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
		}
	}
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveSubscriptionEvent(
	server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	//let old_channel_id = ::ChannelId(old_channel_id);
	//let new_channel_id = ::ChannelId(new_channel_id);
	let visibility = transmute(visibility);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	// Connection announced
	match visibility {
		Visibility::Enter => {
			api.get_mut_server(server_id).unwrap().add_connection(connection_id);
			let server = api.get_server_unwrap(server_id);
			let connection = server.get_connection_unwrap(connection_id);
			plugin.connection_announced(api, &server, &connection, true);
		}
		Visibility::Leave => {
			{
				let server = api.get_server_unwrap(server_id);
				let connection = server.get_connection_unwrap(connection_id);
				plugin.connection_announced(api, &server, &connection, false);
			}
			api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
		}
		Visibility::Retain => {}
	}
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveTimeoutEvent(
	server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64,
	visibility: c_int, timeout_message: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	//let old_channel_id = ::ChannelId(old_channel_id);
	//let new_channel_id = ::ChannelId(new_channel_id);
	//let visibility = transmute(visibility);
	let timeout_message = to_string!(timeout_message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	{
		let server = api.get_server_unwrap(server_id);
		let connection = server.get_connection_unwrap(connection_id);
		plugin.connection_timeout(api, &server, &connection);
	}
	api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onNewChannelEvent(
	server_id: u64, channel_id: u64, parent_channel_id: u64,
) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	//let parent_channel_id = ::ChannelId(parent_channel_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let err = api.get_mut_server(server_id).unwrap().add_channel(channel_id).err();
	if let Some(error) = err {
		error!(api, "Can't get channel information", error);
	}
	let server = api.get_server_unwrap(server_id);
	let channel = server.get_channel_unwrap(channel_id);
	plugin.channel_announced(api, &server, &channel);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelDescriptionUpdateEvent(
	server_id: u64, channel_id: u64,
) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	// FIXME
	/*// Seems like I really like constructions like that, I failed to do it simpler
	// because I can't borrow api to print an error message in the inner part.
	if let Err(error) = if let Some(channel) = api.get_mut_server(server_id)
			.unwrap().get_mut_channel(channel_id) {
			channel.optional_data.update_description();
			channel.get_optional_data().get_description().map(|_| ())
		} else {
			Ok(())
		} {
		error!(api, "Can't get channel description", error);
	}*/
	let server = api.get_server_unwrap(server_id);
	let channel = server.get_channel_unwrap(channel_id);
	plugin.channel_description_updated(api, &server, &channel);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateChannelEvent(server_id: u64, channel_id: u64) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let old_channel;
	if let Err(error) = {
		let server = api.get_mut_server(server_id).unwrap();
		// Try to get the old channel
		old_channel =
			server.remove_channel(channel_id).unwrap_or(crate::ChannelData::new(server_id, channel_id));
		match server.add_channel(channel_id) {
			Ok(_) => {
				let channel = server.get_mut_channel(channel_id).unwrap();
				// Copy optional data from old channel
				channel.update_from(&old_channel);
				Ok(())
			}
			Err(error) => Err(error),
		}
	} {
		error!(api, "Can't get channel information", error);
	}
	let server = api.get_server_unwrap(server_id);
	let channel = server.get_channel_unwrap(channel_id);
	plugin.channel_updated(api, &server, &channel, &crate::Channel::new(api, &old_channel));
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onNewChannelCreatedEvent(
	server_id: u64, channel_id: u64, parent_channel_id: u64, invoker_id: u16,
	invoker_name: *const c_char, invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	let parent_channel_id = crate::ChannelId(parent_channel_id);
	let invoker = if invoker_id == 0 {
		None
	} else {
		Some(crate::InvokerData::new(
			crate::ConnectionId(invoker_id),
			to_string!(invoker_uid),
			to_string!(invoker_name),
		))
	};
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	if let Some(ref invoker) = invoker {
		api.try_update_invoker(server_id, invoker);
	}
	if let Some(error) = match api.get_mut_server(server_id).unwrap().add_channel(channel_id) {
		Ok(channel) => {
			channel.parent_channel_id = Ok(parent_channel_id);
			None
		}
		Err(error) => Some(error),
	} {
		error!(api, "Can't get channel information", error);
	}
	let server = api.get_server_unwrap(server_id);
	let channel = server.get_channel_unwrap(channel_id);
	plugin.channel_created(
		api,
		&server,
		&channel,
		invoker.map(|i| crate::Invoker::new(server.clone(), i)).as_ref(),
	);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onDelChannelEvent(
	server_id: u64, channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
	invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	let invoker = if invoker_id == 0 {
		None
	} else {
		Some(crate::InvokerData::new(
			crate::ConnectionId(invoker_id),
			to_string!(invoker_uid),
			to_string!(invoker_name),
		))
	};
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	if let Some(ref invoker) = invoker {
		api.try_update_invoker(server_id, invoker);
	}
	{
		let server = api.get_server_unwrap(server_id);
		let channel = server.get_channel_unwrap(channel_id);
		plugin.channel_deleted(
			api,
			&server,
			&channel,
			invoker.map(|i| crate::Invoker::new(server.clone(), i)).as_ref(),
		);
	}
	if api.get_mut_server(server_id).and_then(|s| s.remove_channel(channel_id)).is_none() {
		api.log_or_print("Can't remove channel", "rust-ts3plugin", crate::LogLevel::Error);
	}
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateChannelEditedEvent(
	server_id: u64, channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
	invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	let old_channel;
	if let Err(error) = {
		let server = api.get_mut_server(server_id).unwrap();
		// Try to get the old channel
		old_channel =
			server.remove_channel(channel_id).unwrap_or(crate::ChannelData::new(server_id, channel_id));
		match server.add_channel(channel_id) {
			Ok(_) => {
				let channel = server.get_mut_channel(channel_id).unwrap();
				// Copy optional data from old channel
				channel.update_from(&old_channel);
				Ok(())
			}
			Err(error) => Err(error),
		}
	} {
		error!(api, "Can't get channel information", error);
	}
	let server = api.get_server_unwrap(server_id);
	let channel = server.get_channel_unwrap(channel_id);
	plugin.channel_edited(
		api,
		&server,
		&channel,
		&crate::Channel::new(api, &old_channel),
		&crate::Invoker::new(server.clone(), invoker),
	);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelPasswordChangedEvent(server_id: u64, channel_id: u64) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	let channel = server.get_channel_unwrap(channel_id);
	plugin.channel_password_updated(api, &server, &channel);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelMoveEvent(
	server_id: u64, channel_id: u64, new_parent_channel_id: u64, invoker_id: u16,
	invoker_name: *const c_char, invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let channel_id = crate::ChannelId(channel_id);
	let new_parent_channel_id = crate::ChannelId(new_parent_channel_id);
	let invoker = if invoker_id == 0 {
		None
	} else {
		Some(crate::InvokerData::new(
			crate::ConnectionId(invoker_id),
			to_string!(invoker_uid),
			to_string!(invoker_name),
		))
	};
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	if let Some(ref invoker) = invoker {
		api.try_update_invoker(server_id, invoker);
	}
	{
		let server = api.get_server_unwrap(server_id);
		let channel = server.get_channel_unwrap(channel_id);
		let new_parent_channel = server.get_channel_unwrap(new_parent_channel_id);
		plugin.channel_moved(
			api,
			&server,
			&channel,
			&new_parent_channel,
			invoker.map(|i| crate::Invoker::new(server.clone(), i)).as_ref(),
		);
	}
	if let Some(channel) = api.get_mut_server(server_id).and_then(|s| s.get_mut_channel(channel_id))
	{
		channel.parent_channel_id = Ok(new_parent_channel_id);
	}
}

// Ignore clippy warnings, we can't change the TeamSpeak interface
#[allow(clippy::too_many_arguments)]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onTextMessageEvent(
	server_id: u64, target_mode: u16, receiver_id: u16, invoker_id: u16,
	invoker_name: *const c_char, invoker_uid: *const c_char, message: *const c_char,
	ignored: c_int,
) -> c_int {
	let server_id = crate::ServerId(server_id);
	let target_mode = transmute(target_mode as i32);
	let receiver_id = crate::ConnectionId(receiver_id);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let message = to_string!(message);
	let ignored = ignored != 0;
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	let message_receiver = match target_mode {
		crate::TextMessageTargetMode::Client => crate::MessageReceiver::Connection(receiver_id),
		crate::TextMessageTargetMode::Channel => crate::MessageReceiver::Channel,
		crate::TextMessageTargetMode::Server => crate::MessageReceiver::Server,
		_ => {
			api.log_or_print(
				"Got invalid TextMessageTargetMode",
				"rust-ts3plugin",
				crate::LogLevel::Error,
			);
			crate::MessageReceiver::Server
		}
	};
	let server = api.get_server_unwrap(server_id);
	if plugin.message(
		api,
		&server,
		&crate::Invoker::new(server.clone(), invoker),
		message_receiver,
		message,
		ignored,
	) {
		1
	} else {
		0
	}
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientPokeEvent(
	server_id: u64, invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
	message: *const c_char, ignored: c_int,
) -> c_int {
	let server_id = crate::ServerId(server_id);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let message = to_string!(message);
	let ignored = ignored != 0;
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	let server = api.get_server_unwrap(server_id);
	if plugin.poke(api, &server, &crate::Invoker::new(server.clone(), invoker), message, ignored) {
		1
	} else {
		0
	}
}

#[allow(clippy::too_many_arguments)]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientKickFromChannelEvent(
	server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64,
	visibility: c_int, invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
	message: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let old_channel_id = crate::ChannelId(old_channel_id);
	let new_channel_id = crate::ChannelId(new_channel_id);
	let visibility = transmute(visibility);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let message = to_string!(message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	{
		let server = api.get_server_unwrap(server_id);
		let connection = server.get_connection_unwrap(connection_id);
		let old_channel = server.get_channel_unwrap(old_channel_id);
		let new_channel = server.get_channel_unwrap(new_channel_id);
		plugin.channel_kick(
			api,
			&server,
			&connection,
			&old_channel,
			&new_channel,
			visibility,
			&crate::Invoker::new(server.clone(), invoker),
			message,
		);
	}
	// Remove the kicked connection if it is not visible anymore
	if visibility == crate::Visibility::Leave {
		api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
	} else if let Some(connection) = api.get_mut_server(server_id).and_then(|s|
		// Update the current channel of the connection
		s.get_mut_connection(connection_id))
	{
		connection.channel_id = Ok(new_channel_id);
	}
}

#[allow(clippy::too_many_arguments)]
#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientKickFromServerEvent(
	server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64,
	visibility: c_int, invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
	message: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let old_channel_id = crate::ChannelId(old_channel_id);
	let new_channel_id = crate::ChannelId(new_channel_id);
	//let visibility = transmute(visibility);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let message = to_string!(message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	{
		let server = api.get_server_unwrap(server_id);
		let connection = server.get_connection_unwrap(connection_id);
		plugin.server_kick(
			api,
			&server,
			&connection,
			&crate::Invoker::new(server.clone(), invoker),
			message,
		);
	}
	// Remove the kicked connection
	api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
}

#[allow(clippy::too_many_arguments)]
#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientBanFromServerEvent(
	server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64,
	visibility: c_int, invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
	time: u64, message: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	//let old_channel_id = ::ChannelId(old_channel_id);
	//let new_channel_id = ::ChannelId(new_channel_id);
	//let visibility = transmute(visibility);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let message = to_string!(message);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	{
		let server = api.get_server_unwrap(server_id);
		let connection = server.get_connection_unwrap(connection_id);
		plugin.server_ban(
			api,
			&server,
			&connection,
			&crate::Invoker::new(server.clone(), invoker),
			message,
			time,
		);
	}
	// Remove the banned connection
	api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onTalkStatusChangeEvent(
	server_id: u64, talking: c_int, whispering: c_int, connection_id: u16,
) {
	let server_id = crate::ServerId(server_id);
	let talking = transmute(talking);
	let whispering = whispering != 0;
	let connection_id = crate::ConnectionId(connection_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	{
		let server = api.get_server_unwrap(server_id);
		let connection = server.get_connection_unwrap(connection_id);
		plugin.talking_changed(api, &server, &connection, talking, whispering);
	}
	// Update the connection
	if let Some(connection) =
		api.get_mut_server(server_id).and_then(|s| s.get_mut_connection(connection_id))
	{
		connection.talking = Ok(talking);
		connection.whispering = Ok(whispering);
	}
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onAvatarUpdated(
	server_id: u64, connection_id: u16, avatar_path: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let path = if avatar_path.is_null() { None } else { Some(to_string!(avatar_path)) };
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	let connection = server.get_connection_unwrap(connection_id);
	plugin.avatar_changed(api, &server, &connection, path);
}
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientChannelGroupChangedEvent(
	server_id: u64, channel_group_id: u64, channel_id: u64, connection_id: u16, invoker_id: u16,
	invoker_name: *const c_char, invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let channel_group_id = crate::ChannelGroupId(channel_group_id);
	let channel_id = crate::ChannelId(channel_id);
	let connection_id = crate::ConnectionId(connection_id);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	let server = api.get_server_unwrap(server_id);
	let connection = server.get_connection_unwrap(connection_id);
	let channel_group = server.get_channel_group_unwrap(channel_group_id);
	let channel = server.get_channel_unwrap(channel_id);
	plugin.connection_channel_group_changed(
		api,
		&server,
		&connection,
		&channel_group,
		&channel,
		&crate::Invoker::new(server.clone(), invoker),
	);
}

#[allow(clippy::too_many_arguments)]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerGroupClientAddedEvent(
	server_id: u64, connection_id: u16, connection_name: *const c_char,
	connection_uid: *const c_char, server_group_id: u64, invoker_id: u16,
	invoker_name: *const c_char, invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let connection_name = to_string!(connection_name);
	let connection_uid = to_string!(connection_uid);
	let connection = crate::InvokerData::new(connection_id, connection_uid, connection_name);
	let server_group_id = crate::ServerGroupId(server_group_id);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	let server = api.get_server_unwrap(server_id);
	let server_group = server.get_server_group_unwrap(server_group_id);
	plugin.connection_server_group_added(
		api,
		&server,
		&crate::Invoker::new(server.clone(), connection),
		&server_group,
		&crate::Invoker::new(server.clone(), invoker),
	);
}

#[allow(clippy::too_many_arguments)]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerGroupClientDeletedEvent(
	server_id: u64, connection_id: u16, connection_name: *const c_char,
	connection_uid: *const c_char, server_group_id: u64, invoker_id: u16,
	invoker_name: *const c_char, invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let connection_name = to_string!(connection_name);
	let connection_uid = to_string!(connection_uid);
	let connection = crate::InvokerData::new(connection_id, connection_uid, connection_name);
	let server_group_id = crate::ServerGroupId(server_group_id);
	let invoker_id = crate::ConnectionId(invoker_id);
	let invoker_name = to_string!(invoker_name);
	let invoker_uid = to_string!(invoker_uid);
	let invoker = crate::InvokerData::new(invoker_id, invoker_uid, invoker_name);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	api.try_update_invoker(server_id, &invoker);
	let server = api.get_server_unwrap(server_id);
	let server_group = server.get_server_group_unwrap(server_group_id);
	plugin.connection_server_group_removed(
		api,
		&server,
		&crate::Invoker::new(server.clone(), connection),
		&server_group,
		&crate::Invoker::new(server.clone(), invoker),
	);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerPermissionErrorEvent(
	server_id: u64, message: *const c_char, error: c_uint, return_code: *const c_char,
	permission_id: c_uint,
) -> c_int {
	let server_id = crate::ServerId(server_id);
	let message = to_string!(message);
	let error = transmute(error);
	let return_code = to_string!(return_code);
	let permission_id = crate::PermissionId(permission_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	let permission = api.get_permission(permission_id).unwrap();
	if plugin.permission_error(api, &server, permission, error, message, return_code) {
		1
	} else {
		0
	}
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditPlaybackVoiceDataEvent(
	server_id: u64, connection_id: u16, samples: *mut c_short, sample_count: c_int, channels: c_int,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	let connection = server.get_connection_unwrap(connection_id);
	plugin.playback_voice_data(api, &server, &connection, samples, channels);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditPostProcessVoiceDataEvent(
	server_id: u64, connection_id: u16, samples: *mut c_short, sample_count: c_int,
	channels: c_int, channel_speaker_array: *const c_uint, channel_fill_mask: *mut c_uint,
) {
	let server_id = crate::ServerId(server_id);
	let connection_id = crate::ConnectionId(connection_id);
	let samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
	let channel_speaker_array =
		slice::from_raw_parts(channel_speaker_array as *mut crate::Speaker, channels as usize);
	let channel_fill_mask = channel_fill_mask.as_mut().unwrap();
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	let connection = server.get_connection_unwrap(connection_id);
	plugin.post_process_voice_data(
		api,
		&server,
		&connection,
		samples,
		channels,
		channel_speaker_array,
		channel_fill_mask,
	);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditMixedPlaybackVoiceDataEvent(
	server_id: u64, samples: *mut c_short, sample_count: c_int, channels: c_int,
	channel_speaker_array: *const c_uint, channel_fill_mask: *mut c_uint,
) {
	let server_id = crate::ServerId(server_id);
	let samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
	let channel_speaker_array =
		slice::from_raw_parts(channel_speaker_array as *mut crate::Speaker, channels as usize);
	let channel_fill_mask = channel_fill_mask.as_mut().unwrap();
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	plugin.mixed_playback_voice_data(
		api,
		&server,
		samples,
		channels,
		channel_speaker_array,
		channel_fill_mask,
	);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditCapturedVoiceDataEvent(
	server_id: u64, samples: *mut c_short, sample_count: c_int, channels: c_int, edited: *mut c_int,
) {
	let server_id = crate::ServerId(server_id);
	let samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
	let mut send = (*edited & 2) != 0;
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	// Set the first bit if the sound data were edited
	*edited |= plugin.captured_voice_data(api, &server, samples, channels, &mut send) as c_int;
	// Set the second bit of `edited` to `send`
	*edited = (*edited & !2) | ((send as c_int) << 1);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onPluginCommandEvent(
	server_id: u64, plugin_name: *const c_char, plugin_command: *const c_char, invoker_id: u16,
	invoker_name: *const c_char, invoker_uid: *const c_char,
) {
	let server_id = crate::ServerId(server_id);
	let invoker = if invoker_id == 0 {
		None
	} else {
		Some(crate::InvokerData::new(
			crate::ConnectionId(invoker_id),
			to_string!(invoker_uid),
			to_string!(invoker_name),
		))
	};
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	if let Some(ref invoker) = invoker {
		api.try_update_invoker(server_id, invoker);
	}
	let server = api.get_server_unwrap(server_id);
	plugin.plugin_message(
		api,
		&server,
		to_string!(plugin_name),
		to_string!(plugin_command),
		invoker.map(|i| crate::Invoker::new(server.clone(), i)).as_ref(),
	);
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_processCommand(server_id: u64, command: *const c_char) -> c_int {
	let server_id = crate::ServerId(server_id);
	let mut data = DATA.lock().unwrap();
	let data = data.0.as_mut().unwrap();
	let api = &mut data.0;
	let plugin = &mut data.1;
	let server = api.get_server_unwrap(server_id);
	if plugin.process_command(api, &server, to_string!(command)) { 0 } else { 1 }
}
