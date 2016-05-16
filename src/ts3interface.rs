use libc::{c_char, c_int, c_uint};
use std::ffi::CStr;
use std::mem::transmute;
use std::sync::mpsc::{channel, Sender};

use ts3plugin_sys::clientlib_publicdefinitions::*;
use ts3plugin_sys::public_errors::Error;
use ts3plugin_sys::ts3functions::Ts3Functions;

use ::plugin::Plugin;

static mut TX: Option<*const Sender<FunctionCall>> = None;

enum FunctionCall {
    ConnectStatusChange(::ServerId, ConnectStatus, Error),
    ChannelAnnounced(::ServerId, ::ChannelId, ::ChannelId),
    ClientMove(::ServerId, ::ConnectionId, ::ChannelId, ::ChannelId, Visibility, String),
    Quit
}

/// Manager thread
#[doc(hidden)]
pub fn manager_thread(mut plugin: Box<Plugin>, main_transmitter: Sender<()>, mut api: ::TsApi) {
    let (tx, rx) = channel();
    unsafe {
        TX = Some(&tx)
    }
    // Send that we are ready
    main_transmitter.send(()).unwrap();

    // Wait for messages
    loop {
        match rx.recv().unwrap() {
            FunctionCall::ConnectStatusChange(server_id, status, error) => {
                // Add the server if we can get information about it
                // and don't have that server cached already.
                if status != ConnectStatus::Connecting && api.get_server(server_id).is_none() {
                    if let Err(error) = api.add_server(server_id) {
                        api.log_or_print(format!("Can't get server information: {:?}", error), "rust-ts3plugin", ::LogLevel::Error)
                    }
                }
                // Execute plugin callback
                plugin.connect_status_change(&mut api, server_id, status, error);
                // Remove server if we disconnected
                if status == ConnectStatus::Disconnected {
                    api.remove_server(server_id);
                }
            },
            FunctionCall::ChannelAnnounced(server_id, channel_id, _) => {
                let err = {
	            	let server = api.get_mut_server(server_id).unwrap();
                    server.add_channel(channel_id).err()
                };
                if let Some(error) = err {
                    api.log_or_print(format!("Can't get channel information: {:?}", error), "rust-ts3plugin", ::LogLevel::Error)
                }
                plugin.channel_announced(&mut api, server_id, channel_id);
            }
            FunctionCall::ClientMove(server_id, connection_id, old_channel_id, new_channel_id, visibility, move_message) => {
                if old_channel_id == ::ChannelId(0) {
                    //TODO Don't send own connects and disconnects
                    // Client connected
                    let err = {
                        let server = api.get_mut_server(server_id).unwrap();
                        server.add_connection(connection_id).err()
                    };
                    if let Some(error) = err {
                        api.log_or_print(format!("Can't get connection information: {:?}", error), "rust-ts3plugin", ::LogLevel::Error)
                    }
                    plugin.connection_changed(&mut api, server_id, connection_id, true, move_message)
                } else if new_channel_id == ::ChannelId(0) {
                    // Client disconnected
                    plugin.connection_changed(&mut api, server_id, connection_id, false, move_message)
                } else if old_channel_id == new_channel_id {
                    // Client announced
                    plugin.connection_announced(&mut api, server_id, connection_id)
                } else {
                    // Client switched channel
                    plugin.connection_moved(&mut api, server_id, connection_id, old_channel_id, new_channel_id)
                }
            },
            FunctionCall::Quit => {
                plugin.shutdown(&api);
                break
            },
        }
    }
    unsafe {
        TX = None
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
pub unsafe extern "C" fn ts3plugin_onConnectStatusChangeEvent(server_id: u64, status: c_int, error: c_uint) {
    (*TX.unwrap()).send(FunctionCall::ConnectStatusChange(::ServerId(server_id), transmute(status), transmute(error))).unwrap()
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onNewChannelEvent(server_id: u64, channel_id: u64, parent_channel_id: u64) {
    (*TX.unwrap()).send(FunctionCall::ChannelAnnounced(::ServerId(server_id), ::ChannelId(channel_id), ::ChannelId(parent_channel_id))).unwrap()
}

#[allow(non_snake_case)]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn ts3plugin_onClientMoveEvent(server_id: u64, connection_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int, move_message: *const c_char) {
    (*TX.unwrap()).send(FunctionCall::ClientMove(::ServerId(server_id), ::ConnectionId(connection_id), ::ChannelId(old_channel_id), ::ChannelId(new_channel_id), transmute(visibility), to_string!(move_message))).unwrap()
}
