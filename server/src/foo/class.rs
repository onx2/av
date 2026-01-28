// use spacetimedb::{ReducerContext, SpacetimeType, table};

// #[table(name=class_tbl)]
// pub struct Class {
//     #[auto_inc]
//     #[primary_key]
//     pub id: u8,

//     name: ClassName,
//     description: String,
// }

// #[table(name=class_specialization_tbl)]
// pub struct ClassSpecialization {
//     #[auto_inc]
//     #[primary_key]
//     pub id: u8,

//     #[index(btree)]
//     class_id: u8,

//     name: ClassSpecializationName,

//     description: String,
// }

// #[repr(u8)]
// #[derive(SpacetimeType, Copy, Clone, Debug, PartialEq, Eq)]
// pub enum ClassName {
//     Arcanist,
//     Stalker,
//     Shaman,
//     Occulist,
//     Myrmidon,
//     Templar,
// }

// #[repr(u8)]
// #[derive(SpacetimeType, Copy, Clone, Debug, PartialEq, Eq)]
// pub enum ClassSpecializationName {
//     Guardian,
//     Judge,
//     Preacher,

//     ShadowWalker,
//     Sharpshooter,
//     BladeDancer,

//     Spiritmender,
//     Earthshaper,
//     Stormcaller,

//     Emberborn,
//     Corruptor,
//     Plaguespreader,

//     Legionnaire,
//     Centurion,
//     Vanguard,

//     Pyromancer,
//     Cryomancer,
//     ArcaneWeaver,
// }

// impl Class {
//     pub fn regenerate(ctx: &ReducerContext) {

//     }
// }
// // impl Class {
// //     pub fn description(&self) -> &'static str {
// //         match self {
// //             Class::Arcanist => "The Arcanist is a master of arcane magic, harnessing the immense power drawn from the very fabric of the world itself. With unparalleled control over elemental forces, they unleash devastating spells, shape the environment to their will, and manipulate the arcane energies that flow through the world of Aelynmar. Whether raining down torrents of fire, freezing enemies with ice, or summoning storms of destruction, the Arcanist is a force to be reckoned with, controlling the battlefield with their vast array of elemental powers.",
// //             Class::Shaman => "The Shaman is a spiritual leader who communes with the natural world, drawing upon the power of spirits and the land to protect allies, weaken enemies, and alter the flow of battle. Through their deep connection with the earth, animals, and the spiritual realm, Shamans can heal the wounded, buff their allies, and debuff their foes. They wield runes and totems, casting nature-infused magic that can turn the tide of battle in an instant. Shamans can call upon animal spirits to fight alongside them, summon the elements, and place powerful totems that influence the battlefield. Their unique blend of healing, support, and control makes them invaluable members of any adventuring group.",
// //             Class::Stalker => "The Stalker is a versatile and agile fighter who strikes swiftly and silently. Blending ranged attacks with rapid melee strikes, Stalkers excel at taking down their foes before they can react. Masters of stealth, they use the environment to their advantage, disappearing into the shadows and ambushing unsuspecting targets. Whether using bows for precise shots or daggers and throwing knives for close-range combat, Stalkers are unpredictable and deadly.",
// //             Class::Occulist => "The Occultist is a dark spellcaster who delves into the forbidden and often dangerous aspects of magic. With a mastery over shadowy forces, curses, and summoning unholy creatures, they wield corrupted power to weaken, manipulate, and destroy their enemies from afar. Whether summoning demonic entities to fight on their behalf or casting debilitating curses to drain the life from foes, the Occultist thrives in sowing chaos and fear, weakening their enemies before delivering a final, devastating blow of dark magic.",
// //             Class::Myrmidon => "A disciplined and versatile warrior who excels at both offense and defense. They are masters of tactical combat, using their skills to disrupt enemy formations, control the battlefield, and deliver precise, powerful strikes. They can specialize in different weapon styles and combat stances.",
// //             Class::Templar => "The Templar is a holy warrior, a paragon of divine power and martial skill. Channeling the light of the divine, they strike down enemies with righteous fury while shielding their allies from harm. Whether engaging in brutal combat, protecting their comrades, or lifting their spirits with divine blessings, the Templar stands as an unwavering force in the face of darkness.",
// //         }
// //     }

// //     pub fn available_specializations(&self) -> &'static [Specialization] {
// //         match self {
// //             Class::Arcanist => &[
// //                 Specialization::Pyromancer,
// //                 Specialization::Cryomancer,
// //                 Specialization::ArcaneWeaver,
// //             ],
// //             Class::Shaman => &[
// //                 Specialization::Spiritmender,
// //                 Specialization::Earthshaper,
// //                 Specialization::Stormcaller,
// //             ],
// //             Class::Stalker => &[
// //                 Specialization::ShadowWalker,
// //                 Specialization::Sharpshooter,
// //                 Specialization::BladeDancer,
// //             ],
// //             Class::Occulist => &[
// //                 Specialization::Emberborn,
// //                 Specialization::Corruptor,
// //                 Specialization::Plaguespreader,
// //             ],
// //             Class::Myrmidon => &[
// //                 Specialization::Legionnaire,
// //                 Specialization::Centurion,
// //                 Specialization::Vanguard,
// //             ],
// //             Class::Templar => &[
// //                 Specialization::Guardian,
// //                 Specialization::Judge,
// //                 Specialization::Preacher,
// //             ],
// //         }
// //     }
// // }
