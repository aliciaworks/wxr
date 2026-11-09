#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrTextureType` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrTextureType {
    Texture = "texture",
    TextureArray = "texture-array",
}
