extern crate libc;
extern crate ts3plugin_sys;

pub mod ts3interface;

use ts3plugin_sys::clientlib_publicdefinitions::*;
use ts3plugin_sys::plugin_definitions::*;
use ts3plugin_sys::public_definitions::*;
use ts3plugin_sys::public_errors::Error;

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
/// #![feature(box_raw)]
/// #[macro_use]
/// extern crate ts3plugin;
///
/// use ts3plugin::*;
///
/// struct MyTsPlugin;
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
///     "A wonderful tiny example plugin\0", MyTsPlugin);
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


    // ************************** Optional functions ***************************
    /// Tell client if plugin offers a configuration window.
    /// If this function is not overwritten, ConfigureOffer::No is returned.
    fn offers_configure(&self) -> ConfigureOffer { ConfigureOffer::No }

    /// Plugin might offer a configuration window. If offers_configure returns
    /// ConfigureOffer::No, this function does not need to be implemented.
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

    /// Client changed current server connection handler.
    fn current_server_connection_changed(&mut self, s: u64) {}

    /// Implement the following three functions when the plugin should display a
    /// line in the server/channel/client info.
    fn info_title(&self) -> String { "".to_string() }

    /// Dynamic content shown in the right column in the info frame.
    /// Check the parameter `type` if you want to implement this feature only
    /// for specific item types. Return `None` to have the client ignore the
    /// info data.
    fn info_data(&mut self, connection_id: u64, id: u64, item_type: ItemType) ->
        Option<String> { None }

    /// Plugin requests to be always automatically loaded by the TeamSpeak 3
    /// client unless the user manually disabled it in the plugin dialog.
    /// The default value is `false` which means the plugin won't be loaded
    /// automatically.
    fn request_autoload(&self) -> bool { false }

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
    fn on_connect_status_change(&mut self, connection_id: u64, new_status:
        ConnectStatus, error_number: Error) {}
    fn on_new_channel(&mut self, connection_id: u64, channel_id: u64,
        channel_parent_id: u64) {}
    fn on_new_channel_created(&mut self, connection_id: u64, channel_id: u64,
        channel_parent_id: u64, invoker_id: u16, invoker_name: &str,
        invoker_unique_id: &str) {}
    fn on_delete_channel(&mut self, connection_id: u64, channel_id: u64,
        invoker_id: u16, invoker_name: &str, invoker_unique_id: &str) {}
    fn on_channel_move(&mut self, connection_id: u64, channel_id: u64,
        new_channel_parent_id: u64, invoker_id: u16, invoker_name: &str,
        invoker_unique_id: &str) {}
    fn on_update_channel(&mut self, connection_id: u64, channel_id: u64) {}
    fn on_update_channel_edited(&mut self, connection_id: u64, channel_id: u64,
        invoker_id: u16, invoker_name: &str, invoker_unique_id: &str) {}
    fn on_update_client(&mut self, connection_id: u64, invoker_id: u16,
        invoker_name: &str, invoker_unique_id: &str) {}
    fn on_client_move(&mut self, connection_id: u64, client_id: u16,
        old_channel_id: u64, new_channel_id: u64, visibility: Visibility,
        move_message: &str) {}
    fn on_client_move_subscription(&mut self, connection_id: u64, client_id:
        u16, old_channel_id: u64, new_channel_id: u64, visibility: Visibility)
        {}
    fn on_client_move_timeout(&mut self, connection_id: u64, client_id: u16,
        old_channel_id: u64, new_channel_id: u64, visibility: Visibility,
        timeout_message: &str) {}
    fn on_client_move_moved(&mut self, connection_id: u64, client_id: u16,
        old_channel_id: u64, new_channel_id: u64, visibility: Visibility,
        mover_id: u16, mover_name: &str, mover_unique_id: &str, move_message:
        &str) {}
    fn on_client_kick_from_channel(&mut self, connection_id: u64, client_id:
        u16, old_channel_id: u64, new_channel_id: u64, visibility: Visibility,
        kicker_id: u16, kicker_name: &str, kicker_unique_id: &str, kick_message:
        &str) {}
    fn on_client_kick_from_server(&mut self, connection_id: u64, client_id: u16,
        old_channel_id: u64, new_channel_id: u64, visibility: Visibility,
        kicker_name: &str, kicker_unique_id: &str, kick_message: &str) {}
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
    /// `ff_ignored` is `true` if the friend/foe manager ignored that message.
    fn on_text_message(&mut self, connection_id: u64, target_mode: u16, to_id:
        u16, from_id: u16, from_name: &str, from_unique_id: &str, message: &str,
        ff_ignored: bool) {}
    fn on_talk_status_change(&mut self, connection_id: u64, status: TalkStatus,
        is_receiving_whisper: bool, client_id: u16) {}
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

#[repr(C)]
pub struct PluginData
{
    pub name:        &'static str,
    pub version:     &'static str,
    pub author:      &'static str,
    pub description: &'static str
}

/// Create a plugin. This macro has to be called once per library to create the
/// function interface that is used by TeamSpeak.
///
/// All strings that are provided to this macro have to be null-terminated.
///
/// # Arguments
///
///  - name        - The name of the plugin as displayed in TeamSpeak
///  - version     - The version of the plugin as displayed in TeamSpeak
///  - author      - The author of the plugin as displayed in TeamSpeak
///  - description - The description of the plugin as displayed in TeamSpeak
///  - typename    - The type of the class that implements the plugin
///
/// # Examples
///
/// Create an example plugin with a given name, version, author, description and
/// a struct `MyTsPlugin` that implements the `Plugin` trait:
///
/// ```ignore
/// create_plugin!("My Ts Plugin\0", "0.1.0\0", "My Name\0",
///     "A wonderful tiny example plugin\0", MyTsPlugin);
/// ```
#[macro_export]
macro_rules! create_plugin
{
    ($name: expr, $version: expr, $author: expr, $description: expr, $typename: expr) =>
    {
        #[no_mangle]
        pub static PLUGIN_DATA: $crate::PluginData = $crate::PluginData
        {
            name: $name,
            version: $version,
            author: $author,
            description: $description
        };

        #[no_mangle]
        pub fn create_instance() -> *mut $crate::Plugin
        {
            Box::into_raw(Box::new($typename))
        }

        #[no_mangle]
        pub unsafe fn remove_instance(instance: *mut $crate::Plugin)
        {
            drop(Box::from_raw(instance));
        }
    };
}
