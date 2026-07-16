use std::fmt;

/// 丹房可炼制、可赐予弟子的全部丹药。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Medicine {
    Wound,
    Qi,
    Spirit,
    Energy,
    Foundation,
    GatherQi,
    CalmSpirit,
    RestoreOrigin,
    Marrow,
    Sinew,
    Awaken,
    Lightness,
    Longevity,
}

impl Medicine {
    pub const ALL: [Self; 13] = [
        Self::Wound,
        Self::Qi,
        Self::Spirit,
        Self::Energy,
        Self::Foundation,
        Self::GatherQi,
        Self::CalmSpirit,
        Self::RestoreOrigin,
        Self::Marrow,
        Self::Sinew,
        Self::Awaken,
        Self::Lightness,
        Self::Longevity,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Wound => "金疮药",
            Self::Qi => "养气丹",
            Self::Spirit => "清神散",
            Self::Energy => "回精丸",
            Self::Foundation => "培元丹",
            Self::GatherQi => "聚气丹",
            Self::CalmSpirit => "宁神丹",
            Self::RestoreOrigin => "回天丹",
            Self::Marrow => "洗髓丹",
            Self::Sinew => "强筋丹",
            Self::Awaken => "开窍丹",
            Self::Lightness => "轻身丹",
            Self::Longevity => "延寿丹",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|medicine| medicine.name() == name)
    }
}

impl fmt::Display for Medicine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// 早期存档曾使用的金疮药别名。
pub const LEGACY_WOUND_MEDICINE_NAME: &str = "金创药";
