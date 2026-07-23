#![cfg_attr(not(feature = "std"), no_std)]

//! The faithful device runtime.
//!
//! Everything here runs identically in firmware and in the simulator. A
//! [`device::Device`] owns one box's clock, transport, groove, and LCD model;
//! it renders mono audio and a framebuffer from shared mesh state. Hardware
//! glue (I2S DMA, ESP-NOW, the SSD1306 driver) lives in the firmware crate;
//! the host simulator provides the same surfaces with a drifting virtual clock
//! and host audio. Neither re-implements the musical behavior.

pub mod device;
pub mod display;
pub mod font;

pub use device::{ArpDirection, Device, DeviceVoice, Engine};
pub use display::{DisplayState, Lcd};
