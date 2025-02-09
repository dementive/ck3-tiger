use crate::block::{Block, BV};
use crate::context::ScopeContext;
use crate::db::{Db, DbKind};
use crate::everything::Everything;
use crate::game::GameFlags;
use crate::item::{Item, ItemLoader};
use crate::scopes::Scopes;
use crate::token::Token;
use crate::tooltipped::Tooltipped;
use crate::trigger::validate_trigger;
use crate::validator::{Validator, ValueValidator};

#[derive(Clone, Debug)]
pub struct ScriptedIllustration {}

inventory::submit! {
    ItemLoader::Normal(GameFlags::Ck3, Item::ScriptedIllustration, ScriptedIllustration::add)
}

impl ScriptedIllustration {
    pub fn add(db: &mut Db, key: Token, block: Block) {
        db.add(Item::ScriptedIllustration, key, block, Box::new(Self {}));
    }
}

impl DbKind for ScriptedIllustration {
    fn validate(&self, key: &Token, block: &Block, data: &Everything) {
        let mut vd = Validator::new(block, data);
        // TODO: validate the call from gui
        let mut sc = ScopeContext::new(Scopes::all(), key);

        vd.multi_field_validated("texture", |bv, data| match bv {
            BV::Value(token) => validate_texture(key, ValueValidator::new(token, data)),
            BV::Block(block) => {
                let mut vd = Validator::new(block, data);
                vd.field_validated_value("reference", validate_texture);
                vd.field_validated_block("trigger", |block, data| {
                    validate_trigger(block, data, &mut sc, Tooltipped::No);
                });
            }
        });
        vd.multi_field_validated_block("environment", |block, data| {
            let mut vd = Validator::new(block, data);
            if let Some(token) = vd.field_value("reference") {
                data.verify_exists(Item::PortraitEnvironment, token);
            }
            vd.field_validated_block("trigger", |block, data| {
                validate_trigger(block, data, &mut sc, Tooltipped::No);
            });
        });
    }
}

fn validate_texture(_key: &Token, mut vd: ValueValidator) {
    vd.dir_file("gfx/interface/illustrations");
}
