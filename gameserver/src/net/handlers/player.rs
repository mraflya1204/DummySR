use std::sync::LazyLock;

use super::*;

// Bottom-left corner text, set by the Lua the client runs on every heartbeat.
// Unity rich text: `\n` starts a new line.

/// Upper text (upstream showed t.me/neonteam25 here). Empty hides it. Its box only fits one line
/// at a large size, so put a second line in VERSION_TEXT instead of using `\n`.
const WATERMARK_TEXT: &str = "DummySR";
/// Lower line (upstream showed "RobinSR! | " plus the game version here). Empty hides it.
const VERSION_TEXT: &str = "Forked from RobinSR by NeonTeam";
/// true puts the game's version string after VERSION_TEXT, like upstream RobinSR.
const SHOW_GAME_VERSION: bool = false;
/// Color for both lines: "#RRGGBB" or "#RRGGBBAA". "" keeps the game's color.
const TEXT_COLOR: &str = "#00e1ff";
/// Font sizes. The game's default here is roughly 16; 0 keeps it. Too large and the text box
/// cuts the line off.
const WATERMARK_FONT_SIZE: u32 = 28;
const VERSION_FONT_SIZE: u32 = 0;

/// Precompiled Luau chunk the client runs on every heartbeat: sets the frame-rate target and
/// rewrites the two corner texts. `build_heartbeat_payload` swaps its strings for the settings
/// above when the server starts; don't edit the base64 by hand.
const HEARTBEAT_LUA: &str = "BAEcA2xvZwJDUwtVbml0eUVuZ2luZQtBcHBsaWNhdGlvbg90YXJnZXRGcmFtZVJhdGUPUXVhbGl0eVNldHRpbmdzCnZTeW5jQ291bnQIcGF0Y2hGcHMKR2FtZU9iamVjdARGaW5kKFVJUm9vdC9BYm92ZURpYWxvZy9CZXRhSGludERpYWxvZyhDbG9uZSkDUlBHBkNsaWVudA1Mb2NhbGl6ZWRUZXh0BnR5cGVvZhZHZXRDb21wb25lbnRJbkNoaWxkcmVuJjxjb2xvcj0jMDBlMWZmPnQubWUvbmVvbnRlYW0yNTwvY29sb3I+BHRleHQLVmVyc2lvblRleHQiPGNvbG9yPSMwMGUxZmY+Um9iaW5TUiEgfCA8L2NvbG9yPg88Y29sb3I9IzAwZTFmZj4KR2xvYmFsVmFycw1zX1ZlcnNpb25EYXRhF0dldFNlcnZlclBha1R5cGVWZXJzaW9uCDwvY29sb3I+C3BhdGNoQmV0YVdtBG1haW4GeHBjYWxsBQEBAAAAAAEWAAEAAAAEAQEYAAUAAAAAAgAAAAAACwwAAwACBADABAFoARABAAMEAAAADAAGAAUEAMAEAQAAEAEA0AcAAAAWAAEACAMCAwMDBAQCBADAAwUDBgQFBADAAwcABwgBGAAAAAAAAQAAAAABCAAAAAAJAAAAAAA2DAEDAAIEAMAPAAF2BAAAAAUBBQAVAAICDAQIAAcYAMAPAwQUCQAAAE0sAwIMAgsAAACgQBUCAgAUAACxDAAAABUAAAIFAQ0AEAEA1Q4AAAAMAQMAAgQAwA8AAXYEAAAABQEPABUAAgIMBAgABxgAwA8DBBQJAAAATSwDAgwCCwAAAKBAFQICABQAALEMAAAAFQAAAgUCEAAFAxEADAgIAAcYAMAPBwisEgAAAA8GByMTAAAAFAYGkRQAAAAVBgICBgQGAAUFFQA1AQIFEAEA1Q4AAAAWAAEAFgMCAwMDCQQCBADAAwoDCwMMAw0EBxgAwAMOAw8EAACgQAMQAxEDEgMTAxQDFQMWAxcDGAMZAAwaARgAAAAAAAAAAAAAAAAAAAAAAAAAAAMAAAAAAAAAAAAAAAAAAAAAAQEBAAAAAAAAAAAAAQD8AAUOAAAAAAEAAQAAAAMJAAAAFQABARYAAQAAABkbARgAAAEaAAAAAAcAAAEAAAxFAAAARAAAAEQBAQBEAgIARAMDAEoAAgAMBAUAAABAQAYFAwAGBgAAFQQDARYAAQAGBgAGAQYCBgMDHAQAAEBABAABAgMBAAEYAAMDBQ0ABAAAAAAAAQAAAAAE";

static HEARTBEAT_PAYLOAD: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let original = rbase64::decode(HEARTBEAT_LUA).expect("HEARTBEAT_LUA is not valid base64");
    let settings = CornerText {
        watermark: WATERMARK_TEXT,
        version: VERSION_TEXT,
        show_game_version: SHOW_GAME_VERSION,
        color: TEXT_COLOR,
        watermark_size: WATERMARK_FONT_SIZE,
        version_size: VERSION_FONT_SIZE,
    };

    match build_heartbeat_payload(&original, &settings) {
        Ok(patched) => patched,
        Err(err) => {
            tracing::error!("failed to patch heartbeat lua, sending it unchanged: {err}");
            original
        }
    }
});

struct CornerText<'a> {
    watermark: &'a str,
    version: &'a str,
    show_game_version: bool,
    color: &'a str,
    watermark_size: u32,
    version_size: u32,
}

impl CornerText<'_> {
    fn open_tags(&self, size: u32) -> String {
        let mut tags = String::new();
        if size > 0 {
            tags += &format!("<size={size}>");
        }
        if !self.color.is_empty() {
            tags += &format!("<color={}>", self.color);
        }
        tags
    }

    fn close_tags(&self, size: u32) -> String {
        let mut tags = String::new();
        if !self.color.is_empty() {
            tags += "</color>";
        }
        if size > 0 {
            tags += "</size>";
        }
        tags
    }

    fn styled(&self, text: &str, size: u32) -> String {
        if text.is_empty() {
            String::new()
        } else {
            format!("{}{text}{}", self.open_tags(size), self.close_tags(size))
        }
    }
}

// Last instructions of patchBetaWm in HEARTBEAT_LUA, as this client build encodes them. The
// version line is set with `VersionText.text = R2 .. R3 .. R4 .. R5`, where
// R2 = "<color=#00e1ff>RobinSR! | </color>", R3 = "<color=#00e1ff>",
// R4 = GetServerPakTypeVersion() and R5 = "</color>".
const LOADK_R5_CLOSE_TAG: u32 = 0x0015_0505; // R5 = K21 ("</color>")
const CONCAT_R1_R2_TO_R5: u32 = 0x0502_0135; // R1 = R2 .. R3 .. R4 .. R5
const SETTABLEKS_R1_TEXT: u32 = 0xd500_0110; // R0.text = R1, key in the next word
const AUX_KEY_TEXT: u32 = 0x0000_000e; // K14 ("text")
const MOVE_R1_R2: u32 = 0x0002_0106; // R1 = R2

fn build_heartbeat_payload(original: &[u8], text: &CornerText) -> anyhow::Result<Vec<u8>> {
    let watermark = text.styled(text.watermark, text.watermark_size);

    if text.show_game_version {
        // Keep the concatenation: [tags + VERSION_TEXT] .. "" .. version .. [closing tags]
        let prefix = format!("{}{}", text.open_tags(text.version_size), text.version);
        let close = text.close_tags(text.version_size);
        return crate::luau::replace_strings(
            original,
            &[
                ("<color=#00e1ff>t.me/neonteam25</color>", watermark.as_str()),
                ("<color=#00e1ff>RobinSR! | </color>", prefix.as_str()),
                ("<color=#00e1ff>", ""),
                ("</color>", close.as_str()),
            ],
        );
    }

    // Without the version, the line is exactly the styled VERSION_TEXT: R2 holds it and the
    // concatenation becomes a plain copy of R2.
    let version_line = text.styled(text.version, text.version_size);
    let patched = crate::luau::replace_strings(
        original,
        &[
            ("<color=#00e1ff>t.me/neonteam25</color>", watermark.as_str()),
            ("<color=#00e1ff>RobinSR! | </color>", version_line.as_str()),
        ],
    )?;

    crate::luau::replace_code(
        &patched,
        &[
            LOADK_R5_CLOSE_TAG,
            CONCAT_R1_R2_TO_R5,
            SETTABLEKS_R1_TEXT,
            AUX_KEY_TEXT,
        ],
        &[
            LOADK_R5_CLOSE_TAG,
            MOVE_R1_R2,
            SETTABLEKS_R1_TEXT,
            AUX_KEY_TEXT,
        ],
    )
}

pub async fn on_get_basic_info_cs_req(
    _session: &mut PlayerSession,
    _body: &GetBasicInfoCsReq,
    res: &mut GetBasicInfoScRsp,
) {
    res.player_setting_info = Some(PlayerSettingInfo::default());
    res.gender = Gender::Woman as u32;
    res.is_gender_set = true;
}

pub async fn on_player_heart_beat_cs_req(
    session: &mut PlayerSession,
    body: &PlayerHeartBeatCsReq,
    res: &mut PlayerHeartBeatScRsp,
) {
    for text in std::mem::take(&mut session.pending_server_messages) {
        if let Err(err) = send_server_message(session, text).await {
            tracing::warn!("failed to send Server message: {err}");
        }
    }

    res.client_time_ms = body.client_time_ms;
    res.server_time_ms = body.client_time_ms;
    res.download_data = Some(ClientDownloadData {
        version: 51,
        time: res.server_time_ms as i64,
        data: HEARTBEAT_PAYLOAD.clone(),
        ..Default::default()
    });
}

pub async fn on_player_login_finish_cs_req(
    session: &mut PlayerSession,
    _req: &PlayerLoginFinishCsReq,
    _res: &mut PlayerLoginFinishScRsp,
) -> Result<()> {
    session
        .send(ContentPackageSyncDataScNotify {
            data: Some(ContentPackageData {
                content_package_list: [
                    200001, 200002, 200003, 200004, 200005, 200006, 200007, 200008, 200009, 200010,
                    200011, 200012, 150017, 150015, 150021, 150018, 130011, 130012, 130013, 150025,
                    140006, 150026, 130014, 150034, 150029, 150035, 150041, 150039, 150045, 150057,
                    150042, 150067, 150064, 150063, 150024, 171002, 150068, 150070, 150071, 150073,
                    150074, 150075, 150076, 150077, 150078, 150079,
                ]
                .into_iter()
                .map(|v| ContentPackageInfo {
                    status: ContentPackageStatus::Finished.into(),
                    content_id: v,
                })
                .collect(),
                ..Default::default()
            }),
        })
        .await?;

    Ok(())
}
