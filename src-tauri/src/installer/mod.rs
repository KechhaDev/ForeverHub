mod download;
mod metadata;
mod natives;
mod libraries;
mod assets;
mod minecraft;
mod fabric;
mod installation;

pub(crate) use installation::prepare_installation;

pub(crate) use fabric::install_fabric;

pub(crate) use minecraft::install_minecraft;

pub(crate) use assets::download_assets;

pub(crate) use download::{
    download_verified,
    maven_path,
    safe_relative_path,
};

pub(crate) use metadata::{
    fetch_json,
    find_minecraft_version,
    AssetIndex,
    AssetIndexInfo,
    MinecraftVersionDetails,
};

pub(crate) use natives::{
    extract_native_library,
    get_natives_directory,
    prepare_natives_directory,
};

pub(crate) use libraries::{
    download_fabric_library,
    download_fabric_natives,
    download_libraries,
};