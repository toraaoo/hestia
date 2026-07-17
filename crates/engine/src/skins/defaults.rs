//! The vanilla default skins (the nine 1.19.3+ characters, each in both model
//! variants). Nothing is bundled: Mojang serves every texture publicly at
//! `https://textures.minecraft.net/texture/<key>`, so the table is keys and
//! names only, and equipping one is a by-URL skin change.

use proto::skins::SkinVariant;

pub struct DefaultSkin {
    pub key: &'static str,
    pub name: &'static str,
    pub variant: SkinVariant,
}

pub fn texture_url(key: &str) -> String {
    format!("https://textures.minecraft.net/texture/{key}")
}

pub const DEFAULT_SKINS: &[DefaultSkin] = &[
    DefaultSkin {
        key: "46acd06e8483b176e8ea39fc12fe105eb3a2a4970f5100057e9d84d4b60bdfa7",
        name: "Alex",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "1abc803022d8300ab7578b189294cce39622d9a404cdc00d3feacfdf45be6981",
        name: "Alex",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "6ac6ca262d67bcfb3dbc924ba8215a18195497c780058a5749de674217721892",
        name: "Ari",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "4c05ab9e07b3505dc3ec11370c3bdce5570ad2fb2b562e9b9dd9cf271f81aa44",
        name: "Ari",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "fece7017b1bb13926d1158864b283b8b930271f80a90482f174cca6a17e88236",
        name: "Efe",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "daf3d88ccb38f11f74814e92053d92f7728ddb1a7955652a60e30cb27ae6659f",
        name: "Efe",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "226c617fde5b1ba569aa08bd2cb6fd84c93337532a872b3eb7bf66bdd5b395f8",
        name: "Kai",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "e5cdc3243b2153ab28a159861be643a4fc1e3c17d291cdd3e57a7f370ad676f3",
        name: "Kai",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "7cb3ba52ddd5cc82c0b050c3f920f87da36add80165846f479079663805433db",
        name: "Makena",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "dc0fcfaf2aa040a83dc0de4e56058d1bbb2ea40157501f3e7d15dc245e493095",
        name: "Makena",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "6c160fbd16adbc4bff2409e70180d911002aebcfa811eb6ec3d1040761aea6dd",
        name: "Noor",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "90e75cd429ba6331cd210b9bd19399527ee3bab467b5a9f61cb8a27b177f6789",
        name: "Noor",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "d5c4ee5ce20aed9e33e866c66caa37178606234b3721084bf01d13320fb2eb3f",
        name: "Steve",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "31f477eb1a7beee631c2ca64d06f8f68fa93a3386d04452ab27f43acdf1b60cb",
        name: "Steve",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "b66bc80f002b10371e2fa23de6f230dd5e2f3affc2e15786f65bc9be4c6eb71a",
        name: "Sunny",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "a3bd16079f764cd541e072e888fe43885e711f98658323db0f9a6045da91ee7a",
        name: "Sunny",
        variant: SkinVariant::Classic,
    },
    DefaultSkin {
        key: "eee522611005acf256dbd152e992c60c0bb7978cb0f3127807700e478ad97664",
        name: "Zuri",
        variant: SkinVariant::Slim,
    },
    DefaultSkin {
        key: "f5dddb41dcafef616e959c2817808e0be741c89ffbfed39134a13e75b811863d",
        name: "Zuri",
        variant: SkinVariant::Classic,
    },
];

pub fn find(key: &str) -> Option<&'static DefaultSkin> {
    DEFAULT_SKINS.iter().find(|d| d.key == key)
}
