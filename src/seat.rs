use std::collections::HashMap;
use crate::{river, PointerBinding, Action};

pub struct Seat {
    pub proxy: river::RiverSeatV1,
    pub new: bool,
    pub removed: bool,
    pub focused: Option<river::RiverWindowV1>,
    pub hovered: Option<river::RiverWindowV1>,
    pub interacted: Option<river::RiverWindowV1>,
    pub xkb_bindings: HashMap<river::ObjectId, PointerBinding>,
    pub pending_action: Action,
    pub op: SeatOp,
    pub op_dx: i32,
    pub op_dy: u32,
    pub op_release: bool,
}

pub enum SeatOp {
    None,
    Move {
        window_proxy: river::RiverWindowV1,
        start_x: u32,
        start_y: u32,
    },
    Resize {
        window_proxy: river::RiverWindowV1,
        start_x: u32,
        start_y: u32,
        start_width: u32,
        start_height: u32,
        edges: river::Edges,
    },
}
