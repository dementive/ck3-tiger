use fnv::{FnvHashMap, FnvHashSet};
use std::path::{Path, PathBuf};

use crate::block::validator::Validator;
use crate::block::Block;
use crate::context::ScopeContext;
use crate::desc::validate_desc;
use crate::everything::Everything;
use crate::fileset::{FileEntry, FileHandler};
use crate::helpers::dup_error;
use crate::item::Item;
use crate::modif::{validate_modifs, ModifKinds};
use crate::pdxfile::PdxFile;
use crate::scopes::Scopes;
use crate::token::Token;
use crate::trigger::validate_normal_trigger;
use crate::validate::validate_traits;

#[derive(Clone, Debug, Default)]
pub struct Doctrines {
    groups: FnvHashMap<String, DoctrineGroup>,
    doctrines: FnvHashMap<String, Doctrine>,
    parameters: FnvHashSet<String>, // only the boolean parameters
}

impl Doctrines {
    fn load_item(&mut self, key: &Token, block: &Block) {
        if let Some(other) = self.groups.get(key.as_str()) {
            if other.key.loc.kind >= key.loc.kind {
                dup_error(key, &other.key, "doctrine group");
            }
        }
        self.groups.insert(
            key.to_string(),
            DoctrineGroup::new(key.clone(), block.clone()),
        );

        for (doctrine, block) in block.iter_pure_definitions() {
            // skip definitions that are not doctrines
            if doctrine.is("is_available_on_create") || doctrine.is("name") {
                continue;
            }

            if let Some(other) = self.doctrines.get(doctrine.as_str()) {
                if other.key.loc.kind >= doctrine.loc.kind {
                    dup_error(doctrine, &other.key, "doctrine");
                }
            }

            if let Some(b) = block.get_field_block("parameters") {
                for (k, v) in b.get_assignments() {
                    if v.is("yes") || v.is("no") {
                        self.parameters.insert(k.to_string());
                    }
                }
            }
            self.doctrines.insert(
                doctrine.to_string(),
                Doctrine::new(doctrine.clone(), block.clone(), key.clone()),
            );
        }
    }

    pub fn validate(&self, data: &Everything) {
        for group in self.groups.values() {
            group.validate(data);
        }
        for doctrine in self.doctrines.values() {
            doctrine.validate(data);
        }
    }

    pub fn exists(&self, key: &str) -> bool {
        self.doctrines.contains_key(key)
    }

    pub fn parameter_exists(&self, key: &str) -> bool {
        self.parameters.contains(key)
    }
}

impl FileHandler for Doctrines {
    fn subpath(&self) -> PathBuf {
        PathBuf::from("common/religion/doctrines")
    }

    fn handle_file(&mut self, entry: &FileEntry, fullpath: &Path) {
        if !entry.filename().to_string_lossy().ends_with(".txt") {
            return;
        }

        let Some(block) = PdxFile::read(entry, fullpath) else { return };
        for (key, b) in block.iter_pure_definitions_warn() {
            self.load_item(key, b);
        }
    }
}

#[derive(Clone, Debug)]
pub struct DoctrineGroup {
    key: Token,
    block: Block,
}

impl DoctrineGroup {
    pub fn new(key: Token, block: Block) -> Self {
        Self { key, block }
    }

    pub fn needs_icon(&self, data: &Everything) -> bool {
        if let Some(group) = self.block.get_field_value("group") {
            if group.is("special") || group.is("not_creatable") {
                return false;
            }
        }

        if let Some(icon_path) =
            data.get_defined_string_warn(&self.key, "NGameIcons::FAITH_DOCTRINE_GROUP_ICON_PATH")
        {
            let path = format!("{}/{}.dds", icon_path, &self.key);
            return !data.fileset.exists(&path);
        }
        true
    }

    pub fn validate(&self, data: &Everything) {
        let mut vd = Validator::new(&self.block, data);
        let mut sc = ScopeContext::new_root(Scopes::Faith, self.key.clone());

        if let Some(bv) = vd.field("name") {
            validate_desc(bv, data, &mut sc);
        } else {
            let loca = format!("{}_name", self.key);
            data.localization.verify_exists_implied(&loca, &self.key);
        }

        // doc says "grouping" but that's wrong
        vd.field_value("group");

        vd.field_integer("number_of_picks");

        vd.field_validated_block("is_available_on_create", |b, data| {
            validate_normal_trigger(b, data, &mut sc, false);
        });

        // Any remaining definitions are doctrines, so accept them all.
        // They are validated separately.
        for (_, bv) in vd.unknown_keys() {
            bv.expect_block();
        }
    }
}

#[derive(Clone, Debug)]
pub struct Doctrine {
    key: Token,
    block: Block,
    group: Token,
}

impl Doctrine {
    pub fn new(key: Token, block: Block, group: Token) -> Self {
        Self { key, block, group }
    }

    pub fn validate(&self, data: &Everything) {
        let mut vd = Validator::new(&self.block, data);
        let mut sc = ScopeContext::new_root(Scopes::Faith, self.key.clone());

        let icon_path =
            data.get_defined_string_warn(&self.key, "NGameIcons::FAITH_DOCTRINE_ICON_PATH");
        if let Some(icon_path) = icon_path {
            if let Some(icon) = vd.field_value("icon") {
                let path = format!("{}/{}.dds", icon_path, icon);
                data.fileset.verify_exists_implied(&path, &icon);
            } else if data.doctrines.groups[self.group.as_str()].needs_icon(data) {
                let path = format!("{}/{}.dds", icon_path, &self.key);
                data.fileset.verify_exists_implied(&path, &self.key);
            }
        }

        if let Some(bv) = vd.field("name") {
            validate_desc(bv, data, &mut sc);
        } else {
            let loca = format!("{}_name", self.key);
            data.localization.verify_exists_implied(&loca, &self.key);
        }

        if let Some(bv) = vd.field("desc") {
            validate_desc(bv, data, &mut sc);
        } else {
            let loca = format!("{}_desc", self.key);
            data.localization.verify_exists_implied(&loca, &self.key);
        }

        vd.field_bool("visible");
        vd.field_validated_block("parameters", validate_parameters);
        vd.field_script_value("piety_cost", &mut sc);
        vd.field_validated_block("is_shown", |b, data| {
            validate_normal_trigger(b, data, &mut sc, false);
        });
        vd.field_validated_block("can_pick", |b, data| {
            validate_normal_trigger(b, data, &mut sc, true);
        });

        if let Some(block) = vd.field_block("character_modifier") {
            let mut vd = Validator::new(block, data);
            let mut sc = ScopeContext::new_root(
                Scopes::Character,
                self.block.get_key("character_modifier").unwrap().clone(),
            );
            if let Some(name) = vd.field_value("name") {
                data.localization.verify_exists(name);
            }
            validate_modifs(block, data, ModifKinds::Character, &mut sc, vd);
        }

        // Not documented, but used in vanilla
        if let Some(block) = vd.field_block("clergy_modifier") {
            let vd = Validator::new(block, data);
            let mut sc = ScopeContext::new_root(
                Scopes::Character,
                self.block.get_key("clergy_modifier").unwrap().clone(),
            );
            validate_modifs(block, data, ModifKinds::Character, &mut sc, vd);
        }

        // In the docs but not used in vanilla
        if let Some(block) = vd.field_block("doctrine_character_modifier") {
            let mut vd = Validator::new(block, data);
            let mut sc = ScopeContext::new_root(
                Scopes::Character,
                self.block
                    .get_key("doctrine_character_modifier")
                    .unwrap()
                    .clone(),
            );
            vd.field_value("doctrine"); // TODO: check that doctrine exists
            if let Some(doctrine) = vd.field_value("doctrine") {
                data.verify_exists(Item::Doctrine, doctrine);
            }
            validate_modifs(block, data, ModifKinds::Character, &mut sc, vd);
        }

        vd.field_validated_block("traits", validate_traits);
    }
}

fn validate_parameters(block: &Block, data: &Everything) {
    let mut vd = Validator::new(block, data);

    for (key, _, bv) in block.iter_items() {
        if let Some(param) = key {
            if let Some(v) = bv.expect_value() {
                if v.is("yes") || v.is("no") {
                    continue;
                }
                vd.field_numeric(param.as_str());
            }
        }
    }
    // We've handled all the keys, so register that in the validator
    _ = vd.unknown_keys();
}
