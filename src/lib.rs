extern crate libc;
extern crate ts3plugin_sys;

pub mod ts3interface;

use ts3plugin_sys::plugin_definitions::*;

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

pub trait Plugin
{
    // Required functions
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

    // Optional functions
    /// Tell client if plugin offers a configuration window.
    /// If this function is not overwritten, ConfigureOffer::No is returned.
    fn offers_configure(&self) -> ConfigureOffer { ConfigureOffer::No }
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
#[macro_export]
macro_rules! create_plugin
{
    ($name: expr, $version: expr, $author: expr, $description: expr, $typename: ty) =>
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
            Box::into_raw(Box::new(TTSPlugin))
        }

        #[no_mangle]
        pub unsafe fn remove_instance(instance: *mut $crate::Plugin)
        {
            drop(Box::from_raw(instance));
        }
    };
}
