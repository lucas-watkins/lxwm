use std::collections::HashMap;
use crate::{river, PointerBinding, Action, XkbBinding};
use crate::river::RiverSeatV1;

pub struct Seat {
    pub proxy: river::RiverSeatV1,
    pub new: bool,
    pub removed: bool,
    pub focused: Option<river::RiverWindowV1>,
    pub hovered: Option<river::RiverWindowV1>,
    pub interacted: Option<river::RiverWindowV1>,
    pub xkb_bindings: HashMap<river::ObjectId, XkbBinding>,
    pub pointer_bindings: HashMap<river::ObjectId, PointerBinding>,
    pub pending_action: Action,
    pub op: SeatOp,
    pub op_dx: i32,
    pub op_dy: i32,
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

impl Seat {
    pub fn new(proxy: RiverSeatV1) -> Self {
        Self {
            proxy,
            new: true,
            removed: false,
            focused: None,
            hovered: None,
            interacted: None,
            xkb_bindings: HashMap::new(),
            pointer_bindings: HashMap::new(),
            pending_action: Action::None,
            op: SeatOp::None,
            op_dx: 0,
            op_dy: 0,
            op_release: false,
        }
    }
}
