use crate::validator::Validator;
use crate::block::Block;
use crate::db::{Db, DbKind};
use crate::everything::Everything;
use crate::item::Item;
use crate::token::Token;

#[derive(Clone, Debug)]
pub struct Loyalty {}

impl Loyalty {
    pub fn add(db: &mut Db, key: Token, block: Block) {
        db.add(Item::Loyalty, key, block, Box::new(Self {}));
    }
}

impl DbKind for Loyalty {
    fn validate(&self, _key: &Token, block: &Block, data: &Everything) {
        let mut vd = Validator::new(block, data);

        vd.req_field("value");

        vd.field_numeric("value");
        vd.field_numeric("min");
        vd.field_numeric("max");
        vd.field_numeric("yearly_decay");
        vd.field_numeric("months");
    }
}