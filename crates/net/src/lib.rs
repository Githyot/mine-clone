//! Multiplayer plumbing: the wire protocol, the QUIC transport, join codes, LAN discovery,
//! router port opening and the relay. The game decides what to send; this crate only carries it.

pub mod codes;
pub mod lan;
pub mod protocol;
pub mod relay;
pub mod transport;
pub mod upnp;

pub use protocol::*;
pub use transport::{ConnId, Guest, GuestEvent, Host, HostEvent};
