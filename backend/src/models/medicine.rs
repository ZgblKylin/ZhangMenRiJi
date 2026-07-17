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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MedicineRate {
    Regular,
    Slow,
    VerySlow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PillRecipe {
    pub id: &'static str,
    pub medicine: Medicine,
    pub herb_cost: i32,
    pub quantity: i32,
    pub months: i32,
    pub rate: MedicineRate,
}

pub const PILL_RECIPES: [PillRecipe; 13] = [
    PillRecipe {
        id: "wound",
        medicine: Medicine::Wound,
        herb_cost: 4,
        quantity: 2,
        months: 1,
        rate: MedicineRate::Regular,
    },
    PillRecipe {
        id: "qi",
        medicine: Medicine::Qi,
        herb_cost: 6,
        quantity: 1,
        months: 2,
        rate: MedicineRate::Regular,
    },
    PillRecipe {
        id: "spirit",
        medicine: Medicine::Spirit,
        herb_cost: 5,
        quantity: 2,
        months: 1,
        rate: MedicineRate::Regular,
    },
    PillRecipe {
        id: "energy",
        medicine: Medicine::Energy,
        herb_cost: 6,
        quantity: 1,
        months: 2,
        rate: MedicineRate::Regular,
    },
    PillRecipe {
        id: "foundation",
        medicine: Medicine::Foundation,
        herb_cost: 12,
        quantity: 1,
        months: 6,
        rate: MedicineRate::Slow,
    },
    PillRecipe {
        id: "gather_qi",
        medicine: Medicine::GatherQi,
        herb_cost: 12,
        quantity: 1,
        months: 6,
        rate: MedicineRate::Slow,
    },
    PillRecipe {
        id: "calm_spirit",
        medicine: Medicine::CalmSpirit,
        herb_cost: 12,
        quantity: 1,
        months: 6,
        rate: MedicineRate::Slow,
    },
    PillRecipe {
        id: "restore_origin",
        medicine: Medicine::RestoreOrigin,
        herb_cost: 12,
        quantity: 1,
        months: 6,
        rate: MedicineRate::Slow,
    },
    PillRecipe {
        id: "marrow",
        medicine: Medicine::Marrow,
        herb_cost: 20,
        quantity: 1,
        months: 12,
        rate: MedicineRate::VerySlow,
    },
    PillRecipe {
        id: "sinew",
        medicine: Medicine::Sinew,
        herb_cost: 20,
        quantity: 1,
        months: 12,
        rate: MedicineRate::VerySlow,
    },
    PillRecipe {
        id: "awaken",
        medicine: Medicine::Awaken,
        herb_cost: 20,
        quantity: 1,
        months: 12,
        rate: MedicineRate::VerySlow,
    },
    PillRecipe {
        id: "lightness",
        medicine: Medicine::Lightness,
        herb_cost: 20,
        quantity: 1,
        months: 12,
        rate: MedicineRate::VerySlow,
    },
    PillRecipe {
        id: "longevity",
        medicine: Medicine::Longevity,
        herb_cost: 24,
        quantity: 1,
        months: 12,
        rate: MedicineRate::VerySlow,
    },
];

pub fn pill_recipe(id: &str) -> Option<PillRecipe> {
    PILL_RECIPES.iter().copied().find(|recipe| recipe.id == id)
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
