use ts3plugin_sys::ts3functions::Ts3Functions;
use ::plugin::*;

/// The api functions provided by TeamSpeak
pub static mut ts3functions: Option<Ts3Functions> = None;

// Manager thread
pub fn manager_thread(plugin: &mut Plugin) {
	
}

// ************************** Interface for TeamSpeak **************************

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
