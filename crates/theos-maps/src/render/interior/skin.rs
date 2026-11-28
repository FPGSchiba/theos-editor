//! The colours of the interior renders: `base`'s ground, the blueprint and ruin themes, and the
//! role colours of the graph. Call sites name a colour; no RGB value appears outside this file.

use crate::render::palette::Rgb;
use crate::rooms::Role;

/// The colours of the cells.
#[derive(Debug, Clone, Copy)]
pub(super) struct Ground {
    /// Outside the envelope; `None` leaves the pixel transparent.
    pub outside: Option<Rgb>,
    /// Inside the envelope but neither floor nor wall.
    pub solid: Rgb,
    /// Walkable floor.
    pub floor: Rgb,
    /// Built wall cells.
    pub wall: Rgb,
    /// Cover on the floor.
    pub cover: Rgb,
}

/// The colours of the drawn symbols and texts.
#[derive(Debug, Clone, Copy)]
pub(super) struct Ink {
    /// Room labels.
    pub label: Rgb,
    /// Open doors.
    pub door: Rgb,
    /// Locked doors and their key names.
    pub locked: Rgb,
    /// Drops.
    pub drop: Rgb,
    /// The critical route.
    pub route: Rgb,
    /// Key marks.
    pub key: Rgb,
    /// Exit rings.
    pub exit: Rgb,
}

/// A theme of `blueprint`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Theme {
    /// The cells.
    pub ground: Ground,
    /// The symbols.
    pub ink: Ink,
}

/// `base`: light floor, dark walls, a mid tone for cover, transparent outside.
pub(super) const BASE: Ground = Ground {
    outside: None,
    solid: [0x24, 0x22, 0x20],
    floor: [0xdc, 0xd9, 0xd2],
    wall: [0x24, 0x22, 0x20],
    cover: [0x9a, 0x96, 0x8c],
};

/// `blueprint`: white walls on blueprint blue, the floor a little lighter than the background.
pub(super) const BLUEPRINT: Theme = Theme {
    ground: Ground {
        outside: Some([0x1b, 0x44, 0x7c]),
        solid: [0x17, 0x3a, 0x6a],
        floor: [0x2c, 0x5d, 0x99],
        wall: [0xf2, 0xf6, 0xfb],
        cover: [0x7f, 0xa6, 0xd2],
    },
    ink: Ink {
        label: [0xdb, 0xe7, 0xf5],
        door: [0xf2, 0xf6, 0xfb],
        locked: [0xe5, 0x48, 0x3c],
        drop: [0x7f, 0xd4, 0xf2],
        route: [0x8f, 0xe3, 0xb0],
        key: [0xf6, 0xc3, 0x45],
        exit: [0xff, 0x9f, 0x4a],
    },
};

/// `blueprint-ruin`: the same drawing in worn sepia tones.
pub(super) const RUIN: Theme = Theme {
    ground: Ground {
        outside: Some([0x3b, 0x2f, 0x22]),
        solid: [0x2e, 0x24, 0x1a],
        floor: [0x6e, 0x5a, 0x41],
        wall: [0xd9, 0xc6, 0xa0],
        cover: [0x9a, 0x82, 0x5e],
    },
    ink: Ink {
        label: [0xea, 0xdc, 0xbc],
        door: [0xd9, 0xc6, 0xa0],
        locked: [0xc4, 0x4a, 0x32],
        drop: [0xa8, 0xb0, 0x78],
        route: [0xc9, 0xb4, 0x6a],
        key: [0xe0, 0xa8, 0x3c],
        exit: [0xd2, 0x7a, 0x3a],
    },
};

/// The background of `graph`.
pub(super) const GRAPH_BACKGROUND: Rgb = [0x1b, 0x44, 0x7c];
/// The edge of a node of `graph`.
pub(super) const NODE_EDGE: Rgb = [0xf2, 0xf6, 0xfb];

/// The colour of a node of `graph` by role.
pub(super) fn role_colour(role: Role) -> Rgb {
    match role {
        Role::Start => [0x6f, 0xcf, 0x7f],
        Role::Goal => [0xf6, 0xc3, 0x45],
        Role::Path => [0x7f, 0xa6, 0xd2],
        Role::Leaf => [0x9a, 0xa5, 0xb5],
        Role::Exit => [0xff, 0x9f, 0x4a],
    }
}

/// The name of a role in a legend.
pub(super) fn role_name(role: Role) -> &'static str {
    match role {
        Role::Start => "start",
        Role::Goal => "goal",
        Role::Path => "path",
        Role::Leaf => "leaf",
        Role::Exit => "exit",
    }
}

/// Every role, in the order a legend lists them.
pub(super) const ROLES: [Role; 5] = [Role::Start, Role::Goal, Role::Path, Role::Leaf, Role::Exit];
