//! The lore rules of the codex part: places and parent chains (spec §5.10), truths and clues
//! (§5.8), developer words in in-world text (§5.11) and the formalism mirror (§5.14).

mod floor_number;
mod formalism_edited;
mod formalism_missing;
mod formalism_source;
mod formalism_stray;
mod inworld_vocab;
mod never_confirm;
mod no_overshoot;
mod parent_cycle;
mod requires;
mod reveal_order;
mod three_clue;

pub(super) use floor_number::FLOOR_NUMBER;
pub(super) use formalism_edited::FORMALISM_EDITED;
pub(super) use formalism_missing::FORMALISM_MISSING;
pub(super) use formalism_source::FORMALISM_SOURCE;
pub(super) use formalism_stray::FORMALISM_STRAY;
pub(super) use inworld_vocab::INWORLD_VOCAB;
pub(super) use never_confirm::NEVER_CONFIRM;
pub(super) use no_overshoot::NO_OVERSHOOT;
pub(super) use parent_cycle::PARENT_CYCLE;
pub(super) use requires::REQUIRES;
pub(super) use reveal_order::REVEAL_ORDER;
pub(super) use three_clue::THREE_CLUE;
