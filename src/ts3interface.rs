use libc::{c_char, c_int, c_uint};
use std::ffi::CStr;
use std::mem::transmute;
use std::sync::mpsc::{channel, Sender};

use ts3plugin_sys::clientlib_publicdefinitions::*;
use ts3plugin_sys::public_errors::Error;
use ts3plugin_sys::ts3functions::Ts3Functions;

use ::plugin::Plugin;

static mut TX: Option<*const Sender<FunctionCall>> = None;

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

enum FunctionCall {
    ConnectStatusChange(::ServerId, ConnectStatus, Error),
    ServerStop(::ServerId, String),
    ServerEdited(::ServerId, ::Invoker),
    ServerConnectionInfo(::ServerId),
    ConnectionInfo(::ServerId, ::ConnectionId),
    ConnectionUpdated(::ServerId, ::ConnectionId, ::Invoker),
    ConnectionMove(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility, String),
    ConnectionMoved(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility, String, ::Invoker),
    ConnectionSubscribed(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility),
    ConnectionTimeout(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility, String),
    ChannelAnnounced(::ServerId, ::ChannelId, ::ChannelId),
    ChannelDescriptionUpdate(::ServerId, ::ChannelId),
    ChannelUpdate(::ServerId, ::ChannelId),
    ChannelCreated(::ServerId, ::ChannelId, ::ChannelId, ::Invoker),
    ChannelDeleted(::ServerId, ::ChannelId, ::Invoker),
    ChannelEdited(::ServerId, ::ChannelId, ::Invoker),
    ChannelPasswordUpdate(::ServerId, ::ChannelId),
    ChannelMove(::ServerId, ::ChannelId, ::ChannelId, ::Invoker),
    ChannelKick(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility, ::Invoker, String),
    ServerKick(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility, ::Invoker, String),
    ServerBan(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility, ::Invoker, String, u64),
    TalkStatusChanged(::ServerId, ::ConnectionId, ::TalkStatus, bool),
    AvatarChanged(::ServerId, ::ConnectionId, Option<String>),
    ConnectionChannelGroupChanged(::ServerId, ::ConnectionId, ::ChannelGroupId, ::ChannelId, ::Invoker),
    ConnectionServerGroupAdded(::ServerId, ::Invoker, ::ServerGroupId, ::Invoker),
    ConnectionServerGroupRemoved(::ServerId, ::Invoker, ::ServerGroupId, ::Invoker),
    /// Some functions request a return value. The return value should be passed
    /// through the sender.
    /// IMPORTANT: In cases where a return value is needed, the plugins shouldn't
    /// get a mutable reference to the api, but only a constant reference.
    ReturningCall(Sender<ReturnValue>, ReturningCall),
    Quit
}

struct ReturnValue(bool);

enum ReturningCall {
    ServerError(::ServerId, ::Error, String, String, String),
    PermissionError(::ServerId, ::PermissionId, ::Error, String, String),
    Message(::ServerId, ::TextMessageTargetMode, u16, ::Invoker, String, bool),
    Poke(::ServerId, ::Invoker, String, bool),
}

/// The manager thread calls plugin functions on events.
/// T is the plugin type.
#[doc(hidden)]
pub fn manager_thread<T: Plugin>(main_transmitter: Sender<Result<(), ::InitError>>) {
    // Create channel where we will receive the events from TeamSpeak callbacks
    let (tx, rx) = channel();
    unsafe {
        TX = Some(&tx)
    }
    // Create the TsApi
    let mut api = ::TsApi::new();
    if let Err(error) = api.load() {
        error!(api, "Can't create TsApi", error);
        main_transmitter.send(Err(::InitError::Failure)).unwrap();
        return;
    }
    // Create the plugin
    let mut plugin: Box<Plugin> = match T::new(&api) {
        Ok(plugin) => {
            // Send that we are ready
            main_transmitter.send(Ok(())).unwrap();
            plugin
        },
        Err(error) => {
            main_transmitter.send(Err(error)).unwrap();
            return;
        }
    };

    // Wait for messages
    loop {
        match rx.recv().unwrap() {
            FunctionCall::ConnectStatusChange(server_id, status, error) => {
                // Add the server if we can get information about it
                // and don't have that server cached already.
                if status != ConnectStatus::Connecting && api.get_server(server_id).is_none() {
                    if let Err(error) = api.add_server(server_id) {
                        error!(api, "Can't get server information", error);
                    }
                }
                // Execute plugin callback
                plugin.connect_status_change(&mut api, server_id, status, error);
                // Remove server if we disconnected
                if status == ConnectStatus::Disconnected {
                    api.remove_server(server_id);
                }
            },
            FunctionCall::ServerStop(server_id, message) => {
                plugin.server_stop(&mut api, server_id, message);
            },
            FunctionCall::ServerEdited(server_id, invoker) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.server_edited(&mut api, server_id, invoker);
            },
            FunctionCall::ServerConnectionInfo(server_id) => {
                plugin.server_connection_info(&mut api, server_id);
            },
            FunctionCall::ConnectionInfo(server_id, connection_id) => {
                plugin.connection_info(&mut api, server_id, connection_id);
            },
            FunctionCall::ConnectionUpdated(server_id, connection_id, invoker) => {
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
                    plugin.connection_updated(&mut api, server_id, connection_id, old_connection, invoker);
                }
            },
            FunctionCall::ConnectionMove(server_id, connection_id, old_channel_id,
                                     new_channel_id, visibility, move_message) => {
                if old_channel_id == ::ChannelId(0) {
                    // Connection connected, this will also be called for ourselves
                    let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                    if let Err(error) = err {
                        error!(api, "Can't get connection information", error);
                    }
                    plugin.connection_changed(&mut api, server_id, connection_id, true, move_message)
                } else if new_channel_id == ::ChannelId(0) {
                    // Connection disconnected
                    plugin.connection_changed(&mut api, server_id, connection_id, false, move_message);
                    api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
                } else if old_channel_id == new_channel_id {
                    // Connection announced
                    match visibility {
                        Visibility::Enter => {
                            let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                            if let Err(error) = err {
                                error!(api, "Can't get connection information", error);
                            }
                            plugin.connection_announced(&mut api, server_id, connection_id, true);
                        },
                        Visibility::Leave => {
                            plugin.connection_announced(&mut api, server_id, connection_id, false);
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
                    plugin.connection_move(&mut api, server_id, connection_id,
                                            old_channel_id, new_channel_id, visibility);
                    // Remove the connection if it left visibility
                    if visibility == Visibility::Leave {
                        api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
                    }
                }
            },
            FunctionCall::ConnectionMoved(server_id, connection_id, old_channel_id,
                    new_channel_id, visibility, move_message, invoker) => {
                // Appart from the invoker, the same code as for ConnectionMove
                api.try_update_invoker(server_id, &invoker);
                if old_channel_id == ::ChannelId(0) {
                    // Connection connected, this will also be called for ourselves
                    let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                    if let Err(error) = err {
                        error!(api, "Can't get connection information", error);
                    }
                    plugin.connection_changed(&mut api, server_id, connection_id, true, move_message)
                } else if new_channel_id == ::ChannelId(0) {
                    // Connection disconnected
                    plugin.connection_changed(&mut api, server_id, connection_id, false, move_message);
                    api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
                } else if old_channel_id == new_channel_id {
                    // Connection announced
                    match visibility {
                        Visibility::Enter => {
                            let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                            if let Err(error) = err {
                                error!(api, "Can't get connection information", error);
                            }
                            plugin.connection_announced(&mut api, server_id, connection_id, true);
                        },
                        Visibility::Leave => {
                            plugin.connection_announced(&mut api, server_id, connection_id, false);
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
                    plugin.connection_moved(&mut api, server_id, connection_id,
                        old_channel_id, new_channel_id, visibility, invoker);
                    // Remove the connection if it left visibility
                    if visibility == Visibility::Leave {
                        api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
                    }
                }
            },
            FunctionCall::ConnectionSubscribed(server_id, connection_id, _, _, visibility) => {
                // Connection announced
                match visibility {
                    Visibility::Enter => {
                        let err = api.get_mut_server(server_id).unwrap().add_connection(connection_id);
                        if let Err(error) = err {
                            error!(api, "Can't get connection information", error);
                        }
                        plugin.connection_announced(&mut api, server_id, connection_id, true);
                    },
                    Visibility::Leave => {
                        plugin.connection_announced(&mut api, server_id, connection_id, false);
                        api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
                    },
                    Visibility::Retain => {}
                }
            },
            FunctionCall::ConnectionTimeout(server_id, connection_id, _, _, _, _) => {
                plugin.connection_timeout(&mut api, server_id, connection_id);
                api.get_mut_server(server_id).unwrap().remove_connection(connection_id);
            },
            FunctionCall::ChannelAnnounced(server_id, channel_id, _) => {
                let err = api.get_mut_server(server_id).unwrap().add_channel(channel_id).err();
                if let Some(error) = err {
                    error!(api, "Can't get channel information", error);
                }
                plugin.channel_announced(&mut api, server_id, channel_id);
            }
            FunctionCall::ChannelDescriptionUpdate(server_id, channel_id) => {
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
                plugin.channel_description_updated(&mut api, server_id, channel_id);
            },
            FunctionCall::ChannelUpdate(server_id, channel_id) => {
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
                    plugin.channel_updated(&mut api, server_id, channel_id, old_channel);
                }
            },
            FunctionCall::ChannelCreated(server_id, channel_id, _, invoker) => {
                api.try_update_invoker(server_id, &invoker);
                if let Err(error) = api.get_mut_server(server_id).unwrap()
                    .add_channel(channel_id) {
                    error!(api, "Can't get channel information", error);
                }
                plugin.channel_created(&mut api, server_id, channel_id, invoker);
            },
            FunctionCall::ChannelDeleted(server_id, channel_id, invoker) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.channel_deleted(&mut api, server_id, channel_id, invoker);
                if api.get_mut_server(server_id).and_then(|s| s.remove_channel(channel_id)).is_none() {
                    api.log_or_print("Can't remove channel", "rust-ts3plugin", ::LogLevel::Error)
                }
            },
            FunctionCall::ChannelEdited(server_id, channel_id, invoker) => {
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
                    plugin.channel_edited(&mut api, server_id, channel_id, old_channel,
                                          invoker);
                }
            },
            FunctionCall::ChannelPasswordUpdate(server_id, channel_id) => {
                plugin.channel_password_updated(&mut api, server_id, channel_id);
            },
            FunctionCall::ChannelMove(server_id, channel_id, new_parent_channel_id, invoker) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.channel_moved(&mut api, server_id, channel_id, new_parent_channel_id, invoker);
                if let Some(channel) = api.get_mut_server(server_id).and_then(|s| s.get_mut_channel(channel_id)) {
                    channel.parent_channel_id = new_parent_channel_id;
                }
            },
            FunctionCall::ChannelKick(server_id, connection_id, old_channel_id,
                                      new_channel_id, visibility, invoker, message) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.channel_kick(&mut api, server_id, connection_id, old_channel_id,
                                    new_channel_id, visibility, invoker, message);
                // Remove the kicked connection if it is not visible anymore
                if visibility == ::Visibility::Leave {
                    api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
                } else if let Some(connection) = api.get_mut_server(server_id).and_then(|s|
                    // Update the current channel of the connection
                    s.get_mut_connection(connection_id)) {
                    connection.channel_id = new_channel_id;
                }
            },
            FunctionCall::ServerKick(server_id, connection_id, _, _, _, invoker, message) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.server_kick(&mut api, server_id, connection_id, invoker, message);
                // Remove the kicked connection
                api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
            },
            FunctionCall::ServerBan(server_id, connection_id, _, _, _, invoker, message, time) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.server_ban(&mut api, server_id, connection_id, invoker, message, time);
                // Remove the kicked connection
                api.get_mut_server(server_id).map(|s| s.remove_connection(connection_id));
            },
            FunctionCall::TalkStatusChanged(server_id, connection_id, talking, whispering) => {
                plugin.talking_changed(&mut api, server_id, connection_id, talking, whispering);
                // Update the connection
                if let Some(connection) = api.get_mut_server(server_id).and_then(|s| s.get_mut_connection(connection_id)) {
                    connection.talking = talking;
                    connection.whispering = whispering;
                }
            },
            FunctionCall::AvatarChanged(server_id, connection_id, path) => {
                plugin.avatar_changed(&mut api, server_id, connection_id, path);
            },
            FunctionCall::ConnectionChannelGroupChanged(server_id, connection,
                channel_group_id, channel_id, invoker) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.connection_channel_group_changed(&mut api, server_id,
                    connection, channel_group_id, channel_id, invoker);
            },
            FunctionCall::ConnectionServerGroupAdded(server_id, connection,
                server_group_id, invoker) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.connection_server_group_added(&mut api, server_id,
                    connection, server_group_id, invoker);
            },
            FunctionCall::ConnectionServerGroupRemoved(server_id, connection,
                server_group_id, invoker) => {
                api.try_update_invoker(server_id, &invoker);
                plugin.connection_server_group_removed(&mut api, server_id,
                    connection, server_group_id, invoker);
            },
            FunctionCall::ReturningCall(sender, call) => {
                // Don't forget: The api should not be borrowed mutable
                // to the plugin functions.
                sender.send(match call {
                    ReturningCall::ServerError(server_id, error, message, return_code, extra_message) => {
                        ReturnValue(plugin.server_error(&api, server_id, error, message, return_code, extra_message))
                    },
                    ReturningCall::PermissionError(server_id, permission_id, error, message, return_code) => {
                        ReturnValue(plugin.permission_error(&api, server_id, permission_id, error,
                            message, return_code))
                    },
                    ReturningCall::Message(server_id, target_mode, receiver_id, invoker, message, ignored) => {
                        api.try_update_invoker(server_id, &invoker);
                        let message_receiver = match target_mode {
                            ::TextMessageTargetMode::Client =>
                                ::MessageReceiver::Connection(::ConnectionId(receiver_id)),
                            ::TextMessageTargetMode::Channel => ::MessageReceiver::Channel,
                            ::TextMessageTargetMode::Server => ::MessageReceiver::Server,
                            _ => {
                                api.log_or_print("Got invalid TextMessageTargetMode",
                                                 "rust-ts3plugin", ::LogLevel::Error);
                                ::MessageReceiver::Server
                            }
                        };
                        ReturnValue(plugin.message(&api, server_id, invoker, message_receiver, message, ignored))

                    },
                    ReturningCall::Poke(server_id, invoker, message, ignored) => {
                        api.try_update_invoker(server_id, &invoker);
                        ReturnValue(plugin.poke(&api, server_id, invoker, message, ignored))
                    },
                }).unwrap();
            },
            FunctionCall::Quit => {
                plugin.shutdown(&api);
                break;
            },
        }
    }
    unsafe {
        TX = None;
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
    if let Some(tx) = TX {
        (*tx).send(FunctionCall::Quit).unwrap();
    }
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onConnectStatusChangeEvent(server_id: u64,
    status: c_int, error: c_uint) {
    (*TX.unwrap()).send(FunctionCall::ConnectStatusChange(::ServerId(server_id),
        transmute(status), transmute(error))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerStopEvent(server_id: u64, message: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ServerStop(::ServerId(server_id), to_string!(message)
        )).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerErrorEvent(server_id: u64,
    message: *const c_char, error: c_uint, return_code: *const c_char,
    extra_message: *const c_char) -> c_int {
    let (sender, receiver) = channel();
    (*TX.unwrap()).send(FunctionCall::ReturningCall(sender, ReturningCall::ServerError(::ServerId(server_id),
        transmute(error), to_string!(message), to_string!(return_code),
        to_string!(extra_message)))).unwrap();
    let ReturnValue(b) = receiver.recv().unwrap();
    if b { 1 } else { 0 }
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerEditedEvent(server_id: u64,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ServerEdited(::ServerId(server_id),
        ::Invoker::new(::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerConnectionInfoEvent(server_id: u64) {
    (*TX.unwrap()).send(FunctionCall::ServerConnectionInfo(::ServerId(server_id))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onConnectionInfoEvent(server_id: u64, connection_id: u16) {
    (*TX.unwrap()).send(FunctionCall::ConnectionInfo(::ServerId(server_id),
        ::ConnectionId(connection_id))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateClientEvent(server_id: u64,
    connection_id: u16, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ConnectionUpdated(::ServerId(server_id),
        ::ConnectionId(connection_id), ::Invoker::new(::ConnectionId(invoker_id),
            to_string!(invoker_uid), to_string!(invoker_name)))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveEvent(server_id: u64, connection_id: u16,
    old_channel_id: u64, new_channel_id: u64, visibility: c_int, move_message: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ConnectionMove(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelId(old_channel_id), ::ChannelId(new_channel_id),
        transmute(visibility), to_string!(move_message))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveMovedEvent(server_id: u64, connection_id: u16,
    old_channel_id: u64, new_channel_id: u64, visibility: c_int, invoker_id: u16,
    invoker_name: *const c_char, invoker_uid: *const c_char, move_message: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ConnectionMoved(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelId(old_channel_id), ::ChannelId(new_channel_id),
        transmute(visibility), to_string!(move_message),
        ::Invoker::new(::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveSubscriptionEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int) {
    (*TX.unwrap()).send(FunctionCall::ConnectionSubscribed(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelId(old_channel_id),
        ::ChannelId(new_channel_id), transmute(visibility))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveTimeoutEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
    timeout_message: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ConnectionTimeout(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelId(old_channel_id),
        ::ChannelId(new_channel_id), transmute(visibility), to_string!(timeout_message)
        )).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onNewChannelEvent(server_id: u64, channel_id: u64,
    parent_channel_id: u64) {
    (*TX.unwrap()).send(FunctionCall::ChannelAnnounced(::ServerId(server_id),
        ::ChannelId(channel_id), ::ChannelId(parent_channel_id))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelDescriptionUpdateEvent(server_id: u64,
    channel_id: u64) {
    (*TX.unwrap()).send(FunctionCall::ChannelDescriptionUpdate(::ServerId(server_id),
        ::ChannelId(channel_id))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateChannelEvent(server_id: u64,
    channel_id: u64) {
    (*TX.unwrap()).send(FunctionCall::ChannelUpdate(::ServerId(server_id),
        ::ChannelId(channel_id))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onNewChannelCreatedEvent(server_id: u64,
    channel_id: u64, parent_channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ChannelCreated(::ServerId(server_id),
        ::ChannelId(channel_id), ::ChannelId(parent_channel_id),
        ::Invoker::new(::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onDelChannelEvent(server_id: u64,
    channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ChannelDeleted(::ServerId(server_id),
        ::ChannelId(channel_id), ::Invoker::new(::ConnectionId(invoker_id),
            to_string!(invoker_uid), to_string!(invoker_name)))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onUpdateChannelEditedEvent(server_id: u64,
    channel_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ChannelEdited(::ServerId(server_id),
        ::ChannelId(channel_id), ::Invoker::new(::ConnectionId(invoker_id),
            to_string!(invoker_uid), to_string!(invoker_name)))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelPasswordChangedEvent(server_id: u64,
    channel_id: u64) {
    (*TX.unwrap()).send(FunctionCall::ChannelPasswordUpdate(::ServerId(server_id),
        ::ChannelId(channel_id))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onChannelMoveEvent(server_id: u64,
    channel_id: u64, new_parent_channel_id: u64, invoker_id: u16,
    invoker_name: *const c_char, invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ChannelMove(::ServerId(server_id),
        ::ChannelId(channel_id), ::ChannelId(new_parent_channel_id),
        ::Invoker::new(::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)))).unwrap();
}

// Ignore clippy warnings, we can't change the TeamSpeak interface
#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onTextMessageEvent(server_id: u64,
    target_mode: u16, receiver_id: u16, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char, message: *const c_char, ignored: c_int) -> c_int {
    let (sender, receiver) = channel();
    (*TX.unwrap()).send(FunctionCall::ReturningCall(sender,
        ReturningCall::Message(::ServerId(server_id), transmute(target_mode as i32),
        receiver_id, ::Invoker::new(::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)), to_string!(message),
        ignored != 0))).unwrap();
    let ReturnValue(b) = receiver.recv().unwrap();
    if b { 1 } else { 0 }
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientPokeEvent(server_id: u64,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
    message: *const c_char, ignored: c_int) -> c_int {
    let (sender, receiver) = channel();
    (*TX.unwrap()).send(FunctionCall::ReturningCall(sender,
        ReturningCall::Poke(::ServerId(server_id),
        ::Invoker::new(::ConnectionId(invoker_id), to_string!(invoker_uid),
        to_string!(invoker_name)), to_string!(message), ignored != 0))).unwrap();
    let ReturnValue(b) = receiver.recv().unwrap();
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
    (*TX.unwrap()).send(FunctionCall::ChannelKick(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelId(old_channel_id),
        ::ChannelId(new_channel_id), transmute(visibility), ::Invoker::new(
            ::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)), to_string!(message)
        )).unwrap();
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientKickFromServerEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
    message: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ServerKick(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelId(old_channel_id),
        ::ChannelId(new_channel_id), transmute(visibility), ::Invoker::new(
            ::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)), to_string!(message)
        )).unwrap();
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientBanFromServerEvent(server_id: u64,
    connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int,
    invoker_id: u16, invoker_name: *const c_char, invoker_uid: *const c_char,
    time: u64, message: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ServerBan(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelId(old_channel_id),
        ::ChannelId(new_channel_id), transmute(visibility), ::Invoker::new(
            ::ConnectionId(invoker_id), to_string!(invoker_uid),
            to_string!(invoker_name)), to_string!(message), time
        )).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onTalkStatusChangeEvent(server_id: u64,
    talking: c_int, whispering: c_int, connection_id: u16) {
    (*TX.unwrap()).send(FunctionCall::TalkStatusChanged(::ServerId(server_id),
        ::ConnectionId(connection_id), transmute(talking), whispering != 0)).unwrap();
}


#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onAvatarUpdated(server_id: u64,
    connection_id: u16, avatar_path: *const c_char) {
    let path = if avatar_path.is_null() { None } else { Some(to_string!(avatar_path)) };
    (*TX.unwrap()).send(FunctionCall::AvatarChanged(::ServerId(server_id),
        ::ConnectionId(connection_id), path)).unwrap();
}
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientChannelGroupChangedEvent(server_id: u64,
    channel_group_id: u64, channel_id: u64, connection_id: u16, invoker_id: u16,
    invoker_name: *const c_char, invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ConnectionChannelGroupChanged(::ServerId(server_id),
        ::ConnectionId(connection_id), ::ChannelGroupId(channel_group_id),
        ::ChannelId(channel_id), ::Invoker::new(::ConnectionId(invoker_id),
        to_string!(invoker_uid), to_string!(invoker_name)))).unwrap();
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerGroupClientAddedEvent(server_id: u64,
    connection_id: u16, connection_name: *const c_char, connection_uid: *const c_char,
    server_group_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ConnectionServerGroupAdded(
        ::ServerId(server_id), ::Invoker::new(::ConnectionId(connection_id),
        to_string!(connection_uid), to_string!(connection_name)),
        ::ServerGroupId(server_group_id), ::Invoker::new(::ConnectionId(invoker_id),
        to_string!(invoker_uid), to_string!(invoker_name)))).unwrap();
}

#[cfg_attr(feature="clippy", allow(too_many_arguments))]
#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerGroupClientDeletedEvent(server_id: u64,
    connection_id: u16, connection_name: *const c_char, connection_uid: *const c_char,
    server_group_id: u64, invoker_id: u16, invoker_name: *const c_char,
    invoker_uid: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ConnectionServerGroupRemoved(
        ::ServerId(server_id), ::Invoker::new(::ConnectionId(connection_id),
        to_string!(connection_uid), to_string!(connection_name)),
        ::ServerGroupId(server_group_id), ::Invoker::new(::ConnectionId(invoker_id),
        to_string!(invoker_uid), to_string!(invoker_name)))).unwrap();
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onServerPermissionErrorEvent(server_id: u64,
    message: *const c_char, error: c_uint, return_code: *const c_char,
    permission_id: c_uint) -> c_int {
    let (sender, receiver) = channel();
    (*TX.unwrap()).send(FunctionCall::ReturningCall(sender,
        ReturningCall::PermissionError(::ServerId(server_id),
        ::PermissionId(permission_id), transmute(error), to_string!(message),
        to_string!(return_code)))).unwrap();
    let ReturnValue(b) = receiver.recv().unwrap();
    if b { 1 } else { 0 }
}
