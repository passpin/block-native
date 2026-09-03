use crate::model::AssetKind;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
};
use thiserror::Error;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

const MANIFEST_PATH: &str = "manifest.json";
const PROGRAM_PATH: &str = "program.bcode";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedAsset {
    pub name: String,
    pub kind: AssetKind,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedAsset {
    pub kind: AssetKind,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LoadedPackage {
    pub program: Vec<u8>,
    pub assets: HashMap<String, LoadedAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    program: String,
    assets: Vec<ManifestAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestAsset {
    name: String,
    kind: AssetKind,
    path: String,
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("invalid package: {0}")]
    Invalid(String),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn build_package(program: &[u8], assets: &[PackedAsset]) -> Result<Vec<u8>, PackageError> {
    let mut names = HashSet::new();
    let mut manifest_assets = Vec::with_capacity(assets.len());
    for (index, asset) in assets.iter().enumerate() {
        validate_asset_name(&asset.name)?;
        if !names.insert(asset.name.clone()) {
            return Err(PackageError::Invalid(format!(
                "duplicate asset name '{}'",
                asset.name
            )));
        }
        manifest_assets.push(ManifestAsset {
            name: asset.name.clone(),
            kind: asset.kind.clone(),
            path: format!("assets/{index:04}.bin"),
        });
    }

    let manifest = Manifest {
        version: 1,
        program: PROGRAM_PATH.into(),
        assets: manifest_assets,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;

    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(MANIFEST_PATH, options)?;
    zip.write_all(&manifest_bytes)?;
    zip.start_file(PROGRAM_PATH, options)?;
    zip.write_all(program)?;
    for (asset, entry) in assets.iter().zip(manifest.assets.iter()) {
        zip.start_file(&entry.path, options)?;
        zip.write_all(&asset.bytes)?;
    }
    Ok(zip.finish()?.into_inner())
}

pub fn read_package(bytes: &[u8]) -> Result<LoadedPackage, PackageError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let manifest: Manifest = {
        let mut file = archive
            .by_name(MANIFEST_PATH)
            .map_err(|_| PackageError::Invalid("missing manifest.json".into()))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        serde_json::from_slice(&data)?
    };
    if manifest.version != 1 {
        return Err(PackageError::Invalid(format!(
            "unsupported package version {}",
            manifest.version
        )));
    }
    if manifest.program != PROGRAM_PATH {
        return Err(PackageError::Invalid(
            "manifest program path must be program.bcode".into(),
        ));
    }

    let program = {
        let mut file = archive
            .by_name(PROGRAM_PATH)
            .map_err(|_| PackageError::Invalid("missing program.bcode".into()))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        data
    };

    let mut assets = HashMap::new();
    let mut paths = HashSet::new();
    for entry in manifest.assets {
        validate_asset_name(&entry.name)?;
        if !is_safe_internal_path(&entry.path) || !entry.path.starts_with("assets/") {
            return Err(PackageError::Invalid(format!(
                "unsafe asset path '{}'",
                entry.path
            )));
        }
        if !paths.insert(entry.path.clone()) {
            return Err(PackageError::Invalid(format!(
                "duplicate asset path '{}'",
                entry.path
            )));
        }
        if assets.contains_key(&entry.name) {
            return Err(PackageError::Invalid(format!(
                "duplicate asset name '{}'",
                entry.name
            )));
        }
        let mut file = archive
            .by_name(&entry.path)
            .map_err(|_| PackageError::Invalid(format!("missing asset '{}'", entry.name)))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        assets.insert(
            entry.name,
            LoadedAsset {
                kind: entry.kind,
                bytes: data,
            },
        );
    }

    Ok(LoadedPackage { program, assets })
}

fn validate_asset_name(name: &str) -> Result<(), PackageError> {
    if name.trim().is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
        || name.contains("..")
        || name.chars().any(char::is_control)
    {
        return Err(PackageError::Invalid(format!("unsafe asset name '{name}'")));
    }
    Ok(())
}

fn is_safe_internal_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
