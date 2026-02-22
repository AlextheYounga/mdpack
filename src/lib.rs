mod pack;
mod unpack;

pub use pack::{PackOptions, pack_to_path, pack_to_string};
pub use unpack::{UnpackOptions, unpack_from_path, unpack_from_str};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
