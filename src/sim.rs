use rand::{Rng, SeedableRng};

#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    pub start_distance: f32,
    pub stop_distance: f32,
}

impl SimConfig {
    pub fn new(start_distance: f32, stop_distance: f32) -> Self {
        Self {
            start_distance,
            stop_distance,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SimActor {
    pub position: f32,
}

#[derive(Clone, Debug)]
pub struct Combatant {
    pub name: String,
    pub weapon_name: String,
    pub attack_bonus: i32,
    pub defense_mod: i32,
    pub armor_dr: i32,
    pub armor_penetration: i32,
    pub damage_expr: String,
    pub strength_damage: i32,
    pub weapon_speed: f32,
    pub reach_ft: f32,
    pub move_speed: f32,
    pub two_hand_grip: bool,
    pub has_weapon: bool,
    pub weapon_defense_always: bool,
    pub max_hp: i32,
    pub hp: i32,
    pub next_attack_time: Option<f32>,
    pub defense_plus_four_ready: bool,
}

impl Combatant {
    pub fn new(
        name: String,
        weapon_name: String,
        attack_bonus: i32,
        defense_mod: i32,
        armor_dr: i32,
        armor_penetration: i32,
        damage_expr: String,
        strength_damage: i32,
        weapon_speed: f32,
        reach_ft: f32,
        move_speed: f32,
        two_hand_grip: bool,
        has_weapon: bool,
        weapon_defense_always: bool,
        max_hp: i32,
    ) -> Self {
        Self {
            name,
            weapon_name,
            attack_bonus,
            defense_mod,
            armor_dr,
            armor_penetration,
            damage_expr,
            strength_damage,
            weapon_speed,
            reach_ft,
            move_speed,
            two_hand_grip,
            has_weapon,
            weapon_defense_always,
            max_hp,
            hp: max_hp,
            next_attack_time: None,
            defense_plus_four_ready: false,
        }
    }

    fn reset_hp(&mut self) {
        self.hp = self.max_hp;
        self.next_attack_time = None;
        self.defense_plus_four_ready = false;
    }
}

impl Default for Combatant {
    fn default() -> Self {
        Self {
            name: "Combatant".to_string(),
            weapon_name: "Weapon".to_string(),
            attack_bonus: 0,
            defense_mod: 0,
            armor_dr: 0,
            armor_penetration: 0,
            damage_expr: "d4p".to_string(),
            strength_damage: 0,
            weapon_speed: 10.0,
            reach_ft: 1.0,
            move_speed: 5.0,
            two_hand_grip: false,
            has_weapon: false,
            weapon_defense_always: false,
            max_hp: 10,
            hp: 10,
            next_attack_time: None,
            defense_plus_four_ready: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimState {
    pub config: SimConfig,
    pub actors: [SimActor; 2],
    pub combatants: [Combatant; 2],
    pub elapsed_seconds: u32,
    pub done: bool,
    pub last_event: Option<String>,
    pub combat_log: Vec<String>,
    rng: rand::rngs::StdRng,
    tick_accum: f32,
}

impl SimState {
    pub fn new(config: SimConfig) -> Self {
        Self {
            config,
            actors: [
                SimActor { position: 0.0 },
                SimActor {
                    position: config.start_distance,
                },
            ],
            combatants: [Combatant::default(), Combatant::default()],
            elapsed_seconds: 0,
            done: false,
            last_event: None,
            combat_log: Vec::new(),
            rng: rand::rngs::StdRng::seed_from_u64(1),
            tick_accum: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.actors[0].position = 0.0;
        self.actors[1].position = self.config.start_distance;
        self.elapsed_seconds = 0;
        self.done = false;
        self.last_event = None;
        self.combat_log.clear();
        self.rng = rand::rngs::StdRng::seed_from_u64(1);
        for combatant in &mut self.combatants {
            combatant.reset_hp();
        }
        self.tick_accum = 0.0;
    }

    pub fn reset_with_combatants(&mut self, combatants: [Combatant; 2]) {
        self.combatants = combatants;
        self.reset();
    }

    pub fn update(&mut self, dt: f32) {
        if self.done {
            return;
        }
        self.tick_accum += dt;
        while self.tick_accum >= 1.0 {
            self.tick_accum -= 1.0;
            self.tick();
            if self.done {
                break;
            }
        }
    }

    pub fn tick(&mut self) {
        if self.done {
            return;
        }
        let distance = self.distance();
        let step_a = self.combatants[0].move_speed.max(0.0);
        let step_b = self.combatants[1].move_speed.max(0.0);
        let reach_a = self.combatants[0].reach_ft.max(1.0);
        let reach_b = self.combatants[1].reach_ft.max(1.0);
        let max_reach = self.config.stop_distance.max(1.0);
        let min_reach = reach_a.min(reach_b);

        if distance > max_reach {
            self.actors[0].position += step_a;
            self.actors[1].position -= step_b;
            for combatant in &mut self.combatants {
                combatant.next_attack_time = None;
            }
        } else {
            self.resolve_combat_round();
            let distance = self.distance();
            if distance > min_reach {
                if reach_a < reach_b {
                    self.actors[0].position += step_a;
                } else if reach_b < reach_a {
                    self.actors[1].position -= step_b;
                }
            }
        }
        self.elapsed_seconds += 1;
    }

    pub fn distance(&self) -> f32 {
        (self.actors[1].position - self.actors[0].position).max(0.0)
    }

    fn resolve_combat_round(&mut self) {
        let now = self.elapsed_seconds as f32;
        let distance = self.distance();
        let mut events = Vec::new();
        for (attacker_idx, defender_idx) in [(0usize, 1usize), (1usize, 0usize)] {
            if self.combatants[attacker_idx].hp <= 0 || self.combatants[defender_idx].hp <= 0 {
                continue;
            }
            if distance > self.combatants[attacker_idx].reach_ft.max(1.0) {
                continue;
            }
            if self.combatants[attacker_idx].next_attack_time.is_none() {
                let attacker_reach = self.combatants[attacker_idx].reach_ft;
                let defender_reach = self.combatants[defender_idx].reach_ft;
                let delay = if attacker_reach < defender_reach { 1.0 } else { 0.0 };
                self.combatants[attacker_idx].next_attack_time = Some(now + delay);
            }
            let next_attack = self.combatants[attacker_idx]
                .next_attack_time
                .unwrap_or(now);
            if now + 0.0001 >= next_attack {
                let event =
                    resolve_attack(&mut self.combatants, attacker_idx, defender_idx, &mut self.rng);
                events.push(event);
                let speed = self.combatants[attacker_idx].weapon_speed.max(1.0);
                self.combatants[attacker_idx].next_attack_time = Some(next_attack + speed);
                if self.combatants[defender_idx].hp <= 0 {
                    self.done = true;
                    break;
                }
            }
        }
        if !events.is_empty() {
            let line = format!("t={}s | {}", self.elapsed_seconds, events.join(" | "));
            self.last_event = Some(line.clone());
            self.combat_log.push(line);
        }
    }
}

fn resolve_attack(
    combatants: &mut [Combatant; 2],
    attacker_idx: usize,
    defender_idx: usize,
    rng: &mut impl Rng,
) -> String {
    let (attack_bonus, damage_expr, strength_damage, weapon_name, armor_penetration) = {
        let attacker = &combatants[attacker_idx];
        (
            attacker.attack_bonus,
            attacker.damage_expr.clone(),
            attacker.strength_damage,
            attacker.weapon_name.clone(),
            attacker.armor_penetration,
        )
    };
    let defense_mod = combatants[defender_idx].defense_mod;
    let armor_dr = combatants[defender_idx].armor_dr;
    let defense_bonus = if combatants[defender_idx].weapon_defense_always
        || (combatants[defender_idx].two_hand_grip
            && combatants[defender_idx].defense_plus_four_ready)
    {
        4
    } else {
        0
    };

    let attack_die = penetrating_roll(20, rng);
    let defense_die = penetrating_roll(20, rng);
    let attack_roll = attack_die + attack_bonus;
    let defense_roll = defense_die + defense_mod + defense_bonus;
        let mut damage = 0;
        let mut hit = false;
        let mut damage_detail = "[0]".to_string();

    if attack_roll >= defense_roll {
        hit = true;
            let (rolled_damage, detail) = roll_damage_expr_with_detail(&damage_expr, rng);
            let mut raw = rolled_damage + strength_damage;
            if raw < 0 {
                raw = 0;
            }
            damage_detail = detail;
        let mut effective_dr = armor_dr;
        if armor_dr >= 5 {
            effective_dr = (armor_dr - armor_penetration).max(0);
        }
        damage = (raw - effective_dr).max(1);
        combatants[defender_idx].hp -= damage;
    }

    let attacker_name = combatants[attacker_idx].name.clone();
    let defender_name = combatants[defender_idx].name.clone();
    if combatants[defender_idx].two_hand_grip
        && combatants[defender_idx].defense_plus_four_ready
        && combatants[defender_idx].has_weapon
        && !combatants[defender_idx].weapon_defense_always
    {
        combatants[defender_idx].defense_plus_four_ready = false;
    }
    if combatants[attacker_idx].two_hand_grip
        && combatants[attacker_idx].has_weapon
        && !combatants[attacker_idx].weapon_defense_always
    {
        combatants[attacker_idx].defense_plus_four_ready = true;
    }
    if hit {
        format!(
            "{} hits {} with {} (atk {} [d20p={}] vs def {} [d20p={}]) for {} dmg {} (hp {})",
            attacker_name,
            defender_name,
            weapon_name,
            attack_roll,
            attack_die,
            defense_roll,
            defense_die,
            damage,
            damage_detail,
            combatants[defender_idx].hp.max(0)
        )
    } else {
        format!(
            "{} misses {} with {} (atk {} [d20p={}] vs def {} [d20p={}])",
            attacker_name,
            defender_name,
            weapon_name,
            attack_roll,
            attack_die,
            defense_roll,
            defense_die
        )
    }
}

fn roll_damage_expr_with_detail(expr: &str, rng: &mut impl Rng) -> (i32, String) {
    let cleaned = clean_damage_expr(expr);
    let (total, detail) = evaluate_expression_with_detail(&cleaned, rng);
    (total, format!("[{}]", detail))
}

fn clean_damage_expr(expr: &str) -> String {
    let first = expr.split(" and ").next().unwrap_or(expr);
    let mut cleaned = String::new();
    for ch in first.chars() {
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

fn evaluate_expression_with_detail(expr: &str, rng: &mut impl Rng) -> (i32, String) {
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
fn penetrating_roll(sides: i32, rng: &mut impl Rng) -> i32 {
    if sides <= 1 {
        return sides.max(0);
    }
    let mut total = 0;
    loop {
        let roll = rng.gen_range(1..=sides);
        total += roll;
        if roll != sides {
            break;
        }
    }
    total
}

fn standard_roll(sides: i32, rng: &mut impl Rng) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn make_state(attacker: Combatant, defender: Combatant) -> SimState {
        let mut state = SimState::new(SimConfig::new(10.0, 1.0));
        state.combatants = [attacker, defender];
        state
    }

    #[test]
    fn attack_miss_does_no_damage() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            1000,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert_eq!(state.combatants[1].hp, 20);
    }

    #[test]
    fn damage_respects_dr_under_five() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            2,
            "1d1".to_string(),
            5,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            4,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert_eq!(state.combatants[1].hp, 18);
    }

    #[test]
    fn damage_applies_armor_penetration_when_dr_high() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            2,
            "1d1".to_string(),
            5,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            6,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert_eq!(state.combatants[1].hp, 18);
    }

    #[test]
    fn negative_penetration_increases_effective_dr() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            -1,
            "1d1".to_string(),
            5,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            6,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert_eq!(state.combatants[1].hp, 19);
    }

    #[test]
    fn damage_minimum_is_one_after_dr() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            10,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert_eq!(state.combatants[1].hp, 19);
    }

    struct FixedRng(u64);

    impl rand::RngCore for FixedRng {
        fn next_u32(&mut self) -> u32 {
            self.0 as u32
        }

        fn next_u64(&mut self) -> u64 {
            self.0
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for byte in dest.iter_mut() {
                *byte = self.0 as u8;
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    #[test]
    fn two_hand_grip_bonus_applies_once_between_attacks() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            true,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = FixedRng(0);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert!(state.combatants[0].defense_plus_four_ready);

        let mut rng = FixedRng(0);
        let _ = resolve_attack(&mut state.combatants, 1, 0, &mut rng);
        assert!(!state.combatants[0].defense_plus_four_ready);
    }

    #[test]
    fn poleaxe_always_gets_defense_bonus() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Poleaxe".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            true,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = FixedRng(0);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert_eq!(state.combatants[1].hp, 20);
    }

    #[test]
    fn one_handed_weapon_does_not_grant_defense_bonus() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Short Sword".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = FixedRng(0);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert!(!state.combatants[1].defense_plus_four_ready);
    }

    #[test]
    fn defense_always_applies_without_two_hand_grip() {
        let attacker = Combatant::new(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            false,
            20,
        );
        let defender = Combatant::new(
            "Defender".to_string(),
            "Polehammer".to_string(),
            0,
            0,
            0,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            true,
            true,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = FixedRng(0);
        let _ = resolve_attack(&mut state.combatants, 0, 1, &mut rng);
        assert_eq!(state.combatants[1].hp, 20);
    }
}
