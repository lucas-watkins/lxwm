use crate::river::{
    river_node_v1::RiverNodeV1,
    river_output_v1::RiverOutputV1,
    river_pointer_binding_v1::RiverPointerBindingV1,
    river_seat_v1::{Modifiers, RiverSeatV1},
    river_window_manager_v1::RiverWindowManagerV1,
    river_window_v1::{Edges, RiverWindowV1},
    river_xkb_binding_v1::RiverXkbBindingV1,
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    None,
    SpawnFoot,
    Close,
    FocusNext,
    Move,
    Resize,
    Exit,
}

#[derive(Debug)]
pub struct XkbBinding {
    pub proxy: RiverXkbBindingV1,
    pub action: Action,
}

#[derive(Debug)]
pub struct PointerBinding {
    pub proxy: RiverPointerBindingV1,
    pub action: Action,
}

#[derive(Debug)]
pub struct Output {
    pub proxy: RiverOutputV1,
    pub removed: bool,
}

impl Output {
    pub fn new(proxy: RiverOutputV1) -> Self {
        Self {
            proxy,
            removed: false,
        }
    }
}
