// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.
#![crate_name = "lightning_liquidity"]

//! # `lightning-liquidity`
//! Types and primitives to integrate a spec-compliant LSP with an LDK-based node.
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![allow(bare_trait_objects)]
#![allow(ellipsis_inclusive_range_patterns)]
#![allow(clippy::drop_non_drop)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod events;
mod lsps0;
#[cfg(lsps1)]
mod lsps1;
pub mod lsps2;
mod utils;

pub use lsps0::message_handler::{JITChannelsConfig, LiquidityManager, LiquidityProviderConfig};
pub use lsps0::msgs::LSPS_MESSAGE_TYPE_ID;
