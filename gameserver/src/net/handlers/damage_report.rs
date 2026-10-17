//! Damage report for the end of a battle, built from the statistics the client sends in
//! `PVEBattleResultCsReq`. Uses the same accounting as Veritas: all damage dealt to enemies counts,
//! overkill and break damage included, and DMG per AV is damage divided by the total elapsed
//! Action Value, for the team and for each character.

use std::fmt::Write as _;

use proto::{BattleEndStatus, BattleStatistics};

/// The client stores elapsed AV in units of 10 AV; Veritas reads the same counter
/// (`ElapsedActionDelay`) and multiplies it by 10.
const AV_PER_DELAY_UNIT: f64 = 10.0;

pub struct Report {
    /// What the player sees.
    pub text: String,
    /// How the numbers were worked out, for the console and `last_battle_stats.txt`.
    pub notes: Vec<String>,
}

pub fn build(stats: &BattleStatistics, end_status: i32) -> Report {
    let mut notes = Vec::new();
    let sum = |values: &mut dyn Iterator<Item = f64>| values.fold(0.0, |acc, x| acc + x);

    // The enemies' side is always filled in and counts every hit in full, overkill included,
    // which is what Veritas totals. The characters' side is sometimes left at zero.
    let enemy_taken = sum(&mut stats.monster_list.iter().map(|m| m.total_damage_taken));
    let character_damage = sum(&mut stats.battle_avatar_list.iter().map(|a| a.total_damage));
    let break_damage = sum(&mut stats
        .battle_avatar_list
        .iter()
        .map(|a| a.total_break_damage));

    // Break damage is reported per character separately; add it when the characters' damage
    // leaves it out (their sum is then closer to what the enemies took with break added).
    let add_break = break_damage > 0.0
        && enemy_taken > 0.0
        && (enemy_taken - (character_damage + break_damage)).abs()
            < (enemy_taken - character_damage).abs();

    let mut characters: Vec<(u32, f64)> = stats
        .battle_avatar_list
        .iter()
        .map(|a| {
            let damage = if add_break {
                a.total_damage + a.total_break_damage
            } else {
                a.total_damage
            };
            (a.id, damage)
        })
        .collect();
    characters.sort_by(|a, b| b.1.total_cmp(&a.1));
    let attributed = sum(&mut characters.iter().map(|(_, damage)| *damage));
    let total = if enemy_taken > 0.0 {
        enemy_taken
    } else {
        attributed
    };

    notes.push(format!(
        "enemies took {enemy_taken:.0}; characters reported {character_damage:.0} \
         plus {break_damage:.0} break ({})",
        if add_break {
            "break added to characters"
        } else {
            "break already in characters' damage or none"
        }
    ));

    let av = elapsed_av(stats);
    notes.push(format!(
        "total_delay_cumulate {} x {AV_PER_DELAY_UNIT} = {:.1} AV, round_cnt {} \
         (completed cycles)",
        stats.total_delay_cumulate,
        stats.total_delay_cumulate * AV_PER_DELAY_UNIT,
        stats.round_cnt
    ));
    if let Some((value, false)) = av
        && value > cycle_end_av(stats.round_cnt) + 1.0
    {
        notes.push(String::from(
            "AV is past the end of the reported cycle; the AV conversion may be off",
        ));
    }

    let av_text = match av {
        Some((value, false)) => format!("{value:.1} AV"),
        Some((value, true)) => format!("~{value:.0} AV estimated from cycles"),
        None => String::from("AV unknown"),
    };
    let per_av = |damage: f64| match av {
        Some((value, _)) if value > 0.0 => Some(damage / value),
        _ => None,
    };

    let mut out = String::new();
    let _ = writeln!(
        out,
        "Damage report: battle {} in cycle {}, {av_text}",
        status_label(end_status),
        stats.round_cnt + 1
    );
    let _ = writeln!(out, "Total DMG: {}", grouped(total));
    let _ = writeln!(
        out,
        "DMG per AV: {}",
        per_av(total).map_or_else(|| String::from("n/a"), grouped)
    );
    if break_damage > 0.0 {
        let _ = writeln!(
            out,
            "Break DMG: {} (part of the total)",
            grouped(break_damage)
        );
    }

    if attributed > 0.0 {
        for (id, damage) in &characters {
            let share = damage / total * 100.0;
            let rate = per_av(*damage).map_or_else(String::new, |v| format!(", {}/AV", compact(v)));
            let _ = writeln!(
                out,
                "{}: {} ({share:.1}%{rate})",
                avatar_label(*id),
                compact(*damage)
            );
        }
        let other = total - attributed;
        if other > total * 0.005 {
            let _ = writeln!(
                out,
                "Not credited to a character: {} ({:.1}%)",
                compact(other),
                other / total * 100.0
            );
        }
    } else if !characters.is_empty() {
        let _ = writeln!(out, "By character: the game left this out for this battle");
    }

    Report {
        text: out.trim_end().to_string(),
        notes,
    }
}

pub fn status_label(end_status: i32) -> &'static str {
    match BattleEndStatus::try_from(end_status) {
        Ok(BattleEndStatus::BattleEndWin) => "won",
        Ok(BattleEndStatus::BattleEndLose) => "lost",
        Ok(BattleEndStatus::BattleEndQuit) => "quit",
        _ => "ended",
    }
}

/// AV at which cycle `completed + 1` ends: 150 for the first cycle, 100 more for each after.
fn cycle_end_av(completed: u32) -> f64 {
    150.0 + 100.0 * f64::from(completed)
}

/// Action Value the fight lasted, and whether it's an estimate. Normally the client's elapsed
/// counter (`total_delay_cumulate`, in units of 10 AV). If that's missing, it's estimated as the
/// end of the cycle the fight stopped in (`round_cnt` counts completed cycles), an upper bound.
fn elapsed_av(stats: &BattleStatistics) -> Option<(f64, bool)> {
    if stats.total_delay_cumulate > 0.0 {
        Some((stats.total_delay_cumulate * AV_PER_DELAY_UNIT, false))
    } else if stats.round_cnt > 0 {
        Some((cycle_end_av(stats.round_cnt), true))
    } else {
        None
    }
}

/// 12345678.9 -> "12,345,679"
fn grouped(value: f64) -> String {
    let digits = (value.max(0.0).round() as u64).to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 12345678.9 -> "12.35M"
fn compact(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1e9 {
        format!("{:.2}B", value / 1e9)
    } else if abs >= 1e6 {
        format!("{:.2}M", value / 1e6)
    } else if abs >= 1e3 {
        format!("{:.1}K", value / 1e3)
    } else {
        format!("{value:.0}")
    }
}

fn avatar_label(id: u32) -> String {
    avatar_name(id).map_or_else(|| format!("#{id}"), str::to_string)
}

/// Character names by avatar ID. IDs not listed here show as "#ID"; add them as needed.
fn avatar_name(id: u32) -> Option<&'static str> {
    Some(match id {
        8001..=8010 => "Trailblazer",
        1001 => "March 7th",
        1002 => "Dan Heng",
        1003 => "Himeko",
        1004 => "Welt",
        1005 => "Kafka",
        1006 => "Silver Wolf",
        1008 => "Arlan",
        1009 => "Asta",
        1013 => "Herta",
        1014 => "Saber",
        1015 => "Archer",
        1101 => "Bronya",
        1102 => "Seele",
        1103 => "Serval",
        1104 => "Gepard",
        1105 => "Natasha",
        1106 => "Pela",
        1107 => "Clara",
        1108 => "Sampo",
        1109 => "Hook",
        1110 => "Lynx",
        1111 => "Luka",
        1112 => "Topaz & Numby",
        1201 => "Qingque",
        1202 => "Tingyun",
        1203 => "Luocha",
        1204 => "Jing Yuan",
        1205 => "Blade",
        1206 => "Sushang",
        1207 => "Yukong",
        1208 => "Fu Xuan",
        1209 => "Yanqing",
        1210 => "Guinaifen",
        1211 => "Bailu",
        1212 => "Jingliu",
        1213 => "Dan Heng IL",
        1214 => "Xueyi",
        1215 => "Hanya",
        1217 => "Huohuo",
        1218 => "Jiaoqiu",
        1220 => "Feixiao",
        1221 => "Yunli",
        1222 => "Lingsha",
        1223 => "Moze",
        1224 => "March 7th (Hunt)",
        1225 => "Fugue",
        1301 => "Gallagher",
        1302 => "Argenti",
        1303 => "Ruan Mei",
        1304 => "Aventurine",
        1305 => "Dr. Ratio",
        1306 => "Sparkle",
        1307 => "Black Swan",
        1308 => "Acheron",
        1309 => "Robin",
        1310 => "Firefly",
        1312 => "Misha",
        1313 => "Sunday",
        1314 => "Jade",
        1315 => "Boothill",
        1317 => "Rappa",
        1321 => "Dahlia",
        1401 => "The Herta",
        1402 => "Aglaea",
        1403 => "Tribbie",
        1404 => "Mydei",
        1405 => "Anaxa",
        1406 => "Cipher",
        1407 => "Castorice",
        1408 => "Phainon",
        1409 => "Hyacine",
        1410 => "Hysilens",
        1412 => "Cerydra",
        1413 => "Evernight",
        1414 => "Dan Heng PT",
        1415 => "Cyrene",
        1501 => "Sparxie",
        1502 => "Yao Guang",
        1503 => "Pearl",
        1504 => "Ashveil",
        1505 => "Evanescia",
        1506 => "Silver Wolf LV.999",
        1507 => "Mortenax Blade",
        1510 => "Himeko Nova",
        1511 => "Aha",
        1512 => "Robin (summer)",
        1513 => "Aventurine (Waveflair)",
        _ => return None,
    })
}
