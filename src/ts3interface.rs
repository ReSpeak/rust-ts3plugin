use libc::*;

use std::ffi::*;
use std::mem::transmute;
use ts3plugin_sys::ts3functions::Ts3Functions;

/// This variables will be exported by the library that finally implements a plugin.
// Silence a warning that comes without reason and isn't fixable
#[allow(improper_ctypes)]
extern
{
    /// Use the macro `create_plugin` to export the name, etc. of the plugin.
    static PLUGIN_DATA: ::PluginData;

    /// Create an instance of the plugin.
    #[no_mangle]
    fn create_instance() -> *mut ::Plugin;
    /// Remove an instance of the plugin.
    #[no_mangle]
    fn remove_instance(instance: *mut ::Plugin);
}

/// We have to manually create and delete this at `init` and `shutdown` by using
/// `create_instance` and `remove_instance`.
static mut plugin: Option<*mut ::Plugin> = None;

/// The api functions provided by TeamSpeak
pub static mut ts3functions: Option<Ts3Functions> = None;

// ************************** Interface for TeamSpeak **************************

/// Unique name identifying this plugin.
/// The result of this function has to be a null-terminated static string.
/// Can be called before init.
///
/// # Examples
///
/// Declare a static null-terminated string:
///
/// ```
/// let text: &'static str = "TEXT\0";
/// ```
///
/// Simply return a string:
///
/// ```
/// # fn get_name() -> &'static str
/// # {
/// "TEXT\0"
/// # }
/// ```
#[no_mangle]
pub unsafe extern fn ts3plugin_name() -> *const c_char
{
    PLUGIN_DATA.name.as_ptr() as *const c_char
}

/// The version of the plugin.
/// Can be called before init.
#[no_mangle]
pub unsafe extern fn ts3plugin_version() -> *const c_char
{
    PLUGIN_DATA.version.as_ptr() as *const c_char
}

/// The author of the plugin.
/// Can be called before init.
#[no_mangle]
pub unsafe extern fn ts3plugin_author() -> *const c_char
{
    PLUGIN_DATA.author.as_ptr() as *const c_char
}

/// The desription of the plugin.
/// Can be called before init.
#[no_mangle]
pub unsafe extern fn ts3plugin_description() -> *const c_char
{
    PLUGIN_DATA.description.as_ptr() as *const c_char
}

/// If the plugin offers the possibility to be configured by the user.
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_offersConfigure() -> c_int
{
    PLUGIN_DATA.configurable as c_int
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_requestAutoload() -> c_int
{
    if PLUGIN_DATA.autoload
    {
        1
    } else
    {
        0
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern fn ts3plugin_apiVersion() -> c_int
{
    20
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_setFunctionPointers(funs: Ts3Functions)
{
    ts3functions = Some(funs);
}

#[no_mangle]
pub unsafe extern fn ts3plugin_init() -> c_int
{
    // Delete the old instance if one exists
    if let Some(instance) = plugin
    {
        remove_instance(instance);
        plugin = None;
    }

    // Create a new plugin instance
    plugin = Some(create_instance());
    match (*plugin.expect("Plugin should be loaded")).init()
    {
        ::InitResult::Success          => 0,
        ::InitResult::Failure          => 1,
        ::InitResult::FailureNoMessage => -2
    }
}

#[no_mangle]
pub unsafe extern fn ts3plugin_shutdown()
{
    (*plugin.expect("Plugin should be loaded")).shutdown();
    remove_instance(plugin.unwrap());
    plugin = None;
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_onConnectStatusChangeEvent(sc_handler_id: u64, new_status: c_int, error_number: c_uint)
{
    (*plugin.expect("Plugin should be loaded")).on_connect_status_change(
        ::Server { id: sc_handler_id },
        transmute(new_status),
        transmute(error_number));
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_onClientMoveEvent(sc_handler_id: u64, client_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int, move_message: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    let message = String::from_utf8_lossy(CStr::from_ptr(move_message).to_bytes()).into_owned();
    (*plugin.expect("Plugin should be loaded")).on_client_move(
        ::Connection { id: client_id, server: server.clone() },
        ::Channel { id: old_channel_id, server: server.clone() },
        ::Channel { id: new_channel_id, server: server },
        transmute(visibility),
        message);
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onClientMoveMovedEvent(sc_handler_id: u64, client_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int, invoker_id: u16, _: *const c_char, _: *const c_char, move_message: *const c_char)
{
    let message = String::from_utf8_lossy(CStr::from_ptr(move_message).to_bytes()).into_owned();
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_client_move_moved(
        ::Connection { id: client_id, server: server.clone() },
        ::Channel { id: old_channel_id, server: server.clone() },
        ::Channel { id: new_channel_id, server: server.clone() },
        transmute(visibility),
        ::Connection { id: invoker_id, server: server },
        message);
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_onClientMoveTimeoutEvent(sc_handler_id: u64, client_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int, move_message: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    let message = String::from_utf8_lossy(CStr::from_ptr(move_message).to_bytes()).into_owned();
    (*plugin.expect("Plugin should be loaded")).on_client_move_timeout(
        ::Connection { id: client_id, server: server.clone() },
        ::Channel { id: old_channel_id, server: server.clone() },
        ::Channel { id: new_channel_id, server: server },
        transmute(visibility),
        message);
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_onClientMoveSubscriptionEvent(sc_handler_id: u64, client_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_client_move_subscription(
        ::Connection { id: client_id, server: server.clone() },
        ::Channel { id: old_channel_id, server: server.clone() },
        ::Channel { id: new_channel_id, server: server },
        transmute(visibility));
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_onTalkStatusChangeEvent(sc_handler_id: u64, status: c_int, is_received_whisper: c_int, client_id: u16)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_talk_status_change(
        ::Connection { id: client_id, server: server },
        transmute(status),
        is_received_whisper != 0);
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onUpdateChannelEditedEvent(sc_handler_id: u64, channel_id: u64, invoker_id: u16, _: *const c_char, _: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_update_channel_edited(
        ::Channel { id: channel_id, server: server.clone() },
        ::Connection { id: invoker_id, server: server });
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onUpdateClientEvent(sc_handler_id: u64, client_id: u16, invoker_id: u16, _: *const c_char, _: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_update_client(
        ::Connection { id: client_id, server: server.clone() },
        ::Connection { id: invoker_id, server: server });
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: from_name and from_uid
pub unsafe extern fn ts3plugin_onTextMessageEvent(sc_handler_id: u64, target_mode: u16, to_id: u16, from_id: u16, _: *const c_char, _: *const c_char, message: *const c_char, ff_ignored: c_int) -> c_int
{
    let server = ::Server { id: sc_handler_id };
    let message = String::from_utf8_lossy(CStr::from_ptr(message).to_bytes()).into_owned();
    if (*plugin.expect("Plugin should be loaded")).on_text_message(
        transmute(target_mode as i32),
        ::Connection { id: from_id, server: server.clone() },
        ::Connection { id: to_id, server: server },
        message,
        ff_ignored != 0)
    {
        1
    } else
    {
        0
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_currentServerConnectionChanged(sc_handler_id: u64)
{
    (*plugin.expect("Plugin should be loaded")).on_current_server_connection_changed(
        ::Server { id: sc_handler_id });
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_onNewChannelEvent(sc_handler_id: u64, channel_id: u64, channel_parent_id: u64)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_new_channel(
        ::Channel { id: channel_id, server: server.clone() },
        ::Channel { id: channel_parent_id, server: server });
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onNewChannelCreatedEvent(sc_handler_id: u64, channel_id: u64, channel_parent_id: u64, invoker_id: u16, _: *const c_char, _: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_new_channel_created(
        ::Channel { id: channel_id, server: server.clone() },
        ::Channel { id: channel_parent_id, server: server.clone() },
        ::Connection { id: invoker_id, server: server });
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onDelChannelEvent(sc_handler_id: u64, channel_id: u64, invoker_id: u16, _: *const c_char, _: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_delete_channel(
        ::Channel { id: channel_id, server: server.clone() },
        ::Connection { id: invoker_id, server: server });
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onChannelMoveEvent(sc_handler_id: u64, channel_id: u64, new_channel_parent_id: u64, invoker_id: u16, _: *const c_char, _: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_channel_move(
        ::Channel { id: channel_id, server: server.clone() },
        ::Channel { id: new_channel_parent_id, server: server.clone() },
        ::Connection { id: invoker_id, server: server });
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onClientKickFromChannelEvent(sc_handler_id: u64, client_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int, invoker_id: u16, _: *const c_char, _: *const c_char, kick_message: *const c_char)
{
    let message = String::from_utf8_lossy(CStr::from_ptr(kick_message).to_bytes()).into_owned();
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_client_kick_from_channel(
        ::Connection { id: client_id, server: server.clone() },
        ::Channel { id: old_channel_id, server: server.clone() },
        ::Channel { id: new_channel_id, server: server.clone() },
        transmute(visibility),
        ::Connection { id: invoker_id, server: server },
        message);
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onClientKickFromServerEvent(sc_handler_id: u64, client_id: u16, old_channel_id: u64, new_channel_id: u64, visibility: c_int, invoker_id: u16, _: *const c_char, _: *const c_char, kick_message: *const c_char)
{
    let message = String::from_utf8_lossy(CStr::from_ptr(kick_message).to_bytes()).into_owned();
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_client_kick_from_server(
        ::Connection { id: client_id, server: server.clone() },
        ::Channel { id: old_channel_id, server: server.clone() },
        ::Channel { id: new_channel_id, server: server.clone() },
        transmute(visibility),
        ::Connection { id: invoker_id, server: server },
        message);
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: client_name, client_uid, invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onServerGroupClientAddedEvent(sc_handler_id: u64, client_id: u16, _: *const c_char, _: *const c_char, server_group_id: u64, invoker_id: u16, _: *const c_char, _: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_server_group_client_added(
        ::Connection { id: client_id, server: server.clone() },
        ::ServerGroup { id: server_group_id, server: server.clone() },
        ::Connection { id: invoker_id, server: server });
}

#[allow(non_snake_case)]
#[no_mangle]
// Omitted arguments: client_name, client_uid, invoker_name and invoker_uid
pub unsafe extern fn ts3plugin_onServerGroupClientDeletedEvent(sc_handler_id: u64, client_id: u16, _: *const c_char, _: *const c_char, server_group_id: u64, invoker_id: u16, _: *const c_char, _: *const c_char)
{
    let server = ::Server { id: sc_handler_id };
    (*plugin.expect("Plugin should be loaded")).on_server_group_client_deleted(
        ::Connection { id: client_id, server: server.clone() },
        ::ServerGroup { id: server_group_id, server: server.clone() },
        ::Connection { id: invoker_id, server: server });
}
