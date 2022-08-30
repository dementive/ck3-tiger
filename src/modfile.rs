use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::block::Block;
use crate::errorkey::ErrorKey;
use crate::errors::warn;
use crate::fileset::FileKind;
use crate::pdxfile::PdxFile;
use crate::token::Token;

#[derive(Clone, Debug)]
#[allow(dead_code)] // remove when TODO are fixed
pub struct ModFile {
    block: Block,
    name: Option<Token>,
    path: Option<Token>,
    // TODO: implement this in Fileset
    replace_path: Vec<Token>,
    version: Option<Token>,
    // TODO: check that these are tags accepted by steam ?
    tags: Option<Vec<Token>>,
    // TODO: check if the version is compatible with the validator.
    // (Newer means the validator is too old, older means it's not up to date
    // with current CK3)
    supported_version: Option<Token>,
    picture: Option<Token>,
}

fn validate_modfile(block: &Block) -> ModFile {
    let modfile = ModFile {
        block: block.clone(),
        name: block.get_field_value("name").cloned(),
        path: block.get_field_value("path").cloned(),
        replace_path: block.get_field_values("replace_path"),
        version: block.get_field_value("version").cloned(),
        tags: block.get_field_list("tags"),
        supported_version: block.get_field_value("supported_version").cloned(),
        picture: block.get_field_value("picture").cloned(),
    };

    if let Some(picture) = &modfile.picture {
        if !picture.is("thumbnail.png") {
            warn(
                picture,
                ErrorKey::Packaging,
                "Steam ignores picture= and always uses thumbnail.png.",
            );
        }
    }

    // TODO: check if supported_version is newer than validator,
    // or is older than known CK3

    modfile
}

impl ModFile {
    pub fn read(pathname: &Path) -> Result<Self> {
        let block = PdxFile::read_no_bom(pathname, FileKind::ModFile, pathname)
            .with_context(|| format!("Could not read .mod file {}", pathname.display()))?;
        Ok(validate_modfile(&block))
    }

    pub fn modpath(&self) -> PathBuf {
        let mut dirpath = self
            .block
            .loc
            .pathname
            .parent()
            .unwrap_or_else(|| Path::new("."));
        if dirpath.components().count() == 0 {
            dirpath = Path::new(".");
        }

        let modpath = if let Some(path) = &self.path {
            dirpath.join(path.as_str())
        } else {
            dirpath.to_path_buf()
        };

        if modpath.exists() {
            modpath
        } else {
            dirpath.to_path_buf()
        }
    }
}
