//! Pure rule helpers (damage, mastery, thresholds, range math).

use rand::Rng;

#[derive(Clone, Debug)]
pub struct DamageExprCache {
    cleaned: String,
    cleaned_nonpenetrating: String,
    is_lower_of: bool,
}

impl DamageExprCache {
    pub fn new(expr: &str) -> Self {
        let lower = expr.to_ascii_lowercase();
        let is_lower_of = lower.contains("lower of");
        let cleaned = clean_damage_expr(expr);
        let cleaned_nonpenetrating = cleaned.replace('p', "");
        Self {
            cleaned,
            cleaned_nonpenetrating,
            is_lower_of,
        }
    }

    pub fn roll(&self, rng: &mut impl Rng, nonpenetrating: bool) -> i32 {
        if nonpenetrating {
            roll_damage_expr_cached(&self.cleaned_nonpenetrating, self.is_lower_of, rng)
        } else {
            roll_damage_expr_cached(&self.cleaned, self.is_lower_of, rng)
        }
    }
}

pub fn roll_damage_expr_with_detail(expr: &str, rng: &mut impl Rng) -> (i32, String) {
    roll_damage_expr_with_detail_inner(expr, rng, false)
}

pub fn roll_damage_expr_with_detail_nonpenetrating(
    expr: &str,
    rng: &mut impl Rng,
) -> (i32, String) {
    roll_damage_expr_with_detail_inner(expr, rng, true)
}

fn roll_damage_expr_with_detail_inner(
    expr: &str,
    rng: &mut impl Rng,
    nonpenetrating: bool,
) -> (i32, String) {
    let lower = expr.to_ascii_lowercase();
    let is_lower_of = lower.contains("lower of");
    let cleaned = clean_damage_expr(expr);
    let cleaned = if nonpenetrating {
        cleaned.replace('p', "")
    } else {
        cleaned
    };
    if is_lower_of {
        let (a_total, a_detail) = evaluate_expression_with_detail(&cleaned, rng);
        let (b_total, b_detail) = evaluate_expression_with_detail(&cleaned, rng);
        let total = a_total.min(b_total);
        let detail = format!("lower of {} vs {}", a_detail, b_detail);
        (total, format!("[{}]", detail))
    } else {
        let (total, detail) = evaluate_expression_with_detail(&cleaned, rng);
        (total, format!("[{}]", detail))
    }
}

pub fn roll_damage_expr(expr: &str, rng: &mut impl Rng, nonpenetrating: bool) -> i32 {
    let lower = expr.to_ascii_lowercase();
    let is_lower_of = lower.contains("lower of");
    let cleaned = clean_damage_expr(expr);
    let cleaned = if nonpenetrating {
        cleaned.replace('p', "")
    } else {
        cleaned
    };
    if is_lower_of {
        let a_total = evaluate_expression(&cleaned, rng);
        let b_total = evaluate_expression(&cleaned, rng);
        a_total.min(b_total)
    } else {
        evaluate_expression(&cleaned, rng)
    }
}

fn roll_damage_expr_cached(
    cleaned: &str,
    is_lower_of: bool,
    rng: &mut impl Rng,
) -> i32 {
    if is_lower_of {
        let a_total = evaluate_expression(cleaned, rng);
        let b_total = evaluate_expression(cleaned, rng);
        a_total.min(b_total)
    } else {
        evaluate_expression(cleaned, rng)
    }
}

pub fn expected_damage_expr(expr: &str) -> f64 {
    if expr.trim().is_empty() {
        return 0.0;
    }
    let cleaned = clean_damage_expr(expr).to_ascii_lowercase();
    expected_expression(&cleaned)
}

pub fn effective_armor_value(raw: f64, armor_pen: i32) -> f64 {
    if raw < 5.0 || armor_pen <= 0 {
        return raw;
    }
    let extra = raw - 5.0;
    let reduced_extra = (extra - armor_pen as f64).max(0.0);
    5.0 + reduced_extra
}

pub fn clean_damage_expr(expr: &str) -> String {
    let first = expr.split(" and ").next().unwrap_or(expr);
    let lower = first.to_ascii_lowercase();
    let candidate = if let Some(pos) = lower.find("lower of") {
        &first[pos + "lower of".len()..]
    } else {
        first
    };
    let mut cleaned = String::new();
    for ch in candidate.chars() {
        if ch == '^' {
            break;
        }
        if ch.is_ascii_alphanumeric() || "+-()".contains(ch) {
            cleaned.push(ch);
        }
    }
    if cleaned.is_empty() {
        "d4p".to_string()
    } else {
        cleaned
    }
}

fn evaluate_expression(expr: &str, rng: &mut impl Rng) -> i32 {
    let mut total = 0;
    let mut idx = 0;
    let chars: Vec<char> = expr.chars().collect();
    while idx < chars.len() {
        let mut sign = 1;
        if chars[idx] == '+' {
            idx += 1;
        } else if chars[idx] == '-' {
            sign = -1;
            idx += 1;
        }

        let start = idx;
        let mut depth = 0;
        while idx < chars.len() {
            match chars[idx] {
                '(' => {
                    depth += 1;
                    idx += 1;
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                        idx += 1;
                    } else {
                        break;
                    }
                }
                '+' | '-' if depth == 0 => break,
                _ => idx += 1,
            }
        }

        let term = &expr[start..idx];
        if !term.is_empty() {
            let term_value = evaluate_term(term, rng);
            total += sign * term_value;
        }
    }
    total
}

fn expected_expression(expr: &str) -> f64 {
    if expr.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    let mut idx = 0;
    let chars: Vec<char> = expr.chars().collect();
    while idx < chars.len() {
        let mut sign = 1.0;
        if chars[idx] == '+' {
            idx += 1;
        } else if chars[idx] == '-' {
            sign = -1.0;
            idx += 1;
        }

        let start = idx;
        let mut depth = 0;
        while idx < chars.len() {
            match chars[idx] {
                '(' => {
                    depth += 1;
                    idx += 1;
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                        idx += 1;
                    } else {
                        break;
                    }
                }
                '+' | '-' if depth == 0 => break,
                _ => idx += 1,
            }
        }

        let term = &expr[start..idx];
        if !term.is_empty() {
            total += sign * expected_term(term);
        }
    }
    total
}

fn evaluate_term(term: &str, rng: &mut impl Rng) -> i32 {
    let trimmed = strip_outer_parens(term);

    if has_top_level_operator(trimmed) {
        return evaluate_expression(trimmed, rng);
    }

    if let Some(d_pos) = trimmed.find('d') {
        let count = if d_pos == 0 {
            1
        } else {
            trimmed[..d_pos].parse::<i32>().unwrap_or(1)
        };

        let after_d = &trimmed[d_pos + 1..];
        let mut digits_end = 0;
        for ch in after_d.chars() {
            if ch.is_ascii_digit() {
                digits_end += ch.len_utf8();
            } else {
                break;
            }
        }

        let (sides_str, rest) = after_d.split_at(digits_end);
        let sides = sides_str.parse::<i32>().unwrap_or(0);
        let penetrating = rest.starts_with('p');

        let mut subtotal = 0;
        for _ in 0..count {
            let roll = if penetrating {
                penetrating_roll(sides, rng)
            } else {
                standard_roll(sides, rng)
            };
            subtotal += roll;
        }
        subtotal
    } else {
        trimmed.parse::<i32>().unwrap_or(0)
    }
}

fn expected_term(term: &str) -> f64 {
    let trimmed = strip_outer_parens(term);

    if has_top_level_operator(trimmed) {
        return expected_expression(trimmed);
    }

    if let Some(d_pos) = trimmed.find('d') {
        let count = if d_pos == 0 {
            1.0
        } else {
            trimmed[..d_pos].parse::<f64>().unwrap_or(1.0)
        };

        let after_d = &trimmed[d_pos + 1..];
        let mut digits_end = 0;
        for ch in after_d.chars() {
            if ch.is_ascii_digit() {
                digits_end += ch.len_utf8();
            } else {
                break;
            }
        }

        let (sides_str, rest) = after_d.split_at(digits_end);
        let sides = sides_str.parse::<f64>().unwrap_or(0.0);
        let penetrating = rest.starts_with('p');

        let single = if penetrating {
            (sides + 2.0) / 2.0
        } else {
            (sides + 1.0) / 2.0
        };

        count * single
    } else {
        trimmed.parse::<f64>().unwrap_or(0.0)
    }
}

pub(crate) fn evaluate_expression_with_detail(
    expr: &str,
    rng: &mut impl Rng,
) -> (i32, String) {
    let mut total = 0;
    let mut detail = String::new();
    let mut idx = 0;
    let chars: Vec<char> = expr.chars().collect();
    while idx < chars.len() {
        let mut sign = 1;
        let mut sign_char = '+';
        if chars[idx] == '+' {
            idx += 1;
        } else if chars[idx] == '-' {
            sign = -1;
            sign_char = '-';
            idx += 1;
        }

        let start = idx;
        let mut depth = 0;
        while idx < chars.len() {
            match chars[idx] {
                '(' => {
                    depth += 1;
                    idx += 1;
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                        idx += 1;
                    } else {
                        break;
                    }
                }
                '+' | '-' if depth == 0 => break,
                _ => idx += 1,
            }
        }

        let term = &expr[start..idx];
        if !term.is_empty() {
            let (term_value, term_detail) = evaluate_term_with_detail(term, rng);
            total += sign * term_value;
            if !detail.is_empty() {
                detail.push(' ');
                detail.push(sign_char);
                detail.push(' ');
            } else if sign_char == '-' {
                detail.push('-');
            }
            detail.push_str(&term_detail);
        }
    }
    (total, detail)
}

fn evaluate_term_with_detail(term: &str, rng: &mut impl Rng) -> (i32, String) {
    let trimmed = strip_outer_parens(term);

    if has_top_level_operator(trimmed) {
        return evaluate_expression_with_detail(trimmed, rng);
    }

    if let Some(d_pos) = trimmed.find('d') {
        let count = if d_pos == 0 {
            1
        } else {
            trimmed[..d_pos].parse::<i32>().unwrap_or(1)
        };

        let after_d = &trimmed[d_pos + 1..];
        let mut digits_end = 0;
        for ch in after_d.chars() {
            if ch.is_ascii_digit() {
                digits_end += ch.len_utf8();
            } else {
                break;
            }
        }

        let (sides_str, rest) = after_d.split_at(digits_end);
        let sides = sides_str.parse::<i32>().unwrap_or(0);
        let penetrating = rest.starts_with('p');

        let mut subtotal = 0;
        let mut rolls = Vec::new();
        for _ in 0..count {
            let roll = if penetrating {
                penetrating_roll(sides, rng)
            } else {
                standard_roll(sides, rng)
            };
            rolls.push(roll);
            subtotal += roll;
        }
        let kind = if penetrating { "d" } else { "d" };
        let detail = format!(
            "{}{}{}={}",
            count,
            kind,
            sides,
            rolls
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("+")
        );
        (subtotal, detail)
    } else {
        let value = trimmed.parse::<i32>().unwrap_or(0);
        (value, value.to_string())
    }
}

pub fn penetrating_roll(sides: i32, rng: &mut impl Rng) -> i32 {
    if sides <= 1 {
        return sides.max(0);
    }
    penetrating_roll_with(sides, || rng.gen_range(1..=sides))
}

pub fn penetrating_roll_with(mut sides: i32, mut next_roll: impl FnMut() -> i32) -> i32 {
    if sides <= 1 {
        return sides.max(0);
    }
    if sides < 0 {
        sides = 0;
    }
    let mut total = 0;
    let mut first = true;
    loop {
        let roll = next_roll().clamp(1, sides);
        if first {
            total += roll;
            first = false;
        } else {
            total += roll - 1;
        }
        if roll != sides {
            break;
        }
    }
    total
}

pub fn standard_roll(sides: i32, rng: &mut impl Rng) -> i32 {
    if sides <= 1 {
        return sides.max(0);
    }
    rng.gen_range(1..=sides)
}

fn strip_outer_parens(mut s: &str) -> &str {
    loop {
        let bytes = s.as_bytes();
        if bytes.len() >= 2 && bytes[0] == b'(' && bytes[bytes.len() - 1] == b')' {
            let mut depth = 0;
            let mut balanced = true;
            for (i, ch) in s.chars().enumerate() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 && i != s.len() - 1 {
                            balanced = false;
                            break;
                        }
                    }
                    _ => (),
                }
            }
            if balanced && depth == 0 {
                s = &s[1..s.len() - 1];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    s
}

fn has_top_level_operator(s: &str) -> bool {
    let mut depth: i32 = 0;
    for ch in s.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '+' | '-' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}
