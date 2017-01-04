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

/// This trait that has to be implemented by a plugin. To enhance a library to a
/// working TeamSpeak plugin you have to call the macro [`create_plugin!`]
/// afterwards.
///
/// [`create_plugin!`]: ../macro.create_plugin.html
#[allow(unused_variables, unknown_lints, too_many_arguments)]
pub trait Plugin: 'static + Send {
	// *************************** Required methods ****************************
	/// Called when the plugin is loaded by TeamSpeak.
	fn new(api: &mut ::TsApi) -> Result<Box<Self>, InitError> where Self: Sized;

	// *************************** Optional methods ****************************
	/// Called after the plugin was created when the plugin id is received.
	/// Use [`api.get_plugin_id()`] to get the plugin id.
	///
	/// [`api.get_plugin_id()`]: ../struct.TsApi.html#method.get_plugin_id
	fn plugin_id_available(&mut self, api: &mut ::TsApi) {}

	/// If the connection status changes.
	/// If `status = `[`ConnectStatus::Connecting`], the connection_id is not yet
	/// registered in the [`TsApi`].
	///
	/// [`ConnectStatus::Connecting`]: ../../ts3plugin_sys/clientlib_publicdefinitions/enum.ConnectStatus.html
	/// [`TsApi`]: ../struct.TsApi.html
	fn connect_status_change(&mut self, api: &mut ::TsApi, server_id: ::ServerId, status:
		::ConnectStatus, error: ::Error) {}

	/// Called if a server is stopped. The server sends also a stop message.
	fn server_stop(&mut self, api: &mut ::TsApi, server_id: ::ServerId, message: String) {}

	/// Called if a server error occurs.
	/// Return `false` if the TeamSpeak client should handle the error normally or
	/// `true` if the client should ignore the error.
	fn server_error(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		error: ::Error, message: String, return_code: String,
		extra_message: String) -> bool { false }

	/// Called if someone edited the server.
	fn server_edited(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		invoker: Option<::Invoker>) {}

	/// Called when the user requests the server info by middle-clicking on the server.
	fn server_connection_info(&mut self, api: &mut ::TsApi, server_id: ::ServerId) {}

	fn connection_info(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId) {}

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

	/// Called if a connection switched the channel.
	fn connection_move(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, old_channel_id: ::ChannelId,
		new_channel_id: ::ChannelId, visibility: ::Visibility) {}

	/// Called if a connection was moved by another connection.
	fn connection_moved(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, old_channel_id: ::ChannelId,
		new_channel_id: ::ChannelId, visibility: ::Visibility, invoker: ::Invoker) {}

	/// Called when a connection times out.
	fn connection_timeout(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId) {}

	/// Called if a channel is announced to the client.
	/// This will be called for each channel when connecting to a server.
	fn channel_announced(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId) {}

	/// Called if the channel description was changed.
	fn channel_description_updated(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId) {}

	/// Called if the channel data are updated and available.
	/// This happens e.g. when the user clicked on the channel for the first time.
	fn channel_updated(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId, old_channel: Option<::Channel>) {}

	/// Called if a channel was created.
	/// The invoker is `None` if the server created the channel.
	fn channel_created(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId, invoker: Option<::Invoker>) {}

	/// Called if a channel was deleted.
	/// The invoker is `None` if the server deleted the channel.
	fn channel_deleted(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId, invoker: Option<::Invoker>) {}

	/// Called if a channel was edited.
	fn channel_edited(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId, old_channel: Option<::Channel>, invoker: ::Invoker) {}

	/// Called if the channel password was updated.
	fn channel_password_updated(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId) {}

	/// The current parent id of the channel is the old one, the new
	/// parent id is given as a parameter.
	fn channel_moved(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		channel_id: ::ChannelId, new_parent_channel_id: ::ChannelId,
		invoker: Option<::Invoker>) {}

	/// A message was received. `ignored` describes, if the friend and fool system
	/// of TeamSpeak ignored the message.
	/// Return `false` if the TeamSpeak client should handle the message normally or
	/// `true` if the client should ignore the message.
	fn message(&mut self, api: &mut ::TsApi, server_id: ::ServerId, invoker: ::Invoker,
		target: ::MessageReceiver, message: String, ignored: bool) -> bool { false }

	/// A user poked us. `ignored` describes, if the friend and fool system
	/// of TeamSpeak ignored the message.
	/// Return `false` if the TeamSpeak client should handle the poke normally or
	/// `true` if the client should ignore the poke.
	fn poke(&mut self, api: &mut ::TsApi, server_id: ::ServerId, invoker: ::Invoker,
		message: String, ignored: bool) -> bool { false }

	fn channel_kick(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, old_channel_id: ::ChannelId, new_channel_id: ::ChannelId,
		visibility: ::Visibility, invoker: ::Invoker, message: String) {}

	fn server_kick(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, invoker: ::Invoker, message: String) {}

	fn server_ban(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, invoker: ::Invoker, message: String, time: u64) {}

	/// The old values of `talking` and `whispering` are available from the connection.
	/// They will be updated after this functions returned.
	fn talking_changed(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, talking: ::TalkStatus, whispering: bool) {}

	/// Called if the avatar of a client is updated.
	/// This also happens when the avatar is discovered for the first time.
	/// The avatar information are only fetched if requested, e.g. if the
	/// user clicks on a connection.
	fn avatar_changed(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, path: Option<String>) {}

	/// Called if a channel group is assigned to a connection.
	fn connection_channel_group_changed(&mut self, api: &mut ::TsApi,
		server_id: ::ServerId, connection_id: ::ConnectionId, channel_group_id: ::ChannelGroupId,
		channel_id: ::ChannelId, invoker: ::Invoker) {}

	/// Called if a server group is added to a connection.
	fn connection_server_group_added(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection: ::Invoker, server_group_id: ::ServerGroupId, invoker: ::Invoker) {}

	/// Called if a server group is removed from a connection.
	fn connection_server_group_removed(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection: ::Invoker, server_group_id: ::ServerGroupId, invoker: ::Invoker) {}

	/// Called when a voice packet from a client was received.
	///
	/// From the TeamSpeak documentation:
	/// The following event is called when a voice packet from a client (not own
	/// client) is decoded and about to be played over your sound device, but
	/// before it is 3D positioned and mixed with other sounds. You can use this
	/// function to alter the voice data (for example when you want to do
	/// effects on it) or to simply get voice data. The TeamSpeak client uses
	/// this function to record sessions.
	///
	/// The voice data is available as 16 bit with 48 KHz. The channels are packed
	/// (interleaved).
	/// The callbacks with audio data are called from another thread than the
	/// other functions.
	fn playback_voice_data(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, samples: &mut [i16], channels: i32) {}

	/// Called when a voice packet from a client was positioned.
	///
	/// From the TeamSpeak documentation:
	/// The following event is called when a voice packet from a client (not own
	/// client) is decoded and 3D positioned and about to be played over your
	/// sound device, but before it is mixed with other sounds. You can use this
	/// function to alter/get the voice data after 3D positioning.
	///
	/// The voice data is available as 16 bit with 48 KHz. The channels are packed
	/// (interleaved).
	/// The callbacks with audio data are called from another thread than the
	/// other functions.
	fn post_process_voice_data(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		connection_id: ::ConnectionId, samples: &mut [i16], channels: i32,
		channel_speaker_array: &[::Speaker], channel_fill_mask: &mut u32) {}

	/// Called when all voice data were mixed.
	///
	/// From the TeamSpeak documentation:
	/// The following event is called when all sounds that are about to be
	/// played back for this server connection are mixed. This is the last
	/// chance to alter/get sound.
	///
	/// The voice data is available as 16 bit with 48 KHz. The channels are packed
	/// (interleaved).
	/// The callbacks with audio data are called from another thread than the
	/// other functions.
	fn mixed_playback_voice_data(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		samples: &mut [i16], channels: i32, channel_speaker_array: &[::Speaker],
		channel_fill_mask: &mut u32) {}

	/// The recorded sound from the current capture device.
	/// `send` is set if the audio data will be send to the server. This attribute
	/// can be changed in this callback.
	/// The return value of this function describes if the sound data was altered.
	/// Return `true` if the sound was changed and `false` otherwise.
	/// The callbacks with audio data are called from another thread than the
	/// other functions.
	fn captured_voice_data(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		samples: &mut [i16], channels: i32, send: &mut bool) -> bool { false }

	/// Return `false` if the TeamSpeak client should handle the error normally or
	/// `true` if the client should ignore the error.
	fn permission_error(&mut self, api: &mut ::TsApi, server_id: ::ServerId,
		permission_id: ::PermissionId, error: ::Error, message: String,
		return_code: String) -> bool { false }

	/// Called if the plugin is getting disabled (either by the user or if
	/// TeamSpeak is exiting).
	fn shutdown(&mut self, api: &mut ::TsApi) {}
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
///  - typename     - The type of the class that implements the [`Plugin`] trait
///
/// # Examples
///
/// Create an example plugin with a given name, version, author, description and
/// a struct `MyTsPlugin` that implements the [`Plugin`] trait:
///
/// ```ignore
/// create_plugin!("My Ts Plugin", "0.1.0", "My Name",
///     "A wonderful tiny example plugin", ConfigureOffer::No, false, MyTsPlugin);
/// ```
///
/// [`Plugin`]: plugin/trait.Plugin.html
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
		pub unsafe extern "C" fn ts3plugin_init() -> std::os::raw::c_int {
			match $crate::ts3interface::private_init::<$typename>() {
				Ok(_) => 0,
				Err($crate::InitError::Failure) => 1,
				Err($crate::InitError::FailureNoMessage) => -2,
			}
		}

		/// Unique name identifying this plugin.
		/// The result of this function has to be a null-terminated static string.
		/// Can be called before init.
		#[no_mangle]
		#[doc(hidden)]
		pub extern "C" fn ts3plugin_name() -> *const std::os::raw::c_char {
			(*PLUGIN_NAME).as_ptr()
		}

		/// The version of the plugin.
		/// Can be called before init.
		#[no_mangle]
		#[doc(hidden)]
		pub extern "C" fn ts3plugin_version() -> *const std::os::raw::c_char {
			(*PLUGIN_VERSION).as_ptr()
		}

		/// The author of the plugin.
		/// Can be called before init.
		#[no_mangle]
		#[doc(hidden)]
		pub extern "C" fn ts3plugin_author() -> *const std::os::raw::c_char {
			(*PLUGIN_AUTHOR).as_ptr()
		}

		/// The desription of the plugin.
		/// Can be called before init.
		#[no_mangle]
		#[doc(hidden)]
		pub extern "C" fn ts3plugin_description() -> *const std::os::raw::c_char {
			(*PLUGIN_DESCRIPTION).as_ptr()
		}

		/// If the plugin offers the possibility to be configured by the user.
		/// Can be called before init.
		#[allow(non_snake_case)]
		#[no_mangle]
		#[doc(hidden)]
		pub extern "C" fn ts3plugin_offersConfigure() -> std::os::raw::c_int {
			$configurable as std::os::raw::c_int
		}

		/// If the plugin should be loaded automatically.
		/// Can be called before init.
		#[allow(non_snake_case)]
		#[no_mangle]
		#[doc(hidden)]
		pub extern "C" fn ts3plugin_requestAutoload() -> std::os::raw::c_int {
			if $autoload {
				1
			} else {
				0
			}
		}
	};
}
