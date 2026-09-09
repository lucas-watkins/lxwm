use crate::{Output, Seat, river};
use std::collections::{HashMap, VecDeque};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

// Version that we expect from River for the window manager protocol.
const RIVER_WINDOW_MANAGER_V1_VERSION: u32 = 4;

// Version that we expect from River for the xkb binding.
const RIVER_XKB_BINDINGS_V1_VERSION: u32 = 1;

// Represents the overall global state of the window manager. Only one of these
// objects should be created.
#[derive(Default)]
pub struct WindowManager {
    pub river_wm_global: Option<river::RiverWindowManagerV1>,
    pub river_xkb_global: Option<river::RiverXkbBindingV1>,
    pub windows: VecDeque<Window>,
    pub outputs: HashMap<river::ObjectId, Output>,
    pub seats: HashMap<river::ObjectId, Seat>,
}

// Represents a window in the window manager.
pub struct Window {
    pub proxy: river::RiverWindowV1,
    pub node: river::RiverNodeV1,
    pub new: bool,
    pub closed: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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
        use river::wl_registry::Event;

        if let Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                // Bind global window manager object
                "river_window_manager_v1" => {
                    if version < RIVER_WINDOW_MANAGER_V1_VERSION {
                        crate::exit!(
                            "Server river_window_manager_v1 version is {version} which is \
                            less than the required version ({RIVER_WINDOW_MANAGER_V1_VERSION})"
                        );
                    }

                    let river_wm_global =
                        proxy.bind(name, RIVER_WINDOW_MANAGER_V1_VERSION, qhandle, ());

                    state.river_wm_global = Some(river_wm_global);
                }

                // Bind global XKB object
                "river_xkb_bindings_v1" => {
                    if version < RIVER_XKB_BINDINGS_V1_VERSION {
                        crate::exit!(
                            "Server river_xkb_bindings_v1 version is {version} which is \
                            less than the required version ({RIVER_XKB_BINDINGS_V1_VERSION})"
                        );
                    }

                    let river_xkb_global =
                        proxy.bind(name, RIVER_XKB_BINDINGS_V1_VERSION, qhandle, ());

                    state.river_xkb_global = Some(river_xkb_global);
                }

                // Catch all as error
                ifname => {
                    crate::exit!("Unrecognized name: {ifname}");
                }
            }
        }
    }
}

// Handles RiverWindowV1 events for the WindowManager object
impl Dispatch<river::RiverWindowV1, ()> for WindowManager {
    fn event(
        state: &mut Self,
        proxy: &river::RiverWindowV1,
        event: <river::RiverWindowV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        use river::river_window_v1::Event;

        // Find the window that has the same proxy passed to us in the event handler.
        // (Find the window we're talking about in this context)
        let window = match state.windows.iter_mut().find(|x| &x.proxy == proxy) {
            Some(window) => window,
            None => return,
        };

        match event {
            Event::Closed => window.closed = true,

            Event::DimensionsHint {
                min_width,
                min_height,
                max_width,
                max_height,
            } => {}

            Event::Dimensions { width, height } => {
                window.width = width;
                window.height = height;
            }

            Event::AppId { app_id } => {}

            Event::Title { title } => {}

            Event::DecorationHint { hint } => {}

            Event::PointerMoveRequested { seat } => {
                window.pointer_move_requested = Some(seat);
            }

            Event::PointerResizeRequested { seat, edges } => {
                window.pointer_resize_requested = Some(seat);
                window.pointer_resize_requested_edges = edges.into_result().expect("Invalid Edges");
            }

            Event::ShowWindowMenuRequested { x, y } => {},

            Event::MaximizeRequested => {},

            Event::MinimizeRequested => {},

            Event::UnmaximizeRequested => {},

            Event::FullscreenRequested { output } => {},

            Event::ExitFullscreenRequested => {},

            Event::UnreliablePid { unreliable_pid } => {},

            Event::PresentationHint { hint } => {},

            Event::Identifier { identifier } => {},

            Event::Parent { parent } => {}
        }
    }
}
