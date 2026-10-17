use common::structs::{EX_RANKS, EnemyOverride, Monster, MultiPathAvatar, PearlMode};
use proto::chat_data::ExtendType;
use std::path::Path;
use tokio::fs;

use crate::{
    net::PlayerSession,
    util::{self, cur_timestamp_ms},
};

use super::*;

const SERVER_UID: u32 = 727;
const SERVER_HEAD_ICON: u32 = 201402;
const SERVER_CHAT_BUBBLE_ID: u32 = 220005;
const SERVER_CHAT_HISTORY: &[&str] = &[
    "'exrank {avatar_id/all} {on/off}' extra Eidolon for Jing Yuan, Dan Heng IL, Acheron ('exrank' lists them)",
    "'dummy {1-5} {monster_id}' one wave of copies of an enemy with 4 billion HP ('dummy off' to undo)",
    "'pearl {target/active/marker/self}' how Pearl's technique buff is attached (try them in turn)",
    "'enemy reset' clear all enemy overrides, 'enemy' shows current ones",
    "'enemy {hp/hppct/level/elite/hardlevel} {value/off}' override enemy stats for the next battle",
    "'lua {path_to_lua_script}' execute lua script",
    "'sw {on/off}' enable/disable silver wolf global buff",
    "'castorice {on/off}' enable/disable castorice global buff",
    "'sync' to synchronize stats between json and in-game view",
    "'mc {mc_id}' mc_id can be set from 8001 to 8008",
    "'march {march_id}' march_id can be set 1001 or 1224",
    "available commands:",
    "visit srtools.neonteam.dev to configure the PS! (you configure relics, equipment, monsters from there)",
];

pub async fn on_get_friend_login_info_cs_req(
    _session: &mut PlayerSession,
    _req: &GetFriendLoginInfoCsReq,
    res: &mut GetFriendLoginInfoScRsp,
) {
    res.black_uid_list = vec![SERVER_UID];
    res.friend_uid_list = vec![SERVER_UID];
}

pub async fn on_get_friend_list_info_cs_req(
    _session: &mut PlayerSession,
    _req: &GetFriendListInfoCsReq,
    res: &mut GetFriendListInfoScRsp,
) {
    res.friend_list = vec![FriendSimpleInfo {
        remark_name: String::from("RobinSR"),
        player_info: Some(PlayerSimpleInfo {
            uid: SERVER_UID,
            platform: PlatformType::Pc.into(),
            online_status: FriendOnlineStatus::Online.into(),
            head_icon: SERVER_HEAD_ICON,
            chat_bubble_id: SERVER_CHAT_BUBBLE_ID,
            level: 70,
            nickname: String::from("Server"),
            signature: String::from("omg"),
            ..Default::default()
        }),
        is_marked: true,
        create_time: 0,
        ..Default::default()
    }];
}

pub async fn on_get_private_chat_history_cs_req(
    session: &mut PlayerSession,
    req: &GetPrivateChatHistoryCsReq,
    res: &mut GetPrivateChatHistoryScRsp,
) {
    // Newest first: recent Server messages (damage reports), then the command help, dated just
    // before the oldest of them.
    let help_time = session
        .server_message_log
        .first()
        .map_or_else(cur_timestamp_ms, |(time, _)| time.saturating_sub(1000));
    res.chat_message_list = session
        .server_message_log
        .iter()
        .rev()
        .map(|(time, text)| server_chat_message(*time, text))
        .chain(
            SERVER_CHAT_HISTORY
                .iter()
                .map(|text| server_chat_message(help_time, text)),
        )
        .collect();
    res.target_side = req.target_side;
    res.contact_side = SERVER_UID;
}

fn server_chat_message(create_time: u64, text: &str) -> ChatMessageData {
    ChatMessageData {
        create_time,
        ckhpffenobe: Some(Eknabklpeel {
            kpobmnlklok: 1,
            role_id: SERVER_UID,
        }),
        bkoalkhdlob: Some(Eknabklpeel {
            kpobmnlklok: 1,
            role_id: SERVER_UID,
        }),
        message_datas: vec![MessageChatData {
            message_type: 1,
            chat_data: Some(ChatData {
                extend_type: Some(ExtendType::MessageText(text.to_string())),
            }),
        }],
        ..Default::default()
    }
}

pub async fn on_send_msg_cs_req(
    session: &mut PlayerSession,
    body: &SendMsgCsReq,
    _res: &mut SendMsgScRsp,
) {
    let Some(json) = session.json_data.get_mut() else {
        tracing::error!("data is not set!");
        return;
    };

    let msg = body
        .message_datas
        .as_ref()
        .and_then(|x| x.chat_data.as_ref())
        .and_then(|x| x.extend_type.as_ref())
        .and_then(|x| match x {
            ExtendType::MessageText(s) => Some(s.as_str()),
            _ => Option::<&str>::None,
        })
        .unwrap_or("");

    if let Some((cmd, args)) = parse_command(msg) {
        match cmd {
            "sync" => {
                let _ = session.sync_player().await;
                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        String::from("Inventory Synced"),
                    ))
                    .await
                    .unwrap();
            }
            "sw" | "castorice" => {
                let status = args.first().unwrap_or(&"on").to_lowercase();
                let enabled = match status.as_str() {
                    "on" | "1" | "true" => true,
                    "off" | "0" | "false" => false,
                    _ => true,
                };

                if cmd == "sw" {
                    json.enable_sw_global = Some(enabled);
                } else {
                    json.enable_castorice_global = Some(enabled);
                }

                json.save_persistent().await;

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        format!(
                            "{} Global Buff: {}",
                            if cmd == "sw" { "SW" } else { "Castorice" },
                            if enabled { "Enabled" } else { "Disabled" }
                        ),
                    ))
                    .await
                    .unwrap();
            }
            "mc" => {
                let mc = MultiPathAvatar::from(
                    args.first()
                        .unwrap_or(&"")
                        .parse::<u32>()
                        .unwrap_or(json.main_character as u32),
                );

                json.main_character = mc;
                json.save_persistent().await;

                session
                    .send(AvatarPathChangedNotify {
                        base_avatar_id: 8001,
                        cur_multi_path_avatar_type: mc as i32,
                    })
                    .await
                    .unwrap();

                let _ = session.sync_player().await;

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        format!("Success change mc to {mc:#?}"),
                    ))
                    .await
                    .unwrap();
            }
            "march" => {
                let mut march_type = MultiPathAvatar::from(
                    args.first()
                        .unwrap_or(&"")
                        .parse::<u32>()
                        .unwrap_or(json.march_type as u32),
                );

                if march_type != MultiPathAvatar::MarchPreservation
                    && march_type != MultiPathAvatar::MarchHunt
                {
                    march_type = MultiPathAvatar::MarchHunt
                }

                json.march_type = march_type;
                json.save_persistent().await;

                session
                    .send(AvatarPathChangedNotify {
                        base_avatar_id: 1001,
                        cur_multi_path_avatar_type: march_type as i32,
                    })
                    .await
                    .unwrap();

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        format!("Success change march to {march_type:#?}"),
                    ))
                    .await
                    .unwrap();
            }
            "lua" => {
                let path = Path::new(args.first().unwrap_or(&""));

                if !path.is_file() {
                    session
                        .send(create_send_message(
                            25,
                            SERVER_UID,
                            body.message_datas
                                .as_ref()
                                .map(|v| v.message_type)
                                .unwrap_or_default(),
                            body.chat_type,
                            format!("File {path:?} does not exist!"),
                        ))
                        .await
                        .unwrap();
                }

                let data = match fs::read(&path).await {
                    Ok(file) => file,
                    Err(err) => {
                        session
                            .send(create_send_message(
                                25,
                                SERVER_UID,
                                body.message_datas
                                    .as_ref()
                                    .map(|v| v.message_type)
                                    .unwrap_or_default(),
                                body.chat_type,
                                format!("Failed to read file: {err:?}"),
                            ))
                            .await
                            .unwrap();

                        return;
                    }
                };

                session
                    .send(ClientDownloadDataScNotify {
                        download_data: Some(ClientDownloadData {
                            version: 51,
                            time: util::cur_timestamp_ms() as i64,
                            data,
                            ..Default::default()
                        }),
                    })
                    .await
                    .unwrap();

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        format!("Executed {path:?}"),
                    ))
                    .await
                    .unwrap();
            }
            "pearl" => {
                let reply = match args.first().map(|v| v.to_lowercase()) {
                    None => format!(
                        "Pearl mode: {} (options: target, active, marker, self)",
                        json.pearl_mode.name()
                    ),
                    Some(arg) => match PearlMode::ALL.into_iter().find(|m| m.name() == arg) {
                        Some(mode) => {
                            json.pearl_mode = mode;
                            json.save_persistent().await;
                            format!(
                                "Pearl mode: {} (next battle: check Pearl's Certified Banger \
                                 and Deep Learning on the character you control)",
                                mode.name()
                            )
                        }
                        None => String::from("Usage: pearl target|active|marker|self"),
                    },
                };

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        reply,
                    ))
                    .await
                    .unwrap();
            }
            "exrank" => {
                let reply = edit_ex_rank(&mut json.ex_rank_off, &args);
                json.save_persistent().await;
                // refresh the character screens with the new ex rank state
                let _ = session.sync_player().await;

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        reply,
                    ))
                    .await
                    .unwrap();
            }
            "dummy" => {
                let reply = edit_dummy(
                    &mut json.enemy_override,
                    &args,
                    &json.battle_config.monsters,
                );
                json.save_persistent().await;

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        reply,
                    ))
                    .await
                    .unwrap();
            }
            "enemy" => {
                let reply = edit_enemy_override(&mut json.enemy_override, &args);
                json.save_persistent().await;

                session
                    .send(create_send_message(
                        25,
                        SERVER_UID,
                        body.message_datas
                            .as_ref()
                            .map(|v| v.message_type)
                            .unwrap_or_default(),
                        body.chat_type,
                        reply,
                    ))
                    .await
                    .unwrap();
            }
            _ => {}
        }
    }
}

/// `exrank` chat command: turn ex ranks (extra Eidolons) on or off. All are on by default.
///   exrank               list them
///   exrank 1308 off      switch Acheron's off
///   exrank all on        switch every one on
fn edit_ex_rank(off: &mut Vec<u32>, args: &[&str]) -> String {
    const USAGE: &str = "Usage: exrank [avatar_id|all] [on|off]";
    let status = |off: &Vec<u32>| {
        EX_RANKS
            .iter()
            .map(|(id, _, name)| {
                format!(
                    "{name} ({id}) {}",
                    if off.contains(id) { "off" } else { "on" }
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let (Some(target), Some(state)) = (args.first(), args.get(1)) else {
        return if args.is_empty() {
            format!("Ex rank: {}", status(off))
        } else {
            String::from(USAGE)
        };
    };
    let enable = match state.to_lowercase().as_str() {
        "on" | "1" | "true" => true,
        "off" | "0" | "false" => false,
        _ => return String::from(USAGE),
    };
    let targets: Vec<u32> = if target.eq_ignore_ascii_case("all") {
        EX_RANKS.iter().map(|(id, _, _)| *id).collect()
    } else {
        match target.parse::<u32>() {
            Ok(id) if EX_RANKS.iter().any(|(a, _, _)| *a == id) => vec![id],
            _ => {
                return format!(
                    "{target} has no ex rank. Characters with one: {}",
                    EX_RANKS
                        .iter()
                        .map(|(id, _, name)| format!("{name} ({id})"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    };

    for id in targets {
        off.retain(|v| *v != id);
        if !enable {
            off.push(id);
        }
    }
    format!("Ex rank: {} (applies from the next battle)", status(off))
}

/// HP for `dummy` enemies. The client accepts up to 4,294,967,295 (it's a 32-bit field).
const DUMMY_HP: u32 = 4_000_000_000;

/// `dummy` chat command: one wave of identical enemies with DUMMY_HP, for damage testing.
///   dummy                show the current setting
///   dummy 3              3 copies of the first enemy in freesr-data.json
///   dummy 5 8012210      5 copies of enemy 8012210
///   dummy off            back to the enemies from freesr-data.json (also clears the HP)
fn edit_dummy(o: &mut EnemyOverride, args: &[&str], configured: &[Vec<Monster>]) -> String {
    const USAGE: &str = "Usage: dummy 1-5 [monster_id], or dummy off";
    let configured_first = configured
        .iter()
        .flatten()
        .next()
        .map(|m| m.monster_id.to_string());

    match args.first().map(|v| v.to_lowercase()) {
        None => match o.count {
            Some(count) => format!(
                "Dummy: {count} x {} with {} HP",
                o.monster_id
                    .map(|id| id.to_string())
                    .or(configured_first)
                    .unwrap_or_default(),
                o.max_hp
                    .map_or(String::from("default"), |hp| with_commas(u64::from(hp)))
            ),
            None => format!("Dummy is off. {USAGE}"),
        },
        Some(arg) if arg == "off" => {
            o.count = None;
            o.monster_id = None;
            o.max_hp = None;
            String::from("Dummy off: enemies and HP from freesr-data.json again")
        }
        Some(arg) => {
            let Ok(count) = arg.parse::<u32>() else {
                return String::from(USAGE);
            };
            if !(1..=EnemyOverride::MAX_COUNT).contains(&count) {
                return String::from(USAGE);
            }
            let monster_id = match args.get(1) {
                None => None,
                Some(id) => match id.parse::<u32>() {
                    Ok(id) if id > 0 => Some(id),
                    _ => return format!("'{id}' is not a monster id. {USAGE}"),
                },
            };
            o.count = Some(count);
            o.monster_id = monster_id;
            o.max_hp = Some(DUMMY_HP);
            format!(
                "Dummy: {count} x {} with {} HP (applies from the next battle)",
                monster_id
                    .map(|id| id.to_string())
                    .or(configured_first)
                    .unwrap_or_else(|| String::from("the default enemy")),
                with_commas(u64::from(DUMMY_HP))
            )
        }
    }
}

/// 4000000000 -> "4,000,000,000"
fn with_commas(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `enemy` chat command. Takes effect on the next battle.
///   enemy                show current overrides
///   enemy hp 5000000     max HP for every enemy
///   enemy hppct 50       start enemies at 50% HP (needs a max HP)
///   enemy level 95       enemy level
///   enemy elite 3        EliteGroup id
///   enemy hardlevel 1    HardLevelGroup id
///   enemy hp off         clear one override (0 also works)
///   enemy reset          clear all overrides
fn edit_enemy_override(o: &mut EnemyOverride, args: &[&str]) -> String {
    let Some(key) = args.first().map(|k| k.to_lowercase()) else {
        return format!("Enemy: {o}");
    };

    let slot = match key.as_str() {
        "reset" => {
            *o = EnemyOverride::default();
            return String::from("Enemy overrides cleared");
        }
        "hp" => &mut o.max_hp,
        "hppct" => &mut o.hp_percent,
        "level" | "lv" => &mut o.level,
        "elite" => &mut o.elite_group,
        "hardlevel" => &mut o.hard_level_group,
        _ => {
            return String::from(
                "Usage: enemy [hp|hppct|level|elite|hardlevel] <number|off>, or enemy reset",
            );
        }
    };

    match args.get(1).map(|v| v.to_lowercase()) {
        None => return format!("Missing value. Example: enemy {key} <number>"),
        Some(v) if v == "off" || v == "0" => *slot = None,
        Some(v) => match v.parse::<u32>() {
            Ok(n) => *slot = Some(n),
            Err(_) => return format!("'{v}' is not a number"),
        },
    }

    format!("Enemy: {o} (applies from the next battle)")
}

fn parse_command(command: &str) -> Option<(&str, Vec<&str>)> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        return Option::None;
    }

    Some((parts[0], parts[1..].to_vec()))
}

/// Sends `text` to the player as a private message from the Server friend.
pub async fn send_server_message(session: &PlayerSession, text: String) -> Result<()> {
    const CHAT_TYPE_PRIVATE: i32 = 1;
    const MSG_TYPE_CUSTOM_TEXT: i32 = 1;

    session
        .send(create_send_message(
            25,
            SERVER_UID,
            MSG_TYPE_CUSTOM_TEXT,
            CHAT_TYPE_PRIVATE,
            text,
        ))
        .await
}

fn create_send_message(
    to: u32,
    from: u32,
    message_type: i32,
    chat_type: i32,
    msg: String,
) -> RevcMsgScNotify {
    RevcMsgScNotify {
        chat_type,
        pffpfkoglpo: to,
        recv_message_data: Some(ChatMessageData {
            create_time: cur_timestamp_ms(),
            ckhpffenobe: Some(Eknabklpeel {
                kpobmnlklok: 1,
                role_id: from,
            }),
            bkoalkhdlob: Some(Eknabklpeel {
                kpobmnlklok: 1,
                role_id: from,
            }),
            message_datas: vec![MessageChatData {
                message_type,
                chat_data: Some(ChatData {
                    extend_type: Some(chat_data::ExtendType::MessageText(msg)),
                }),
            }],
            ..Default::default()
        }),
    }
}
