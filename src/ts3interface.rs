use libc::{c_int, c_uint};
use std::mem::transmute;
use std::sync::mpsc::{channel, Sender};

use ts3plugin_sys::ts3functions::Ts3Functions;

use ::plugin::Plugin;

/// The api functions provided by TeamSpeak
static mut TX: Option<*const Sender<FunctionCall>> = None;

enum FunctionCall {
	ConnectStatusChange(u64, c_int, c_uint),
	Quit
}

// Manager thread
pub fn manager_thread(plugin: &mut Plugin, main_transmitter: Sender<()>) {
	let (tx, rx) = channel();
	unsafe {
		TX = Some(&tx);
	}
	// Send that we are ready
	main_transmitter.send(()).unwrap();

	// Wait for messages
	loop {
		match rx.recv().unwrap() {
			FunctionCall::ConnectStatusChange(connection, status, error) =>
				match ::Server::new(connection) {
					Ok(server) => unsafe {
						plugin.connect_status_change(server, transmute(status), transmute(error))
					},
					Err(error) => ::TsApi::log_or_print(format!("Can't get server: {:?}", error).as_ref(), "rust-ts3plugin", ::LogLevel::Error)
				},
			FunctionCall::Quit => break
		}
	}
    /*(*::plugin.expect("Plugin should be loaded")).on_connect_status_change(
        ::Server { id: sc_handler_id },
        transmute(new_status),
        transmute(error_number));*/
}

pub unsafe fn quit_manager_thread() {
	(*TX.unwrap()).send(FunctionCall::Quit).unwrap();
}

// ************************** Interface for TeamSpeak **************************

#[allow(non_snake_case)]
#[no_mangle]
pub extern fn ts3plugin_apiVersion() -> c_int {
    20
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_setFunctionPointers(funs: Ts3Functions) {
    ::ts3functions = Some(funs);
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern fn ts3plugin_onConnectStatusChangeEvent(connection: u64, status: c_int, error: c_uint) {
	(*TX.unwrap()).send(FunctionCall::ConnectStatusChange(connection, status, error)).unwrap()
}
