#[doc(no_inline)]
pub use libc::{c_char, c_int};

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

/// The trait that has to be implemented by a plugin. To enhance a library to a
/// working TeamSpeak plugin you have to call the macro `create_plugin!`
/// afterwards.
#[allow(unused_variables)]
pub trait Plugin {
    // ************************** Required functions ***************************
    // Custom code called right after loading the plugin.
    fn new(&api: &::TsApi) -> Result<Box<Self>, InitError> where Self: Sized;

    /// If the connection status changes.
    /// If `status = ConnectStatus::Connecting`, the connection_id is not yet
    /// registered in the `TsApi`.
    fn connect_status_change(&mut self, api: &mut ::TsApi, server_id: ::ServerId, status:
        ::ConnectStatus, error: ::Error) {}

    fn server_stop(&mut self, api: &mut ::TsApi, server_id: ::ServerId, message: String) {}

    fn server_error(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        error: ::Error, message: String, return_code: String, extra_message: String) {}

    fn server_edited(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        invoker: ::Invoker) {}

    fn connection_updated(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId, old_connection: Option<::Connection>, invoker: ::Invoker) {}

    /// If the plugin was informed about a new connection. If appeared is true, the connection
    /// was previously not known to the plugin, if appeared is false, the connection left
    /// the view of connection.
    fn connection_announced(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId, appeared: bool) {}

    /// Called, if a connection connects to the server. This is also called for our own
    /// connection.
    fn connection_changed(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId, connected: bool, message: String) {}

    fn connection_moved(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId, old_channel_id: ::ChannelId,
        new_channel_id: ::ChannelId, visibility: ::Visibility) {}

    fn connection_timeout(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId) {}

    fn channel_announced(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId) {}

    fn channel_description_updated(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId) {}

    fn channel_updated(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId, old_channel: Option<::Channel>) {}

    fn channel_created(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId, invoker: ::Invoker) {}

    fn channel_deleted(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId, invoker: ::Invoker) {}

    fn channel_edited(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId, old_channel: Option<::Channel>, invoker: ::Invoker) {}

    fn channel_password_updated(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId) {}

    /// The current parent id of the channel is the old one, the new
    /// parent id is given as a parameter.
    fn channel_moved(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        channel_id: ::ChannelId, new_parent_channel_id: ::ChannelId,
        invoker: ::Invoker) {}

    fn message(&mut self, api: &mut ::TsApi, server_id: ::ServerId, invoker: ::Invoker,
        target: ::MessageReceiver, message: String) {}

    fn channel_kick(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId, old_channel_id: ::ChannelId, new_channel_id: ::ChannelId,
        visibility: ::Visibility, invoker: ::Invoker, message: String) {}

    fn server_kick(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId, invoker: ::Invoker, message: String) {}

    /// The old values of `talking` and `whispering` are available from the connection.
    /// They will be updated after this functions returned.
    fn talking_changed(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
        connection_id: ::ConnectionId, talking: ::TalkStatus, whispering: bool) {}

    /// Called if the plugin is disabled (either by the user or if TeamSpeak is
    /// exiting).
    fn shutdown(&mut self, api: &::TsApi) {}
}

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

        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn ts3plugin_init() -> c_int {
            // Create TsApi
            let mut api = $crate::TsApi::new();
            // And load all currently available data.
            match api.load() {
                Ok(_) => {
                    // Create a new plugin instance
                    match $typename::new(&api) {
                        Ok(plugin) => {
                            let (transmitter, receiver) = std::sync::mpsc::channel();
                            // Start manager thread
                            std::thread::spawn(move || $crate::ts3interface::manager_thread(
                                plugin, transmitter, api));
                            // Wait until manager thread started up
                            match receiver.recv() {
                                Ok(_) => 0,
                                Err(error) => {
                                    println!("Can't start manager thread: {:?}", error);
                                    1
                                }
                            }
                        },
                        Err(error) => match error {
                            $crate::InitError::Failure =>           1,
                            $crate::InitError::FailureNoMessage => -2
                        }
                    }
                },
                Err(error) => {
                    api.log_or_print(format!(
                        "Can't create TsApi: {:?}", error),
                        "rust-ts3plugin", $crate::LogLevel::Error);
                    1
                }
            }
        }

        /// Unique name identifying this plugin.
        /// The result of this function has to be a null-terminated static string.
        /// Can be called before init.
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn ts3plugin_name() -> *const c_char {
            (*PLUGIN_NAME).as_ptr()
        }

        /// The version of the plugin.
        /// Can be called before init.
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn ts3plugin_version() -> *const c_char {
            (*PLUGIN_VERSION).as_ptr()
        }

        /// The author of the plugin.
        /// Can be called before init.
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn ts3plugin_author() -> *const c_char {
            (*PLUGIN_AUTHOR).as_ptr()
        }

        /// The desription of the plugin.
        /// Can be called before init.
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn ts3plugin_description() -> *const c_char {
            (*PLUGIN_DESCRIPTION).as_ptr()
        }

        /// If the plugin offers the possibility to be configured by the user.
        /// Can be called before init.
        #[allow(non_snake_case)]
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn ts3plugin_offersConfigure() -> c_int {
            $configurable as c_int
        }

        /// If the plugin should be loaded automatically.
        /// Can be called before init.
        #[allow(non_snake_case)]
        #[no_mangle]
        #[doc(hidden)]
        pub extern "C" fn ts3plugin_requestAutoload() -> c_int {
            if $autoload {
                1
            } else {
                0
            }
        }
    };
}
