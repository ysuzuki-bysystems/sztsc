use std::ffi::c_int;
use std::ffi::c_void;

use super::lib;

unsafe extern "C" {
    pub(super) fn subscribe_channel_connected(
        pubsub: *mut lib::wPubSub,
        h: unsafe extern "C" fn(*mut c_void, *mut lib::ChannelConnectedEventArgs) -> c_int,
    );

    pub(super) fn subscribe_channel_disconnected(
        pubsub: *mut lib::wPubSub,
        h: unsafe extern "C" fn(*mut c_void, *mut lib::ChannelDisconnectedEventArgs) -> c_int,
    );
}
