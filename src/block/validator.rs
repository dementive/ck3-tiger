use crate::block::{Block, BlockOrValue, Comparator, Date, Token};
use crate::errorkey::ErrorKey;
use crate::errors::{advice, error, warn};
use crate::everything::Everything;

#[derive(Debug)]
pub struct Validator<'a> {
    // The block being validated
    block: &'a Block,
    // A link to all the loaded and processed CK3 and mod files
    data: &'a Everything,
    // Fields that have been requested so far
    known_fields: Vec<&'a str>,
    // Whether loose tokens are expected
    accepted_tokens: bool,
    // Whether subblocks are expected
    accepted_blocks: bool,
}

impl<'a> Validator<'a> {
    pub fn new(block: &'a Block, data: &'a Everything) -> Self {
        Validator {
            block,
            data,
            known_fields: Vec::new(),
            accepted_tokens: false,
            accepted_blocks: false,
        }
    }

    pub fn req_field(&mut self, name: &'a str) -> bool {
        let found = self.field(name).is_some();
        if !found {
            error(
                &self.block.loc,
                ErrorKey::Validation,
                &format!("required field `{}` missing", name),
            );
        }
        found
    }

    pub fn field_check<F>(&mut self, name: &'a str, mut f: F) -> bool
    where
        F: FnMut(&BlockOrValue),
    {
        self.known_fields.push(name);

        let mut found = false;
        for (k, cmp, v) in &self.block.v {
            if let Some(key) = k {
                if key.is(name) {
                    if found {
                        warn(
                            key,
                            ErrorKey::Duplicate,
                            &format!("multiple definitions of `{}`, expected only one.", key),
                        );
                    }
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    f(v);
                    found = true;
                }
            }
        }
        found
    }

    pub fn field(&mut self, name: &'a str) -> Option<&BlockOrValue> {
        if self.field_check(name, |_| ()) {
            self.block.get_field(name)
        } else {
            None
        }
    }

    pub fn field_value(&mut self, name: &'a str) -> Option<&Token> {
        if self.field_check(name, |v| match v {
            BlockOrValue::Token(_) => (),
            BlockOrValue::Block(s) => {
                error(s, ErrorKey::Validation, "expected value, found block");
            }
        }) {
            self.block.get_field_value(name)
        } else {
            None
        }
    }

    pub fn field_value_loca(&mut self, name: &'a str) {
        self.field_check(name, |v| match v {
            BlockOrValue::Token(t) => {
                self.data.localization.verify_exists(t);
            }
            BlockOrValue::Block(s) => {
                error(s, ErrorKey::Validation, "expected value, found block");
            }
        });
    }

    pub fn field_block(&mut self, name: &'a str) -> bool {
        self.field_check(name, |v| match v {
            BlockOrValue::Token(t) => {
                error(t, ErrorKey::Validation, "expected block, found value");
            }
            BlockOrValue::Block(_) => (),
        })
    }

    pub fn field_bool(&mut self, name: &'a str) -> bool {
        self.field_check(name, |v| match v {
            BlockOrValue::Token(t) if t.is("yes") || t.is("no") => (),
            BlockOrValue::Token(t) => {
                error(t, ErrorKey::Validation, "expected yes or no");
            }
            BlockOrValue::Block(s) => {
                error(s, ErrorKey::Validation, "expected value, found block");
            }
        })
    }

    pub fn field_integer(&mut self, name: &'a str) -> bool {
        self.field_check(name, |v| match v {
            BlockOrValue::Token(t) => {
                if t.as_str().parse::<i32>().is_err() {
                    error(t, ErrorKey::Validation, "expected integer");
                }
            }
            BlockOrValue::Block(s) => {
                error(s, ErrorKey::Validation, "expected value, found block");
            }
        })
    }

    pub fn field_float(&mut self, name: &'a str) -> bool {
        self.field_check(name, |v| match v {
            BlockOrValue::Token(t) => {
                if t.as_str().parse::<f64>().is_err() {
                    error(t, ErrorKey::Validation, "expected number");
                }
            }
            BlockOrValue::Block(s) => {
                error(s, ErrorKey::Validation, "expected value, found block");
            }
        })
    }

    pub fn field_choice(&mut self, name: &'a str, choices: &[&str]) -> bool {
        self.field_check(name, |v| match v {
            BlockOrValue::Token(t) => {
                if !choices.contains(&t.as_str()) {
                    let msg = format!("expected one of {}", choices.join(", "));
                    error(t, ErrorKey::Validation, &msg);
                }
            }
            BlockOrValue::Block(s) => {
                error(s, ErrorKey::Validation, "expected value, found block");
            }
        })
    }

    pub fn field_list(&mut self, name: &'a str) -> bool {
        self.field_check(name, |v| match v {
            BlockOrValue::Token(t) => {
                error(t, ErrorKey::Validation, "expected block, found value");
            }
            BlockOrValue::Block(s) => {
                let mut vec = Vec::new();
                for (k, _, v) in &s.v {
                    if let Some(key) = k {
                        warn(
                            key,
                            ErrorKey::Validation,
                            &format!("found key `{}`, expected only values", key),
                        );
                    }
                    match v {
                        BlockOrValue::Token(t) => vec.push(t.clone()),
                        BlockOrValue::Block(s) => {
                            error(s, ErrorKey::Validation, "expected value, found block");
                        }
                    }
                }
            }
        })
    }

    pub fn field_values(&mut self, name: &'a str) -> Vec<&Token> {
        self.known_fields.push(name);

        let mut vec = Vec::new();
        for (k, cmp, v) in &self.block.v {
            if let Some(key) = k {
                if key.is(name) {
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    match v {
                        BlockOrValue::Token(t) => vec.push(t),
                        BlockOrValue::Block(s) => {
                            error(s, ErrorKey::Validation, "expected value, found block");
                        }
                    }
                }
            }
        }
        vec
    }

    pub fn fields(&mut self, name: &'a str) -> bool {
        self.known_fields.push(name);

        let mut found = false;
        for (k, cmp, _) in &self.block.v {
            if let Some(key) = k {
                if key.is(name) {
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    found = true;
                }
            }
        }
        found
    }

    pub fn field_validated<F>(&mut self, name: &'a str, f: F) -> bool
    where
        F: Fn(&BlockOrValue, &Everything),
    {
        self.known_fields.push(name);

        let mut found = false;
        for (k, cmp, v) in &self.block.v {
            if let Some(key) = k {
                if key.is(name) {
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    f(v, self.data);
                    found = true;
                }
            }
        }
        found
    }

    pub fn field_validated_blocks<F>(&mut self, name: &'a str, f: F) -> bool
    where
        F: Fn(&Block, &Everything),
    {
        self.known_fields.push(name);

        let mut found = false;
        for (k, cmp, v) in &self.block.v {
            if let Some(key) = k {
                if key.is(name) {
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    match v {
                        BlockOrValue::Token(t) => {
                            error(t, ErrorKey::Validation, "expected block, found value");
                        }
                        BlockOrValue::Block(s) => f(s, self.data),
                    }
                    found = true;
                }
            }
        }
        found
    }

    pub fn field_validated_block<F>(&mut self, name: &'a str, f: F) -> bool
    where
        F: Fn(&Block, &Everything),
    {
        self.known_fields.push(name);
        let mut found = false;

        for (k, cmp, v) in &self.block.v {
            if let Some(key) = k {
                if key.is(name) {
                    if found {
                        warn(
                            key,
                            ErrorKey::Duplicate,
                            &format!("multiple definitions of `{}`, expected only one.", key),
                        );
                    }
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    match v {
                        BlockOrValue::Token(t) => {
                            error(t, ErrorKey::Validation, "expected block, found value");
                        }
                        BlockOrValue::Block(s) => f(s, self.data),
                    }
                    found = true;
                }
            }
        }
        found
    }

    pub fn field_blocks(&mut self, name: &'a str) -> bool {
        self.known_fields.push(name);

        let mut found = false;
        for (k, cmp, v) in &self.block.v {
            if let Some(key) = k {
                if key.is(name) {
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    match v {
                        BlockOrValue::Token(t) => {
                            error(t, ErrorKey::Validation, "expected block, found value");
                        }
                        BlockOrValue::Block(_) => (),
                    }
                    found = true;
                }
            }
        }
        found
    }

    pub fn req_tokens_integers_exactly(&mut self, expect: usize) {
        self.accepted_tokens = true;
        let mut found = 0;
        for (k, _, v) in &self.block.v {
            if k.is_none() {
                if let BlockOrValue::Token(t) = v {
                    if t.as_str().parse::<i32>().is_ok() {
                        found += 1;
                    } else {
                        error(t, ErrorKey::Validation, "expected integer");
                    }
                }
            }
        }
        if found != expect {
            let msg = format!("expected {} integers", expect);
            error(self.block, ErrorKey::Validation, &msg);
        }
    }

    pub fn advice_field(&mut self, name: &'a str, msg: &str) {
        self.known_fields.push(name);
        if let Some(key) = self.block.get_key(name) {
            advice(key, ErrorKey::Unneeded, msg);
        }
    }

    pub fn validate_history_blocks<F>(&mut self, f: F)
    where
        F: Fn(&Token, &Block, &Block, &Everything),
    {
        for (k, cmp, v) in &self.block.v {
            if let Some(key) = k {
                if Date::try_from(key).is_ok() {
                    self.known_fields.push(key.as_str());
                    if !matches!(cmp, Comparator::Eq) {
                        error(
                            key,
                            ErrorKey::Validation,
                            &format!("expected `{} =`, found `{}`", key, cmp),
                        );
                    }
                    match v {
                        BlockOrValue::Token(t) => {
                            error(t, ErrorKey::Validation, "expected block, found value");
                        }
                        BlockOrValue::Block(s) => f(key, s, self.block, self.data),
                    }
                }
            }
        }
    }

    // TODO: make this execute on drop, and provide another function
    // to tell it not to warn. This way, there's less risk of a function
    // just forgetting to call warn_remaining.
    pub fn warn_remaining(&mut self) {
        for (k, _, v) in &self.block.v {
            match k {
                Some(key) => {
                    if !self.known_fields.contains(&key.as_str()) {
                        warn(
                            key,
                            ErrorKey::Validation,
                            &format!("unknown field `{}`", key),
                        );
                    }
                }
                None => match v {
                    BlockOrValue::Token(t) => {
                        if !self.accepted_tokens {
                            warn(
                                t,
                                ErrorKey::Validation,
                                "found loose value, expected only `key =`",
                            );
                        }
                    }
                    BlockOrValue::Block(s) => {
                        if !self.accepted_blocks {
                            warn(
                                s,
                                ErrorKey::Validation,
                                "found sub-block, expected only `key =`",
                            );
                        }
                    }
                },
            }
        }
    }
}
