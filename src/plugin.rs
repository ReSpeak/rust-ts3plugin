pub use libc::{c_char, c_int};

use std::thread::JoinHandle;

#[derive(Debug)]
pub enum InitError {
    /// Initialisation failed, the plugin will be unloaded again
    Failure,
    /// Like `Failure`, but the client will not show a "failed to load" warning.
    /// This is a very special case and should only be used if a plugin displays
    /// a dialog (e.g. overlay) asking the user to disable the plugin again,
    /// avoiding the show another dialog by the client telling the user the
    /// plugin failed to load.
    /// For normal case, if a plugin really failed to load because of an error,
    /// the correct return value is `Failure`.
    FailureNoMessage
}

#[allow(unused_variables)]
pub trait Plugin : Drop {
    // ************************** Required functions ***************************
    // Custom code called right after loading the plugin.
    //fn new() -> Result<Box<Self>, InitError>;
    fn connect_status_change(&mut self, server: ::Server, status:
        ::ConnectStatus, error: ::Error) {}
}

/// Manager thread that calls all plugin functions.
pub static mut MANAGER_THREAD: Option<*mut JoinHandle<()>> = None;

/// Create a plugin. This macro has to be called once per library to create the
/// function interface that is used by TeamSpeak.
///
/// # Arguments
///
///  - name         - The name of the plugin as displayed in TeamSpeak
///  - version      - The version of the plugin as displayed in TeamSpeak
///  - author       - The author of the plugin as displayed in TeamSpeak
///  - description  - The description of the plugin as displayed in TeamSpeak
///  - configurable - If the plugin offers the possibility to be configured
///  - autoload     - If the plugin should be loaded by default or only if
///                   activated manually
///  - typename     - The type of the class that implements the plugin and has a
///                   `new()`-function
///
/// # Examples
///
/// Create an example plugin with a given name, version, author, description and
/// a struct `MyTsPlugin` that implements the `Plugin` trait:
///
/// ```ignore
/// create_plugin!("My Ts Plugin", "0.1.0", "My Name",
///     "A wonderful tiny example plugin", ConfigureOffer::No, false, MyTsPlugin);
/// ```
#[macro_export]
macro_rules! create_plugin {
    ($name: expr, $version: expr, $author: expr, $description: expr,
        $configurable: expr, $autoload: expr, $typename: ident) => {
	    lazy_static! {
	    	static ref PLUGIN_NAME: std::ffi::CString = std::ffi::CString::new($name).unwrap();
	    	static ref PLUGIN_VERSION: std::ffi::CString = std::ffi::CString::new($version).unwrap();
	    	static ref PLUGIN_AUTHOR: std::ffi::CString = std::ffi::CString::new($author).unwrap();
	    	static ref PLUGIN_DESCRIPTION: std::ffi::CString = std::ffi::CString::new($description).unwrap();
	    }

		/// The used plugin
		static mut PLUGIN: Option<*mut $typename> = None;

        #[no_mangle]
        pub unsafe extern fn ts3plugin_init() -> c_int {
            // Delete the old instance if one exists
            if let Some(instance) = PLUGIN {
                drop(Box::from_raw(instance));
                PLUGIN = None;
            }

            // Create a new plugin instance
            match $typename::new() {
            	Ok(plugin) => {
            		let ptr = Box::into_raw(plugin);
            		PLUGIN = Some(ptr);
            		let (transmitter, receiver) = std::sync::mpsc::channel();
            		// Start manager thread
            		MANAGER_THREAD = Some(Box::into_raw(Box::new(
            			std::thread::spawn(move || $crate::ts3interface::manager_thread(
            				&mut *PLUGIN.unwrap(), transmitter)))));
            		// Wait until manager thread started up
            		receiver.recv().unwrap();
            		0
            	},
            	Err(error) => match error {
		        	$crate::InitError::Failure =>          1,
		        	$crate::InitError::FailureNoMessage => -2
            	}
            }
        }

		/// Unique name identifying this plugin.
		/// The result of this function has to be a null-terminated static string.
		/// Can be called before init.
		#[no_mangle]
		pub unsafe extern fn ts3plugin_name() -> *const c_char {
			(*PLUGIN_NAME).as_ptr()
		}

		/// The version of the plugin.
		/// Can be called before init.
		#[no_mangle]
		pub unsafe extern fn ts3plugin_version() -> *const c_char {
			(*PLUGIN_VERSION).as_ptr()
		}

		/// The author of the plugin.
		/// Can be called before init.
		#[no_mangle]
		pub unsafe extern fn ts3plugin_author() -> *const c_char {
			(*PLUGIN_AUTHOR).as_ptr()
		}

		/// The desription of the plugin.
		/// Can be called before init.
		#[no_mangle]
		pub unsafe extern fn ts3plugin_description() -> *const c_char {
			(*PLUGIN_DESCRIPTION).as_ptr()
		}

		/// If the plugin offers the possibility to be configured by the user.
		/// Can be called before init.
		#[allow(non_snake_case)]
		#[no_mangle]
		pub unsafe extern fn ts3plugin_offersConfigure() -> c_int {
			$configurable as c_int
		}

		/// If the plugin should be loaded automatically.
		/// Can be called before init.
		#[allow(non_snake_case)]
		#[no_mangle]
		pub unsafe extern fn ts3plugin_requestAutoload() -> c_int {
			if $autoload {
				1
			} else {
				0
			}
		}

        #[no_mangle]
        pub unsafe extern fn ts3plugin_shutdown() {
        	if let Some(plugin) = PLUGIN {
	        	// Stop manager thread and wait for the end
	        	$crate::ts3interface::quit_manager_thread();
	        	let manager_thread = *Box::from_raw(MANAGER_THREAD.take().unwrap());
	        	if let Err(error) = manager_thread.join() {
	        		$crate::TsApi::log_or_print(format!("Can't wait for manager thread: {:?}", error).as_ref(), "rust-ts3plugin", $crate::LogLevel::Error);
	        	}
            	drop(Box::from_raw(plugin));
	            PLUGIN = None;
            }
        }
    };
}
