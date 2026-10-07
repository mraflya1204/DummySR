use std::collections::{BTreeMap, HashMap};

use proto::SceneMonsterWave;
use serde::{Deserialize, Serialize};

use super::{monster::Monster, relic::SubAffix};

// PEARL
/// How Pearl's technique buff is attached at battle start. Her technique marks the active
/// character ("Aesthetic Archetype"), who gets Deep Learning while Pearl gets 20 Certified Banger;
/// it isn't known which layout the client expects, so it can be switched in-game with `pearl`.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PearlMode {
    /// Owned by Pearl and aimed at the active character (`target_index_list`).
    #[default]
    Target,
    /// Owned by the active character.
    Active,
    /// Owned by Pearl, plus buff 1000121 on the active character: the same marker the server
    /// uses for Dahlia's dance partner.
    Marker,
    /// Owned by Pearl, nothing else (what upstream RobinSR did).
    #[serde(rename = "self")]
    Pearl,
}

impl PearlMode {
    pub const ALL: [PearlMode; 4] = [Self::Target, Self::Active, Self::Marker, Self::Pearl];

    pub fn name(self) -> &'static str {
        match self {
            Self::Target => "target",
            Self::Active => "active",
            Self::Marker => "marker",
            Self::Pearl => "self",
        }
    }
}

// ENEMY OVERRIDES
/// Enemy stat overrides, set with the `enemy` chat command and stored in `persistent`
/// (not in freesr-data.json, so saving from SRTools doesn't wipe them).
/// `None` means "leave whatever SRTools / the stage config says".
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EnemyOverride {
    /// Max HP for every enemy, as an absolute number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<u32>,
    /// Starting HP as a percentage of max HP. Only applied when a max HP is known
    /// (from `max_hp` above or from SRTools' per-monster HP).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hp_percent: Option<u32>,
    /// Enemy level for every wave.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    /// Row id in the game's EliteGroup table (ATK/DEF/HP/SPD ratios).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elite_group: Option<u32>,
    /// Row id in the game's HardLevelGroup table (how stats scale with level).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard_level_group: Option<u32>,
}

impl EnemyOverride {
    pub fn apply(&self, waves: &mut [SceneMonsterWave]) {
        for wave in waves.iter_mut() {
            let param = wave.monster_param.get_or_insert_with(Default::default);
            if let Some(level) = self.level {
                param.level = level;
            }
            if let Some(group) = self.elite_group {
                param.elite_group = group;
            }
            if let Some(group) = self.hard_level_group {
                param.hard_level_group = group;
            }

            for monster in wave.monster_list.iter_mut() {
                if let Some(hp) = self.max_hp {
                    monster.max_hp = hp;
                    monster.cur_hp = hp;
                }
                if let Some(pct) = self.hp_percent {
                    if monster.max_hp > 0 {
                        let cur = monster.max_hp as u64 * pct.clamp(1, 100) as u64 / 100;
                        monster.cur_hp = cur.max(1) as u32;
                    }
                }
            }
        }
    }
}

impl std::fmt::Display for EnemyOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let show = |v: Option<u32>| v.map_or(String::from("default"), |v| v.to_string());
        write!(
            f,
            "hp={} hppct={} level={} elite={} hardlevel={}",
            show(self.max_hp),
            show(self.hp_percent),
            show(self.level),
            show(self.elite_group),
            show(self.hard_level_group)
        )
    }
}

// BATTLE CONFIG
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BattleConfig {
    pub battle_type: BattleType,
    pub monsters: Vec<Vec<Monster>>,
    pub blessings: Vec<BattleBuffJson>,
    pub stage_id: u32,
    pub cycle_count: u32,
    pub path_resonance_id: u32,
    pub custom_stats: Vec<SubAffix>,
    #[serde(default)]
    pub scepters: Vec<RogueMagicScepter>,
    #[serde(default)]
    pub custom_battle_lineup: Option<BTreeMap<u32, u32>>,
}

impl Default for BattleConfig {
    fn default() -> Self {
        Self {
            battle_type: Default::default(),
            monsters: vec![vec![Monster {
                level: 60,
                monster_id: 3014022,
                max_hp: 0,
            }]],
            stage_id: 201012311,
            blessings: Default::default(),
            cycle_count: Default::default(),
            path_resonance_id: Default::default(),
            custom_stats: Default::default(),
            scepters: Default::default(),
            custom_battle_lineup: Default::default(),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum BattleType {
    #[serde(alias = "DEFAULT")]
    #[default]
    Default = 0,
    #[serde(alias = "MOC")]
    Moc = 1,
    PF = 2,
    SU = 3,
    AS = 4,
    AA = 5,
}

// BATTLE BUFFS
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BattleBuffJson {
    pub level: u32,
    pub id: u32,
    pub dynamic_key: Option<DynamicKey>,
    #[serde(default)]
    pub dynamic_values: Vec<DynamicKey>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DynamicKey {
    pub key: String,
    pub value: u32,
}

#[allow(dead_code)]
impl BattleBuffJson {
    pub fn to_battle_buff_proto(&self) -> proto::BattleBuff {
        proto::BattleBuff {
            id: self.id,
            level: self.level,
            wave_flag: 0xffffffff,
            owner_index: 0xffffffff,
            dynamic_values: if let Some(dyn_key) = &self.dynamic_key {
                HashMap::from([(dyn_key.key.clone(), dyn_key.value as f32)])
            } else {
                Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Default)]
#[repr(u32)]
pub enum RogueMagicComponentType {
    Passive = 3,
    #[default]
    Active = 4,
    Attach = 5,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RogueMagicScepter {
    pub level: u32,
    pub id: u32,
    pub components: Vec<RogueMagicComponent>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RogueMagicComponent {
    pub id: u32,
    pub level: u32,
    pub component_type: RogueMagicComponentType,
}
