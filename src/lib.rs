extern crate libc;
extern crate ts3plugin_sys;

pub mod ts3interface;

use libc::*;
use std::ffi::*;

pub use ts3plugin_sys::clientlib_publicdefinitions::*;
pub use ts3plugin_sys::plugin_definitions::*;
pub use ts3plugin_sys::public_definitions::*;
pub use ts3plugin_sys::public_errors::Error;
pub use ts3plugin_sys::ts3functions::Ts3Functions;

// Helper functions

/// Converts a normal string to a CString
macro_rules! to_cstring
{
    ($string: expr) =>
    {
        CString::new($string).unwrap_or(
            CString::new("String contains null character").unwrap())
    };
}


#[derive(Debug)]
pub enum InitResult
{
    /// Successful initialisation
    Success,
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
///
/// # Examples
///
/// A fully working example that creates a plugin that does nothing:
///
/// ```
/// #[macro_use]
/// extern crate ts3plugin;
///
/// use ts3plugin::*;
///
/// struct MyTsPlugin;
///
/// impl MyTsPlugin
/// {
///     fn new() -> MyTsPlugin
///     {
///         MyTsPlugin
///     }
/// }
///
/// impl Plugin for MyTsPlugin
/// {
///     fn init(&mut self) -> InitResult
///     {
///         println!("Inited");
///         InitResult::Success
///     }
///
///     fn shutdown(&mut self)
///     {
///         println!("Shutdown");
///     }
/// }
///
/// create_plugin!("My Ts Plugin\0", "0.1.0\0", "My Name\0",
///     "A wonderful tiny example plugin\0", ConfigureOffer::No, MyTsPlugin);
/// # fn main() {}
/// ```
#[allow(unused_variables)]
pub trait Plugin
{
    // ************************** Required functions ***************************
    /// Custom code called right after loading the plugin.
    fn init(&mut self) -> InitResult;

    /// Custom code called right before the plugin is unloaded.
    ///
    /// Note:
    ///
    /// If your plugin implements a settings dialog, it must be closed and
    /// deleted here, else the TeamSpeak client will most likely crash (library
    /// removed but dialog from the library code is still used).
    fn shutdown(&mut self);


    // ******************************* Callbacks *******************************
    /// Plugin requests to be always automatically loaded by the TeamSpeak
    /// client unless the user manually disabled it in the plugin dialog.
    /// The default value is `false` which means the plugin won't be loaded
    /// automatically.
    fn request_autoload(&self) -> bool { false }
    fn on_connect_status_change(&mut self, server: Server, new_status:
        ConnectStatus, error_number: Error) {}
    fn on_client_move(&mut self, client: Connection, old_channel: Channel,
        new_channel: Channel, visibility: Visibility, move_message: String) {}
    fn on_client_move_moved(&mut self, client: Connection, old_channel: Channel,
        new_channel: Channel, visibility: Visibility, invoker: Connection,
        move_message: String) {}
    fn on_client_move_timeout(&mut self, client: Connection, old_channel: Channel,
        new_channel: Channel, visibility: Visibility, move_message: String) {}
    fn on_client_move_subscription(&mut self, client: Connection, old_channel: Channel,
        new_channel: Channel, visibility: Visibility) {}
    fn on_talk_status_change(&mut self, client: Connection, status: TalkStatus,
        is_received_whisper: bool) {}
    fn on_update_channel_edited(&mut self, channel: Channel, invoker: Connection) {}
    fn on_update_client(&mut self, client: Connection, invoker: Connection) {}
    /// `ff_ignored` is `true` if the friend/foe manager ignored that message.
    ///
    /// If `true` is returned, the TeamSpeak client will ignore this message, if
    /// `false` is returned, it will be handled normally.
    fn on_text_message(&mut self, mode: TextMessageTargetMode, sender: Connection,
        receiver: Connection, message: String, ff_ignored: bool) -> bool
    {
        false
    }
    /// Client changed current server connection handler.
    fn on_current_server_connection_changed(&mut self, server: Server) {}
    /// After a connection has been established, all current channels on the
    // server are announced to the client with this event.
    fn on_new_channel(&mut self, channel: Channel, parent_channel: Channel) {}
    fn on_new_channel_created(&mut self, channel: Channel,
        parent_channel: Channel, invoker: Connection) {}
    fn on_delete_channel(&mut self, channel: Channel, invoker: Connection) {}
    fn on_channel_move(&mut self, channel: Channel, new_parent_channel: Channel,
        invoker: Connection) {}
    fn on_client_kick_from_channel(&mut self, client: Connection,
        old_channel: Channel, new_channel: Channel, visibility: Visibility,
        invoker: Connection, kick_message: String) {}
    fn on_client_kick_from_server(&mut self, client: Connection,
        old_channel: Channel, new_channel: Channel, visibility: Visibility,
        invoker: Connection, kick_message: String) {}
    fn on_server_group_client_added(&mut self, client: Connection,
        group: ServerGroup, invoker: Connection) {}
    fn on_server_group_client_deleted(&mut self, client: Connection,
        group: ServerGroup, invoker: Connection) {}





    // The following callbacks are not implemented (and get the wrong arguments)
    // ************************** Optional functions ***************************

    /// Plugin might offer a configuration window. If the ConfigureOffer is
    /// `ConfigureOffer::No`, this function does not need to be implemented.
    //FIXME fn configure(&mut self, handle: *void, q_parent_widget: *void) {}
    fn configure(&mut self) {}

    /// If the plugin wants to use error return codes, plugin commands, hotkeys
    /// or menu items, it needs to register a command ID. This function will be
    /// automatically called after the plugin was initialized. This function is
    /// optional. If you don't use these features, this function can be omitted.
    /// Note the passed pluginID parameter is no longer valid after calling this
    /// function, so you must copy it and store it in the plugin.
    fn register_plugin_id(&mut self, id: &str) {}

    /// Plugin command keyword. Return `None` if not used.
    fn command_keyword(&self) -> Option<String> { None }

    /// Plugin processes console command. Return `true` if plugin handled the
    /// command, `false` if not handled.
    fn process_command(&mut self, connection_id: u64, command: &str) -> bool { false }

    /// Implement the following three functions when the plugin should display a
    /// line in the server/channel/client info.
    fn info_title(&self) -> String { "".to_string() }

    /// Dynamic content shown in the right column in the info frame.
    /// Check the parameter `type` if you want to implement this feature only
    /// for specific item types. Return `None` to have the client ignore the
    /// info data.
    fn info_data(&mut self, connection_id: u64, id: u64, item_type: ItemType) ->
        Option<String> { None }

    /// Initialize plugin menus.
    /// This function is called after `init` and `register_plugin_id`. A
    /// pluginID is required for plugin menus to work. `register_plugin_id` must
    /// be implemented to use menus. If plugin menus are not used by a plugin,
    /// return `None`.
    ///
    /// # Returns
    ///
    /// A list of `MenuItem`s and the identifier for the menu icon.
    fn init_menus(&mut self) -> Option<(Vec<MenuItem>, String)> { None }

    /// Initialize plugin hotkeys. Hotkeys require `register_plugin_id` to be
    /// implemented. This function is automatically called by the client after
    /// `init`.
    fn init_hotkeys(&mut self) -> Option<Vec<Hotkey>> { None }

    // ******************************* Callbacks *******************************
    fn on_update_channel(&mut self, connection_id: u64, channel_id: u64) {}
    fn on_client_ids(&mut self, connection_id: u64, unique_client_id: &str,
        client_id: u16, client_name: &str) {}
    fn on_client_ids_finished(&mut self, connection_id: u64) {}
    fn on_server_edited(&mut self, connection_id: u64, editor_id: u16,
        editor_name: &str, editor_unique_id: &str) {}
    fn on_server_updated(&mut self, connection_id: u64) {}
    /// Return `true` if the client should ignore that error because it was
    /// handled by this plugin.
    fn on_server_error(&mut self, connection_id: u64, error_message: &str,
        error: Error, return_code: &str, extra_message: &str) -> bool { false }
    fn on_server_stop(&mut self, connection_id: u64, shutdown_message: &str) {}
    fn on_connection_info(&mut self, connection_id: u64, client_id: u16) {}
    fn on_server_connection_info(&mut self, connection_id: u64) {}
    fn on_channel_subscribe(&mut self, connection_id: u64, channel_id: u64) {}
    fn on_channel_subscribe_finished(&mut self, connection_id: u64, channel_id:
        u64) {}
    fn on_channel_unsubscribe(&mut self, connection_id: u64, channel_id: u64) {}
    fn on_channel_unsubscribe_finished(&mut self, connection_id: u64,
        channel_id: u64) {}
    fn on_channel_description_update(&mut self, connection_id: u64, channel_id:
        u64) {}
    fn on_channel_password_changed(&mut self, connection_id: u64, channel_id:
        u64) {}
    fn on_playback_shutdown_complete(&mut self, connection_id: u64) {}
    fn on_sound_device_list_changed(&mut self, mode_id: &str, play_or_cap: bool)
        {}
    fn on_edit_playback_voice_data(&mut self, connection_id: u64, client_id:
        u16, samples: Vec<i16>, channels: i32) {}
    //TODO fn on_edit_post_process_voice_data(&mut self, connection_id: u64, client_id:
    //    u16, samples: Vec<i16>, channels: i32, channel_speakers: Vec<u32>,
    //    channel_fill_mask: *u32) {}
    //TODO more...fn on_edit_mixed_playback_voice_data(&mut self, connection_id: u64, ) {}
    // line 888-904
    fn on_user_logging_message(&mut self, log_message: &str, log_level:
        LogLevel, log_channel: &str, log_id: u64, log_time: &str,
        complete_log_string: &str) {}
    fn on_client_ban_from_server(&mut self, connection_id: u64, client_id: u16,
        old_channel_id: u64, new_channel_id: u64, visibility: Visibility,
        kicker_id: u16, kicker_name: &str, kicker_unique_id: &str, time: u64,
        kick_message: &str) {}
    /// Return `true` if the plugin handled this event and the client should
    /// ignore it.
    fn on_client_poke_event(&mut self, connection_id: u64, from_client_id: u16,
        poker_name: &str, poker_unique_id: &str, message: &str, ff_ignored:
        bool) -> bool { false }
    fn on_client_self_variable_update(&mut self, connection_id: u64, flag: i32,
        old_value: &str, new_value: &str) {}
    fn on_file_list(&mut self, connection_id: u64, path: &str, name: &str, size:
        u64, datetime: u64, typename: FileListType, incompletesize: u64,
        return_code: &str) {}
    fn on_file_list_finished(&mut self, connection_id: u64, channel_id: u64,
        path: &str) {}
    fn on_file_info(&mut self, connection_id: u64, channel_id: u64, name: &str,
        size: u64, datetime: u64) {}
    fn on_server_group_list(&mut self, connection_id: u64, server_group_id: u64,
        name: &str, typename: i32, icon_id: i32, save_db: bool) {}
    fn on_server_group_list_finished(&mut self, connection_id: u64) {}
    fn on_server_group_by_cliend_id(&mut self, connection_id: u64, name: &str,
        server_group_list: u64, client_database_id: u64) {}
    fn on_server_group_perm_list(&mut self, connection_id: u64, server_group_id:
        u64, permission_id: u32, permission_value: i32, permission_negated: i32,
        permission_skip: i32) {}
    fn on_server_group_perm_list_finished(&mut self, connection_id: u64,
        server_group_id: u64) {}
    fn on_server_group_client_list(&mut self, connection_id: u64,
        server_group_id: u64, client_database_id: u64, client_name_id: &str,
        client_unique_id: &str) {}
    fn on_channel_group_list(&mut self, connection_id: u64, channel_group_id:
        u64, name: &str, typename: i32, icon_id: i32, save_db: bool) {}
    fn on_channel_group_list_finished(&mut self, connection_id: u64) {}
    fn on_channel_group_perm_list(&mut self, connection_id: u64,
        channel_group_id: u64, permission_id: u32, permission_value: i32,
        permission_negated: i32, permission_skip: i32) {}
    fn on_channel_group_perm_list_finished(&mut self, connection_id: u64) {}
    fn on_channel_perm_list(&mut self, connection_id: u64, channel_id: u64,
        permission_id: u32, permission_value: i32, permission_negated: i32,
        permission_skip: i32) {}
    fn on_channel_perm_list_finished(&mut self, connection_id: u64, channel_id:
        i32) {}
    fn on_client_perm_list(&mut self, connection_id: u64, client_database_id:
        u64, permission_id: u32, permission_value: i32, permission_negated: i32,
        permission_skip: i32) {}
    fn on_client_perm_list_finished(&mut self, connection_id: u64,
        client_database_id: u64) {}
    fn on_channel_client_perm_list(&mut self, connection_id: u64, channel_id:
        u64, client_database_id: u64, permission_id: u32, permission_value: i32,
        permission_negated: i32, permission_skip: i32) {}
    fn on_channel_client_perm_list_finished(&mut self, connection_id: u64,
        channel_id: u64, client_database_id: u64) {}
    /// Return `true` if the plugin handled this event and the client should
    /// ignore it.
    fn on_server_permission_error(&mut self, connection_id: u64, error_message:
        &str, error: Error, return_code: &str, failed_permission_id: u32) ->
        bool { false }
    fn on_permission_list_group_end_id(&mut self, connection_id: u64,
        group_end_id: u32) {}
    fn on_permission_list(&mut self, connection_id: u64, permission_id: u32,
        permission_name: &str, permission_description: &str) {}
    fn on_permission_list_finished(&mut self, connection_id: u64) {}
    fn on_permission_overview(&mut self, connection_id: u64, client_database_id:
        u64, overview_type: i32, overview_id1: u64, overview_id2: u64,
        permission_id: u32, permission_value: i32, permission_negated: i32,
        permission_skip: i32) {}
    fn on_permission_overview_finished(&mut self, connection_id: u64, ) {}
    //TODO line 1053 fn on_(&mut self, connection_id: u64, ) {}
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Server
{
    id: u64
}

impl Server
{
    pub fn get_id(&self) -> u64
    {
        self.id
    }

    pub fn get_own_connection(&self) -> Result<Connection, Error>
    {
        let mut id: u16 = 0;
        let res: Error = unsafe { std::mem::transmute((ts3interface::ts3functions.as_ref()
            .expect("Functions should be loaded").get_client_id)
                (self.id, &mut id)) };
        match res
        {
            Error::Ok => Ok(Connection { id: id, server: self.clone() }),
            _         => Err(res)
        }
    }

    pub fn get_property_as_string(&self, property: VirtualServerProperties) -> Result<Box<String>, Error>
    {
        unsafe
        {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_server_variable_as_string)
                    (self.id, property as size_t, &mut name));
            match res
            {
                Error::Ok => Ok(Box::new(String::from_utf8_lossy(CStr::from_ptr(name).to_bytes()).into_owned())),
                _ => Err(res)
            }
        }
    }

    pub fn get_connections(&self) -> Result<Vec<Connection>, Error>
    {
        unsafe
        {
            let mut result: *mut u16 = std::ptr::null_mut();
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_client_list)
                    (self.id, &mut result));
            match res
            {
                Error::Ok =>
                {
                    let mut cs = Vec::new();
                    let mut counter = 0;
                    while *result.offset(counter) != 0
                    {
                        cs.push(Connection { server: self.clone(), id: *result.offset(counter) });
                        counter += 1;
                    }
                    Ok(cs)
                }
                _ => Err(res)
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Channel
{
    id: u64,
    server: Server
}

impl Channel
{
    pub fn get_id(&self) -> u64
    {
        self.id
    }

    pub fn get_server(&self) -> &Server
    {
        &self.server
    }

    pub fn get_property_as_string(&self, property: ChannelProperties) -> Result<Box<String>, Error>
    {
        unsafe
        {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_channel_variable_as_string)
                    (self.server.id, self.id, property as size_t, &mut name));
            match res
            {
                Error::Ok => Ok(Box::new(String::from_utf8_lossy(CStr::from_ptr(name).to_bytes()).into_owned())),
                _ => Err(res)
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Connection
{
    id: u16,
    server: Server
}

impl Connection
{
    pub fn get_id(&self) -> u16
    {
        self.id
    }

    pub fn get_server(&self) -> &Server
    {
        &self.server
    }

    pub fn get_channel(&self) -> Result<Channel, Error>
    {
        unsafe
        {
            let mut id: u64 = 0;
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_channel_of_client)
                    (self.server.id, self.id, &mut id));
            match res
            {
                Error::Ok => Ok(Channel { server: self.server.clone(), id: id }),
                _ => Err(res)
            }
        }
    }

    pub fn get_property_as_string(&self, property: ConnectionProperties) -> Result<Box<String>, Error>
    {
        unsafe
        {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_connection_variable_as_string)
                    (self.server.id, self.id, property as size_t, &mut name));
            match res
            {
                Error::Ok => Ok(Box::new(String::from_utf8_lossy(CStr::from_ptr(name).to_bytes()).into_owned())),
                _ => Err(res)
            }
        }
    }

    pub fn get_client_property_as_string(&self, property: ClientProperties) -> Result<Box<String>, Error>
    {
        unsafe
        {
            let mut name: *mut c_char = std::ptr::null_mut();
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_client_variable_as_string)
                    (self.server.id, self.id, property as size_t, &mut name));
            match res
            {
                Error::Ok => Ok(Box::new(String::from_utf8_lossy(CStr::from_ptr(name).to_bytes()).into_owned())),
                _ => Err(res)
            }
        }
    }

    pub fn get_client_property_as_int(&self, property: ClientProperties) -> Result<i32, Error>
    {
        unsafe
        {
            let mut result: c_int = 0;
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").get_client_variable_as_int)
                    (self.server.id, self.id, property as size_t, &mut result));
            match res
            {
                Error::Ok => Ok(result as i32),
                _ => Err(res)
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ServerGroup
{
    id: u64,
    server: Server
}

impl ServerGroup
{
    pub fn get_id(&self) -> u64
    {
        self.id
    }

    pub fn get_server(&self) -> &Server
    {
        &self.server
    }
}

pub struct TsApi;

impl TsApi
{
    pub unsafe fn get_raw_functions<'a>() -> &'a Ts3Functions
    {
        ts3interface::ts3functions.as_ref().expect("Functions should be loaded")
    }

    pub fn log_message(message: &str, channel: &str, severity: LogLevel) -> Result<(), Error>
    {
        unsafe
        {
            let res: Error = std::mem::transmute((ts3interface::ts3functions.as_ref()
                .expect("Functions should be loaded").log_message)
                    (to_cstring!(message).as_ptr(),
                    severity, to_cstring!(channel).as_ptr(), 0));
            match res
            {
                Error::Ok => Ok(()),
                _ => Err(res)
            }
        }
    }

    pub fn log_or_print(message: &str, channel: &str, severity: LogLevel)
    {
        if let Err(error) = TsApi::log_message(message, channel, severity)
        {
            println!("Error {0:?} while printing '{1}' to {2} ({3:?})", error,
                message, channel, severity);
        }
    }
}

/// A struct that is used for the internal representation of the plugin
#[repr(C)]
pub struct PluginData
{
    pub name:         &'static str,
    pub version:      &'static str,
    pub author:       &'static str,
    pub description:  &'static str,
    pub configurable: ConfigureOffer
}

/// Create a plugin. This macro has to be called once per library to create the
/// function interface that is used by TeamSpeak.
///
/// All strings that are provided to this macro have to be null-terminated.
///
/// # Arguments
///
///  - name         - The name of the plugin as displayed in TeamSpeak
///  - version      - The version of the plugin as displayed in TeamSpeak
///  - author       - The author of the plugin as displayed in TeamSpeak
///  - description  - The description of the plugin as displayed in TeamSpeak
///  - configurable - If the plugin offers the possibility to be configured.
///  - typename     - The type of the class that implements the plugin and has a
///                  `new()`-function
///
/// # Examples
///
/// Create an example plugin with a given name, version, author, description and
/// a struct `MyTsPlugin` that implements the `Plugin` trait:
///
/// ```ignore
/// create_plugin!("My Ts Plugin\0", "0.1.0\0", "My Name\0",
///     "A wonderful tiny example plugin\0", ConfigureOffer::No, MyTsPlugin);
/// ```
#[macro_export]
macro_rules! create_plugin
{
    ($name: expr, $version: expr, $author: expr, $description: expr,
        $configurable: expr, $typename: ident) =>
    {
        #[no_mangle]
        pub static PLUGIN_DATA: $crate::PluginData = $crate::PluginData
        {
            name: $name,
            version: $version,
            author: $author,
            configurable: $configurable,
            description: $description
        };

        #[no_mangle]
        pub fn create_instance() -> *mut $crate::Plugin
        {
            Box::into_raw(Box::new($typename::new()))
        }

        #[no_mangle]
        pub unsafe fn remove_instance(instance: *mut $crate::Plugin)
        {
            drop(Box::from_raw(instance));
        }
    };
}
