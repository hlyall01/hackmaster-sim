//! Validated, allocation-free execution of damage expressions.
use super::{
    expected_die_with_trigger_count, penetrating_roll, penetrating_roll_trigger_set, standard_roll,
};
use rand::Rng;

#[derive(Clone, Debug)]
pub(super) struct Expression(Vec<Term>);
#[derive(Clone, Debug)]
struct Term {
    sign: i32,
    value: Value,
}
#[derive(Clone, Debug)]
enum Value {
    Constant(i32),
    Dice {
        count: i32,
        sides: i32,
        penetrating: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DamageExprError(pub(super) &'static str);
impl std::fmt::Display for DamageExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for DamageExprError {}

impl Expression {
    pub(super) fn empty() -> Self {
        Self(Vec::new())
    }
    pub(super) fn parse(input: &str) -> Result<Self, DamageExprError> {
        // A dash is the catalog's explicit no-damage marker.
        if input == "-" {
            return Ok(Self::empty());
        }
        let mut parser = Parser {
            bytes: input.as_bytes(),
            pos: 0,
            terms: Vec::new(),
        };
        parser.expression(1, 0)?;
        if parser.pos != parser.bytes.len() {
            return Err(DamageExprError("Unexpected closing parenthesis"));
        }
        Ok(Self(parser.terms))
    }
    pub(super) fn roll(
        &self,
        rng: &mut impl Rng,
        nonpenetrating: bool,
        triggers: Option<&[i32]>,
        max_minus_one: bool,
    ) -> i32 {
        let mut total = 0i32;
        for term in &self.0 {
            let value = match term.value {
                Value::Constant(value) => value,
                Value::Dice {
                    count,
                    sides,
                    penetrating,
                } => {
                    let mut subtotal = 0i32;
                    for _ in 0..count {
                        let roll = if !penetrating || nonpenetrating {
                            standard_roll(sides, rng)
                        } else if max_minus_one {
                            penetrating_roll_trigger_set(sides, &[(sides - 1).max(1), sides], rng)
                        } else if let Some(triggers) = triggers.filter(|_| sides == 6) {
                            penetrating_roll_trigger_set(sides, triggers, rng)
                        } else {
                            penetrating_roll(sides, rng)
                        };
                        subtotal = subtotal.saturating_add(roll);
                    }
                    subtotal
                }
            };
            total = total.saturating_add(term.sign.saturating_mul(value));
        }
        total
    }
    pub(super) fn expected(
        &self,
        nonpenetrating: bool,
        triggers: Option<&[i32]>,
        max_minus_one: bool,
    ) -> f64 {
        self.0
            .iter()
            .map(|term| {
                f64::from(term.sign)
                    * match term.value {
                        Value::Constant(value) => f64::from(value),
                        Value::Dice {
                            count,
                            sides,
                            penetrating,
                        } => {
                            let trigger_count = if nonpenetrating || !penetrating {
                                0
                            } else if sides == 6 {
                                triggers.map_or(if max_minus_one { 2 } else { 1 }, <[i32]>::len)
                            } else if max_minus_one {
                                2
                            } else {
                                1
                            };
                            f64::from(count)
                                * expected_die_with_trigger_count(f64::from(sides), trigger_count)
                        }
                    }
            })
            .sum()
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    terms: Vec<Term>,
}
impl Parser<'_> {
    fn expression(&mut self, outer_sign: i32, depth: usize) -> Result<(), DamageExprError> {
        if depth > 64 {
            return Err(DamageExprError("Damage expression nesting is too deep"));
        }
        let mut first = true;
        loop {
            let sign = match self.bytes.get(self.pos) {
                Some(b'+') => {
                    self.pos += 1;
                    outer_sign
                }
                Some(b'-') => {
                    self.pos += 1;
                    -outer_sign
                }
                _ if first => outer_sign,
                _ => return Err(DamageExprError("Expected '+' or '-' between damage terms")),
            };
            first = false;
            if self.bytes.get(self.pos) == Some(&b'(') {
                self.pos += 1;
                self.expression(sign, depth + 1)?;
                if self.bytes.get(self.pos) != Some(&b')') {
                    return Err(DamageExprError("Unclosed parenthesis"));
                }
                self.pos += 1;
            } else {
                let count = self.number()?;
                let value = if self.bytes.get(self.pos) == Some(&b'd') {
                    self.pos += 1;
                    let sides = self.number()?.ok_or(DamageExprError("Missing die size"))?;
                    let count = count.unwrap_or(1);
                    if count > 1_000_000 {
                        return Err(DamageExprError("Too many dice in one term"));
                    }
                    if sides == 0 {
                        return Err(DamageExprError("Die size must be positive"));
                    }
                    let penetrating = self.bytes.get(self.pos) == Some(&b'p');
                    if penetrating {
                        self.pos += 1;
                    }
                    Value::Dice {
                        count,
                        sides,
                        penetrating,
                    }
                } else {
                    Value::Constant(count.ok_or(DamageExprError(
                        "Expected a number, die, or parenthesized expression",
                    ))?)
                };
                if self.terms.len() >= 4096 {
                    return Err(DamageExprError("Too many damage terms"));
                }
                self.terms.push(Term { sign, value });
            }
            if self.pos == self.bytes.len() || self.bytes[self.pos] == b')' {
                return Ok(());
            }
        }
    }
    fn number(&mut self) -> Result<Option<i32>, DamageExprError> {
        let start = self.pos;
        let mut value = 0i32;
        while let Some(ch @ b'0'..=b'9') = self.bytes.get(self.pos) {
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(i32::from(*ch - b'0')))
                .ok_or(DamageExprError("Number is too large"))?;
            self.pos += 1;
        }
        Ok((self.pos != start).then_some(value))
    }
}
