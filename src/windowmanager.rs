use crate::{Output, Seat, river};
use std::collections::{HashMap, VecDeque};
use wayland_client::{
    Connection, QueueHandle, protocol::wl_registry::WlRegistry, Dispatch, Proxy
};

#[derive(Default)]
pub struct WindowManager {
    pub river_wm_global: Option<river::RiverWindowManagerV1>,
    pub river_xkb_global: Option<river::RiverXkbBindingV1>,
    pub windows: VecDeque<Window>,
    pub outputs: HashMap<river::ObjectId, Output>,
    pub seats: HashMap<river::ObjectId, Seat>,
}

pub struct Window {
    pub proxy: river::RiverWindowV1,
    pub node: river::RiverNodeV1,
    pub new: bool,
    pub closed: bool,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub pointer_move_requested: Option<river::RiverSeatV1>,
    pub pointer_resize_requested: Option<river::RiverSeatV1>,
    pub pointer_resize_requested_edges: river::Edges,
}

// This implementation helps the WindowManager singleton declared in main to
// bind to River globals.
impl Dispatch<WlRegistry, ()> for WindowManager {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        todo!()
    }
}
