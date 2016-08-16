use libc::{c_char, c_int, c_short, c_uint};
use std::cell::RefCell;
use std::ffi::CStr;
use std::mem::transmute;
use std::slice;

use ts3plugin_sys::clientlib_publicdefinitions::*;
use ts3plugin_sys::ts3functions::Ts3Functions;

use ::plugin::Plugin;

thread_local!(static DATA: RefCell<Option<(::TsApi, Box<Plugin>)>> = RefCell::new(None));

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
        $api.log_or_print(format!("Error {:?} ({}) in in {}:L{}", $error, $description,
            filename!(), line!()), "rust-ts3plugin", ::LogLevel::Error);
    };
}

/// Initialises the internal data.
/// T is the plugin type.
/// This function will be called from `create_plugin!`, please don't call it manually.
#[doc(hidden)]
pub fn private_init<T: 'static + Plugin>() -> Result<(), ::InitError> {
    // Create the TsApi
    let mut api = ::TsApi::new();
    if let Err(error) = api.load() {
        error!(api, "Can't create TsApi", error);
        return Err(::InitError::Failure);
    }

    // Create the plugin
    match T::new(&mut api) {
        Ok(plugin) => {
            DATA.with(move |data| {
                *data.borrow_mut() = Some((api, plugin));
            });
            Ok(())
        }
        Err(error) => Err(error),
    }
}

// ************************** Interface for TeamSpeak **************************

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn ts3plugin_apiVersion() -> c_int {
    20
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_setFunctionPointers(funs: Ts3Functions) {
    ::ts3functions = Some(funs);
}

/// Called when the plugin should be unloaded.
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_shutdown() {
    DATA.with(|data| {
        let mut data = data.borrow_mut();
        {
            let mut data = data.as_mut().unwrap();
            let mut api = &mut data.0;
            let mut plugin = &mut data.1;
            plugin.shutdown(api);
        }
        // Drop the api and the plugin
        *data = None;
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onConnectStatusChangeEvent(server_id: u64,
    status: c_int, error: c_uint) {
    let server_id = ::ServerId(server_id);
    let status = transmute(status);
    let error = transmute(error);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        // Add the server if we can get information about it
        // and don't have that server cached already.
        if status != ConnectStatus::Connecting && api.get_server(server_id).is_none() {
            if let Err(error) = api.add_server(server_id) {
                error!(api, "Can't get server information", error);
            }
        }
        // Execute plugin callback
        plugin.connect_status_change(api, server_id, status, error);
        // Remove server if we disconnected
        if status == ConnectStatus::Disconnected {
            api.remove_server(server_id);
        }
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerStopEvent(server_id: u64, message: *const c_char) {
    let server_id = ::ServerId(server_id);
    let message = to_string!(message);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.server_stop(api, server_id, message);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerErrorEvent(server_id: u64,
    message: *const c_char, error: c_uint, return_code: *const c_char,
    extra_message: *const c_char) -> c_int {
    let server_id = ::ServerId(server_id);
    let message = to_string!(message);
    let error = transmute(error);
    let return_code = to_string!(return_code);
    let extra_message = to_string!(extra_message);
    let b = DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.server_error(api, server_id, error, message, return_code, extra_message)
    });
    if b { 1 } else { 0 }
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerEditedEvent(server_id: u64,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.server_edited(api, server_id, invoker);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerConnectionInfoEvent(server_id: u64) {
    let server_id = ::ServerId(server_id);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.server_connection_info(api, server_id);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onConnectionInfoEvent(server_id: u64, connection_id: u16) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.connection_info(api, server_id, connection_id);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateClientEvent(server_id: u64,
    connection_id: u16, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        // Save the old connection
        let mut old_connection;
        if let Err(error) = {
                let mut server = api.get_mut_server(server_id).unwrap();
                // Try to get the old channel
                old_connection = server.remove_connection(connection_id);
                match server.add_connection(connection_id) {
                    Ok(_) => {
                        let mut connection = server.get_mut_connection(connection_id).unwrap();
                        if let Some(ref mut old_connection) = old_connection {
                            // Copy optional data from old connection if it exists
                            //TODO do that with the build script
                            connection.database_id = old_connection.database_id.take();
                            connection.channel_group_id = old_connection.channel_group_id.take();
                            connection.server_groups = old_connection.server_groups.take();
                            connection.talk_power = old_connection.talk_power.take();
                            connection.talk_request = old_connection.talk_request.take();
                            connection.talk_request_message = old_connection.talk_request_message.take();
                            connection.channel_group_inherited_channel_id =
                                old_connection.channel_group_inherited_channel_id.take();
                            connection.own_data = old_connection.own_data.take();
                            connection.serverquery_data = old_connection.serverquery_data.take();
                            connection.optional_data = old_connection.optional_data.take();
                        }
                        Ok(())
                    },
                    Err(error) => Err(error),
                }
            } {
            error!(api, "Can't get connection information", error);
        } else {
            plugin.connection_updated(api, server_id, connection_id, old_connection, invoker);
        }
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveEvent(server_id: u64, connection_id: u16,
    old_channel_id: u64, new_channel_id: u64, visibility: c_int, move_message: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let old_channel_id = ::ChannelId(old_channel_id);
    let new_channel_id = ::ChannelId(new_channel_id);
    let visibility = transmute(visibility);
    let move_message = to_string!(move_message);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        if old_channel_id == ::ChannelId(0) {
            // Connection connected, this will also be called for ourselves
            let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
            if let Err(error) = err {
                error!(api, "Can't get connection information", error);
            }
            plugin.connection_changed(api, server_id, connection_id, true, move_message)
        } else if new_channel_id == ::ChannelId(0) {
            // Connection disconnected
            plugin.connection_changed(api, server_id, connection_id, false, move_message);
            api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
        } else if old_channel_id == new_channel_id {
            // Connection announced
            match visibility {
                Visibility::Enter => {
                    let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                    if let Err(error) = err {
                        error!(api, "Can't get connection information", error);
                    }
                    plugin.connection_announced(api, server_id, connection_id, true);
                },
                Visibility::Leave => {
                    plugin.connection_announced(api, server_id, connection_id, false);
                    api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
                },
                Visibility::Retain => {}
            }
        } else {
            // Connection switched channel
            // Add the connection if it entered visibility
            if visibility == Visibility::Enter {
                let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                if let Err(error) = err {
                    error!(api, "Can't get connection information", error);
                }
            }
            // Update the channel
            {
                if let Some(connection) = api.get_mut_server(server_id)
                    .and_then(|s| s.get_mut_connection(connection_id)) {
                    connection.channel_id = new_channel_id;
                }
            }
            plugin.connection_move(api, server_id, connection_id,
                                    old_channel_id, new_channel_id, visibility);
            // Remove the connection if it left visibility
            if visibility == Visibility::Leave {
                api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
            }
        }
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveMovedEvent(server_id: u64, connection_id: u16,
    old_channel_id: u64, new_channel_id: u64, visibility: c_int, invoker_id: u16,
    invoker_name: *const c_char, invoker_uid: *const c_char, move_message: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let old_channel_id = ::ChannelId(old_channel_id);
    let new_channel_id = ::ChannelId(new_channel_id);
    let visibility = transmute(visibility);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    let move_message = to_string!(move_message);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        // Appart from the invoker, the same code as for ClientMove
        api.try_update_invoker(server_id, &invoker);
        if old_channel_id == ::ChannelId(0) {
            // Connection connected, this will also be called for ourselves
            let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
            if let Err(error) = err {
                error!(api, "Can't get connection information", error);
            }
            plugin.connection_changed(api, server_id, connection_id, true, move_message)
        } else if new_channel_id == ::ChannelId(0) {
            // Connection disconnected
            plugin.connection_changed(api, server_id, connection_id, false, move_message);
            api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
        } else if old_channel_id == new_channel_id {
            // Connection announced
            match visibility {
                Visibility::Enter => {
                    let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                    if let Err(error) = err {
                        error!(api, "Can't get connection information", error);
                    }
                    plugin.connection_announced(api, server_id, connection_id, true);
                },
                Visibility::Leave => {
                    plugin.connection_announced(api, server_id, connection_id, false);
                    api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
                },
                Visibility::Retain => {}
            }
        } else {
            // Connection switched channel
            // Add the connection if it entered visibility
            if visibility == Visibility::Enter {
                let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                if let Err(error) = err {
                    error!(api, "Can't get connection information", error);
                }
            }
            // Update the channel
            {
                if let Some(connection) = api.get_mut_server(server_id)
                    .and_then(|s| s.get_mut_connection(connection_id)) {
                    connection.channel_id = new_channel_id;
                }
            }
            plugin.connection_moved(api, server_id, connection_id,
                old_channel_id, new_channel_id, visibility, invoker);
            // Remove the connection if it left visibility
            if visibility == Visibility::Leave {
                api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
            }
        }
    });
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveSubscriptionEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    //let old_channel_id = ::ChannelId(old_channel_id);
    //let new_channel_id = ::ChannelId(new_channel_id);
    let visibility = transmute(visibility);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        // Connection announced
        match visibility {
            Visibility::Enter => {
                let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                if let Err(error) = err {
                    error!(api, "Can't get connection information", error);
                }
                plugin.connection_announced(api, server_id, connection_id, true);
            },
            Visibility::Leave => {
                plugin.connection_announced(api, server_id, connection_id, false);
                api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
            },
            Visibility::Retain => {}
        }
    });
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveTimeoutEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
    timeout_message: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    //let old_channel_id = ::ChannelId(old_channel_id);
    //let new_channel_id = ::ChannelId(new_channel_id);
    //let visibility = transmute(visibility);
    let timeout_message = to_string!(timeout_message);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.connection_timeout(api, server_id, connection_id);
        api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
    });
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onNewChannelEvent(server_id: u64, channel_id: u64,
    parent_channel_id: u64) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    //let parent_channel_id = ::ChannelId(parent_channel_id);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        let err = api.get_mut_server(server_id).unwrap().add_channel(channel_id).err();
        if let Some(error) = err {
            error!(api, "Can't get channel information", error);
        }
        plugin.channel_announced(api, server_id, channel_id);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelDescriptionUpdateEvent(server_id: u64,
    channel_id: u64) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        // Seems like I really like constructions like that, I failed to do it simpler
        // because I can't borrow api to print an error message in the inner part.
        if let Err(error) = if let Some(channel) = api.get_mut_server(server_id)
                    .unwrap().get_mut_channel(channel_id) {
                if channel.get_optional_data().is_none() {
                    match ::OptionalChannelData::new(server_id, channel_id) {
                        Ok(data) => {
                            channel.optional_data = Some(data);
                            Ok(())
                        },
                        Err(error) => Err(error),
                    }
                } else {
                    channel.get_mut_optional_data().as_mut().unwrap().update_description()
                }
            } else {
                Ok(())
            } {
            error!(api, "Can't get channel description", error);
        }
        plugin.channel_description_updated(api, server_id, channel_id);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateChannelEvent(server_id: u64,
    channel_id: u64) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        let mut old_channel;
        if let Err(error) = {
                let mut server = api.get_mut_server(server_id).unwrap();
                // Try to get the old channel
                old_channel = server.remove_channel(channel_id);
                match server.add_channel(channel_id) {
                    Ok(_) => {
                        let mut channel = server.get_mut_channel(channel_id).unwrap();
                        if let Some(ref mut old_channel) = old_channel {
                            // Copy optional data from old channel if it exists
                            channel.optional_data = old_channel.optional_data.take();
                        }
                        Ok(())
                    },
                    Err(error) => Err(error),
                }
            } {
            error!(api, "Can't get channel information", error);
        } else {
            plugin.channel_updated(api, server_id, channel_id, old_channel);
        }
    });
}

#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onNewChannelCreatedEvent(server_id: u64,
    channel_id: u64, parent_channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    //let parent_channel_id = ::ChannelId(parent_channel_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        if let Err(error) = api.get_mut_server(server_id).unwrap()
            .add_channel(channel_id) {
            error!(api, "Can't get channel information", error);
        }
        plugin.channel_created(api, server_id, channel_id, invoker);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onDelChannelEvent(server_id: u64,
    channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.channel_deleted(api, server_id, channel_id, invoker);
        if api.get_mut_server(server_id).and_then(|s| s.remove_channel(channel_id)).is_none() {
            api.log_or_print("Can't remove channel", "rust-ts3plugin", ::LogLevel::Error)
        }
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateChannelEditedEvent(server_id: u64,
    channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        let mut old_channel;
        if let Err(error) = {
                let mut server = api.get_mut_server(server_id).unwrap();
                // Try to get the old channel
                old_channel = server.remove_channel(channel_id);
                match server.add_channel(channel_id) {
                    Ok(_) => {
                        let mut channel = server.get_mut_channel(channel_id).unwrap();
                        if let Some(ref mut old_channel) = old_channel {
                            // Copy optional data from old channel if it exists
                            channel.optional_data = old_channel.optional_data.take();
                        }
                        Ok(())
                    },
                    Err(error) => Err(error),
                }
            } {
            error!(api, "Can't get channel information", error);
        } else {
            plugin.channel_edited(api, server_id, channel_id, old_channel,
                                  invoker);
        }
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelPasswordChangedEvent(server_id: u64,
    channel_id: u64) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.channel_password_updated(api, server_id, channel_id);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelMoveEvent(server_id: u64,
    channel_id: u64, new_parent_channel_id: u64, invoker_id: u16,
    invoker_name: *const c_char, invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let channel_id = ::ChannelId(channel_id);
    let new_parent_channel_id = ::ChannelId(new_parent_channel_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.channel_moved(api, server_id, channel_id, new_parent_channel_id, invoker);
        if let Some(channel) = api.get_mut_server(server_id).and_then(|s| s.get_mut_channel(channel_id)) {
            channel.parent_channel_id = new_parent_channel_id;
        }
    });
}

// Ignore clippy warnings, we can't change the TeamSpeak interface
#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onTextMessageEvent(server_id: u64,
    target_mode: u16, receiver_id: u16, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char, message: *const c_char, ignored: c_int) -> c_int {
    let server_id = ::ServerId(server_id);
    let target_mode = transmute(target_mode as i32);
    let receiver_id = ::ConnectionId(receiver_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    let message = to_string!(message);
    let ignored = ignored != 0;
    let b = DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        let message_receiver = match target_mode {
            ::TextMessageTargetMode::Client =>
                ::MessageReceiver::Connection(receiver_id),
            ::TextMessageTargetMode::Channel => ::MessageReceiver::Channel,
            ::TextMessageTargetMode::Server => ::MessageReceiver::Server,
            _ => {
                api.log_or_print("Got invalid TextMessageTargetMode",
                                 "rust-ts3plugin", ::LogLevel::Error);
                ::MessageReceiver::Server
            }
        };
        plugin.message(api, server_id, invoker, message_receiver, message, ignored)
    });
    if b { 1 } else { 0 }
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientPokeEvent(server_id: u64,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
    message: *const c_char, ignored: c_int) -> c_int {
    let server_id = ::ServerId(server_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    let message = to_string!(message);
    let ignored = ignored != 0;
    let b = DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.poke(api, server_id, invoker, message, ignored)
    });
    if b { 1 } else { 0 }
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientKickFromChannelEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
    message: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let old_channel_id = ::ChannelId(old_channel_id);
    let new_channel_id = ::ChannelId(new_channel_id);
    let visibility = transmute(visibility);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    let message = to_string!(message);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.channel_kick(api, server_id, connection_id, old_channel_id,
                            new_channel_id, visibility, invoker, message);
        // Remove the kicked connection if it is not visible anymore
        if visibility == ::Visibility::Leave {
            api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
        } else if let Some(connection) = api.get_mut_server(server_id).and_then(|s|
            // Update the current channel of the connection
            s.get_mut_connection(connection_id)) {
            connection.channel_id = new_channel_id;
        }
    });
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientKickFromServerEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
    message: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let old_channel_id = ::ChannelId(old_channel_id);
    let new_channel_id = ::ChannelId(new_channel_id);
    //let visibility = transmute(visibility);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    let message = to_string!(message);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.server_kick(api, server_id, connection_id, invoker, message);
        // Remove the kicked connection
        api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
    });
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case, unused_variables)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientBanFromServerEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
    time: u64, message: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    //let old_channel_id = ::ChannelId(old_channel_id);
    //let new_channel_id = ::ChannelId(new_channel_id);
    //let visibility = transmute(visibility);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    let message = to_string!(message);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.server_ban(api, server_id, connection_id, invoker, message, time);
        // Remove the banned connection
        api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onTalkStatusChangeEvent(server_id: u64,
    talking: c_int, whispering: c_int, connection_id: u16) {
    let server_id = ::ServerId(server_id);
    let talking = transmute(talking);
    let whispering = whispering != 0;
    let connection_id = ::ConnectionId(connection_id);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.talking_changed(api, server_id, connection_id, talking, whispering);
        // Update the connection
        if let Some(connection) = api.get_mut_server(server_id).and_then(|s| s.get_mut_connection(connection_id)) {
            connection.talking = talking;
            connection.whispering = whispering;
        }
    });
}


#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onAvatarUpdated(server_id: u64,
    connection_id: u16, avatar_path: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let path = if avatar_path.is_null() { None } else { Some(to_string!(avatar_path)) };
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.avatar_changed(api, server_id, connection_id, path);
    });
}
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientChannelGroupChangedEvent(server_id: u64,
    channel_group_id: u64, channel_id: u64, connection_id: u16, invoker_id: u16,
    invoker_name: *const c_char, invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let channel_group_id = ::ChannelGroupId(channel_group_id);
    let channel_id = ::ChannelId(channel_id);
    let connection_id = ::ConnectionId(connection_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.connection_channel_group_changed(api, server_id,
            connection_id, channel_group_id, channel_id, invoker);
    });
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerGroupClientAddedEvent(server_id: u64,
    connection_id: u16, connection_name: *const c_char, connection_uid: *const c_char,
    server_group_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let connection_name = to_string!(connection_name);
    let connection_uid = to_string!(connection_uid);
    let connection = ::Invoker::new(connection_id, connection_uid, connection_name);
    let server_group_id = ::ServerGroupId(server_group_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.connection_server_group_added(api, server_id,
            connection, server_group_id, invoker);
    });
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerGroupClientDeletedEvent(server_id: u64,
    connection_id: u16, connection_name: *const c_char, connection_uid: *const c_char,
    server_group_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let connection_name = to_string!(connection_name);
    let connection_uid = to_string!(connection_uid);
    let connection = ::Invoker::new(connection_id, connection_uid, connection_name);
    let server_group_id = ::ServerGroupId(server_group_id);
    let invoker_id = ::ConnectionId(invoker_id);
    let invoker_name = to_string!(invoker_name);
    let invoker_uid = to_string!(invoker_uid);
    let invoker = ::Invoker::new(invoker_id, invoker_uid, invoker_name);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        api.try_update_invoker(server_id, &invoker);
        plugin.connection_server_group_removed(api, server_id,
            connection, server_group_id, invoker);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerPermissionErrorEvent(server_id: u64,
    message: *const c_char, error: c_uint, return_code: *const c_char,
    permission_id: c_uint) -> c_int {
    let server_id = ::ServerId(server_id);
    let message = to_string!(message);
    let error = transmute(error);
    let return_code = to_string!(return_code);
    let permission_id = ::PermissionId(permission_id);
    let b = DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.permission_error(api, server_id, permission_id, error,
            message, return_code)
    });
    if b { 1 } else { 0 }
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditPlaybackVoiceDataEvent(server_id: u64,
    connection_id: u16, samples: *mut c_short, sample_count: c_int, channels: c_int) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let mut samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.playback_voice_data(api, server_id, connection_id, samples, channels);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditPostProcessVoiceDataEvent(server_id: u64,
    connection_id: u16, samples: *mut c_short, sample_count: c_int, channels: c_int,
    channel_speaker_array: *const c_uint, channel_fill_mask: *mut c_uint) {
    let server_id = ::ServerId(server_id);
    let connection_id = ::ConnectionId(connection_id);
    let mut samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
    let channel_speaker_array = slice::from_raw_parts(channel_speaker_array as *mut ::Speaker,
        channels as usize);
    let channel_fill_mask = channel_fill_mask.as_mut().unwrap();
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.post_process_voice_data(api, server_id, connection_id,
            samples, channels, channel_speaker_array, channel_fill_mask);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditMixedPlaybackVoiceDataEvent(server_id: u64,
    samples: *mut c_short, sample_count: c_int, channels: c_int,
    channel_speaker_array: *const c_uint, channel_fill_mask: *mut c_uint) {
    let server_id = ::ServerId(server_id);
    let mut samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
    let channel_speaker_array = slice::from_raw_parts(channel_speaker_array as *mut ::Speaker,
        channels as usize);
    let channel_fill_mask = channel_fill_mask.as_mut().unwrap();
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        plugin.mixed_playback_voice_data(api, server_id, samples, channels,
            channel_speaker_array, channel_fill_mask);
    });
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onEditCapturedVoiceDataEvent(server_id: u64,
    samples: *mut c_short, sample_count: c_int, channels: c_int, edited: *mut c_int) {
    let server_id = ::ServerId(server_id);
    let mut samples = slice::from_raw_parts_mut(samples, (sample_count * channels) as usize);
    let mut send = (*edited & 2) != 0;
    DATA.with(move |data| {
        let mut data = data.borrow_mut();
        let mut data = data.as_mut().unwrap();
        let mut api = &mut data.0;
        let mut plugin = &mut data.1;
        // Set the first bit if the sound data were edited
        *edited |= plugin.captured_voice_data(api, server_id,
            samples, channels, &mut send) as c_int;
        // Set the second bit of `edited` to `send`
        *edited = (*edited & !2) | ((send as c_int) << 1);
    });
}
