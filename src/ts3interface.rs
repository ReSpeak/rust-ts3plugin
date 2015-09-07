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
        ::Channel { id: new_channel_id, server: server.clone() },
        transmute(visibility),
        &message);
}
