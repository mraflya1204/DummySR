//! Damage report for the end of a battle, built from the statistics the client sends in
//! `PVEBattleResultCsReq`. The server never sees individual hits; these totals are the client's
//! own bookkeeping.

use std::fmt::Write as _;

use proto::{BattleEndStatus, BattleStatistics};

/// Report text, one line per entry: header, Total DMG, DMG per AV, then one line per character.
pub fn build(stats: &BattleStatistics, end_status: i32) -> String {
    let mut characters: Vec<(u32, f64)> = stats
        .battle_avatar_list
        .iter()
        .map(|avatar| (avatar.id, avatar.total_damage))
        .collect();
    characters.sort_by(|a, b| b.1.total_cmp(&a.1));

    let total: f64 = characters.iter().map(|(_, damage)| damage).sum();
    let break_total: f64 = stats
        .battle_avatar_list
        .iter()
        .map(|avatar| avatar.total_break_damage)
        .sum();
    let av = elapsed_av(stats);

    let cycles = match stats.round_cnt {
        1 => String::from("1 cycle"),
        n => format!("{n} cycles"),
    };
    let av_text = match av {
        Some((value, false)) => format!("{value:.1} AV"),
        Some((value, true)) => format!("~{value:.0} AV estimated from cycles"),
        None => String::from("AV unknown"),
    };

    let mut out = String::new();
    let _ = writeln!(
        out,
        "Damage report: battle {}, {cycles}, {av_text}",
        status_label(end_status)
    );
    let _ = writeln!(out, "Total DMG: {}", grouped(total));
    match av {
        Some((value, _)) if value > 0.0 => {
            let _ = writeln!(out, "DMG per AV: {}", grouped(total / value));
        }
        _ => {
            let _ = writeln!(out, "DMG per AV: n/a");
        }
    }
    if break_total > 0.0 {
        let _ = writeln!(
            out,
            "Break DMG (listed separately): {}",
            grouped(break_total)
        );
    }

    for (id, damage) in &characters {
        let share = if total > 0.0 {
            damage / total * 100.0
        } else {
            0.0
        };
        let per_av = match av {
            Some((value, _)) if value > 0.0 => format!(", {}/AV", compact(damage / value)),
            _ => String::new(),
        };
        let _ = writeln!(
            out,
            "{}: {} ({share:.1}%{per_av})",
            avatar_label(*id),
            compact(*damage)
        );
    }

    out.trim_end().to_string()
}

/// Sum of the damage the enemies say they took. Logged next to the report as a cross-check:
/// if it matches Total DMG plus Break DMG, break damage isn't part of the characters' totals.
pub fn enemy_damage_taken(stats: &BattleStatistics) -> f64 {
    stats
        .monster_list
        .iter()
        .map(|monster| monster.total_damage_taken)
        .sum()
}

pub fn status_label(end_status: i32) -> &'static str {
    match BattleEndStatus::try_from(end_status) {
        Ok(BattleEndStatus::BattleEndWin) => "won",
        Ok(BattleEndStatus::BattleEndLose) => "lost",
        Ok(BattleEndStatus::BattleEndQuit) => "quit",
        _ => "ended",
    }
}

/// Action Value the fight lasted, and whether it's an estimate. The client reports it as
/// `total_delay_cumulate` ("delay" is the game's internal name for AV). If that's missing, it's
/// estimated from the cycle count (150 AV for the first cycle, 100 for each one after), which is
/// an upper bound because the last cycle may not have finished.
fn elapsed_av(stats: &BattleStatistics) -> Option<(f64, bool)> {
    if stats.total_delay_cumulate > 0.0 {
        Some((stats.total_delay_cumulate, false))
    } else if stats.round_cnt > 0 {
        Some((150.0 + 100.0 * f64::from(stats.round_cnt - 1), true))
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
        1506 => "Silver Wolf LV.999",
        _ => return None,
    })
}
