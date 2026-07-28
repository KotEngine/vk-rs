//! File uploaders

pub mod base;
pub mod photo;
pub mod doc;
pub mod video;
pub mod audio;
pub mod voice;
pub mod speech;

pub use base::*;
pub use photo::*;
pub use doc::*;
pub use video::*;
pub use audio::*;
pub use voice::*;
pub use speech::*;
pub use voice::GraffitiUploader;
