// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

//! Events are surfaced by the library to indicate some action must be taken
//! by the client.
//!
//! Because we don't have a built-in runtime, it's up to the client to poll
//! [`crate::LiquidityManager::get_and_clear_pending_events()`] to receive events.

use crate::jit_channel;

/// An Event which you should probably take some action in response to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
	/// A LSPS2 (JIT Channel) protocol event
	LSPS2(jit_channel::event::Event),
}
