use crate::river::river_seat_v1::Modifiers;
use crate::river::river_window_manager_v1::Event;
use crate::river::{
    Edges, ObjectId, RiverNodeV1, RiverOutputV1, RiverPointerBindingV1, RiverSeatV1,
    RiverWindowManagerV1, RiverWindowV1, RiverXkbBindingV1, RiverXkbBindingsV1,
};
use crate::{Action, Output, Seat, SeatOp, river};
use bitflags::Flags;
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
    pub river_xkb_global: Option<river::RiverXkbBindingsV1>,
    pub wm_data: WMData,
}

#[derive(Default)]
pub struct WMData {
    pub windows: VecDeque<Window>,
    pub outputs: HashMap<ObjectId, Output>,
    pub seats: HashMap<ObjectId, Seat>,
}

impl WMData {
    fn handle_manage_start(
        &mut self,
        proxy: &RiverWindowManagerV1,
        river_xkb: &RiverXkbBindingsV1,
        qhandle: &QueueHandle<WindowManager>,
    ) {
        self.remove_outputs();
        self.remove_windows();
        self.remove_seats();
        self.init_new_windows();
        self.init_new_seats(river_xkb, qhandle);
        self.manage_windows();
        self.manage_seats(proxy);

        proxy.manage_finish();
    }

    fn handle_render_start(&mut self, proxy: &RiverWindowManagerV1) {
        for seat in &mut self.seats.values_mut() {
            match &seat.op {
                SeatOp::None => {}

                SeatOp::Move {
                    window_proxy,
                    start_x,
                    start_y,
                } => {
                    if let Some(window) = self.windows.iter_mut().find(|w| &w.proxy == window_proxy)
                    {
                        window.set_position(start_x + seat.op_dx, start_y + seat.op_dy);
                    }
                }

                SeatOp::Resize {
                    window_proxy,
                    start_x,
                    start_y,
                    start_width,
                    start_height,
                    edges,
                } => {
                    if let Some(window) = self.windows.iter_mut().find(|w| &w.proxy == window_proxy)
                    {
                        let (mut x, mut y) = (*start_x, *start_y);

                        if edges.contains(Edges::Left) {
                            x += start_width - window.width;
                        }

                        if edges.contains(Edges::Top) {
                            y += start_height - window.height;
                        }

                        window.set_position(x, y);
                    }
                }
            }
        }
    }

    fn remove_outputs(&mut self) {
        self.outputs.retain(|_, output| {
            if output.removed {
                output.proxy.destroy();
                return false;
            }

            true
        });
    }

    fn remove_windows(&mut self) {
        let old_windows = std::mem::take(&mut self.windows);

        self.windows = old_windows
            .into_iter()
            .filter(|window| {
                if window.closed {
                    for seat in self.seats.values_mut() {
                        if let SeatOp::Move { window_proxy, .. }
                        | SeatOp::Resize { window_proxy, .. } = &seat.op
                        {
                            if window_proxy == &window.proxy {
                                seat.op_end();
                            }
                        }
                    }
                    return false;
                }
                true
            })
            .collect();
    }

    fn init_new_windows(&mut self) {
        for window in self.windows.iter_mut().filter(|w| w.new) {
            window.set_position(window.x, window.y);
            window.proxy.propose_dimensions(window.width, window.height);
            window.new = false;
        }
    }

    fn remove_seats(&mut self) {
        self.seats.retain(|_, seat| {
            if seat.removed {
                seat.xkb_bindings
                    .values_mut()
                    .for_each(|binding| binding.proxy.destroy());

                seat.pointer_bindings
                    .values_mut()
                    .for_each(|binding| binding.proxy.destroy());

                return false;
            }

            true
        });
    }

    fn init_new_seats(
        &mut self,
        river_xkb: &RiverXkbBindingsV1,
        qhandle: &QueueHandle<WindowManager>,
    ) {
        const SPACE: u32 = 0x20;
        const N: u32 = 0x6e;
        const Q: u32 = 0x71;
        const ESC: u32 = 0xff1b;
        const BTN_LEFT: u32 = 0x110;
        const BTN_RIGHT: u32 = 0x111;

        let mods = Modifiers::Mod4;

        for seat in self.seats.values_mut() {
            if seat.new {
                seat.create_xkb_binding(river_xkb, qhandle, mods, SPACE, Action::SpawnFoot);
                seat.create_xkb_binding(river_xkb, qhandle, mods, Q, Action::Close);
                seat.create_xkb_binding(river_xkb, qhandle, mods, N, Action::FocusNext);
                seat.create_xkb_binding(river_xkb, qhandle, mods, ESC, Action::Exit);
                seat.create_pointer_binding(qhandle, mods, BTN_LEFT, Action::Move);
                seat.create_pointer_binding(qhandle, mods, BTN_RIGHT, Action::Resize);
                seat.new = false;
            }
        }
    }

    fn manage_windows(&mut self) {
        for window in self.windows.iter_mut() {
            if let Some(seat_proxy) = window.pointer_move_requested.take() {
                let seat = self
                    .seats
                    .get_mut(&seat_proxy.id())
                    .expect("Seat not found!");
                seat.pointer_move(window);
            }
            if let Some(seat_proxy) = window.pointer_resize_requested.take() {
                let seat = self
                    .seats
                    .get_mut(&seat_proxy.id())
                    .expect("Seat not found!");
                seat.pointer_resize(window, window.pointer_resize_requested_edges);
            }
        }
    }

    fn manage_seats(&mut self, wm_proxy: &RiverWindowManagerV1) {
        for seat in self.seats.values_mut() {
            if let Some(window_proxy) = seat.interacted.take() {
                let i = self
                    .windows
                    .iter()
                    .position(|window| window.proxy == window_proxy)
                    .expect("Interacted window not found");

                let window = self.windows.remove(i).unwrap();
                self.windows.push_back(window);
            }

            seat.focus_top(&self.windows);
            seat.do_action(&mut self.windows, wm_proxy);

            if seat.op_release {
                seat.op_end();
                seat.op_release = false;
            } else {
                seat.op_manage();
            }
        }
    }
}

// Represents a window in the window manager.
pub struct Window {
    pub proxy: river::RiverWindowV1,
    pub node: RiverNodeV1,
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

impl Window {
    pub fn new(proxy: RiverWindowV1, qhandle: &QueueHandle<WindowManager>) -> Self {
        let node = proxy.get_node(qhandle, ());

        Self {
            proxy,
            node,
            new: true,
            closed: false,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            pointer_move_requested: None,
            pointer_resize_requested: None,
            pointer_resize_requested_edges: Edges::None,
        }
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.node.set_position(x, y);
        self.x = x;
        self.y = y;
    }
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
        let window = match state.wm_data.windows.iter_mut().find(|x| &x.proxy == proxy) {
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

            Event::ShowWindowMenuRequested { x, y } => {}

            Event::MaximizeRequested => {}

            Event::MinimizeRequested => {}

            Event::UnmaximizeRequested => {}

            Event::FullscreenRequested { output } => {}

            Event::ExitFullscreenRequested => {}

            Event::UnreliablePid { unreliable_pid } => {}

            Event::PresentationHint { hint } => {}

            Event::Identifier { identifier } => {}

            Event::Parent { parent } => {}
        }
    }
}

impl Dispatch<RiverWindowManagerV1, ()> for WindowManager {
    fn event(
        state: &mut Self,
        proxy: &RiverWindowManagerV1,
        event: <RiverWindowManagerV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        use crate::river::river_window_manager_v1::Event;

        match event {
            Event::Unavailable => {
                eprintln!("Another window manager is running!");
                std::process::exit(1);
            }

            Event::Finished => {
                std::process::exit(0);
            }

            Event::ManageStart => {
                let river_xkb = state
                    .river_xkb_global
                    .as_ref()
                    .expect("river_xkb_bindings_v1 missing");

                state.wm_data.handle_manage_start(proxy, river_xkb, qhandle);
            }

            Event::RenderStart => state.wm_data.handle_render_start(proxy),

            Event::SessionLocked => {}

            Event::SessionUnlocked => {}

            Event::Window { id } => state.wm_data.windows.push_back(Window::new(id, qhandle)),

            Event::Output { id } => {
                state.wm_data.outputs.insert(id.id(), Output::new(id));
            }

            Event::Seat { id } => {
                state.wm_data.seats.insert(id.id(), Seat::new(id));
            }
        }

        wayland_client::event_created_child!(WindowManager, RiverWindowManagerV1, [
            river::river_window_manager_v1::EVT_WINDOW_OPCODE => (RiverWindowV1, ()),
            river::river_window_manager_v1::EVT_OUTPUT_OPCODE => (RiverOutputV1, ()),
            river::river_window_manager_v1::EVT_SEAT_OPCODE => (RiverSeatV1, ()),
        ]);
    }
}

impl Dispatch<RiverOutputV1, ()> for WindowManager {
    fn event(
        state: &mut Self,
        proxy: &RiverOutputV1,
        event: <RiverOutputV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        use river::river_output_v1::Event;

        let output = state
            .wm_data
            .outputs
            .get_mut(&proxy.id())
            .expect("Output not found!");

        match event {
            Event::Removed => output.removed = true,

            Event::WlOutput { name: _ } => {}

            Event::Position { x: _, y: _ } => {}

            Event::Dimensions {
                width: _,
                height: _,
            } => {}
        }
    }
}

impl Dispatch<RiverSeatV1, ()> for WindowManager {
    fn event(
        state: &mut Self,
        proxy: &RiverSeatV1,
        event: <RiverSeatV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        use river::river_seat_v1::Event;

        let seat = state
            .wm_data
            .seats
            .get_mut(&proxy.id())
            .expect("Seat not found!");

        match event {
            Event::Removed => seat.removed = true,
            Event::WlSeat { name: _ } => {}
            Event::PointerEnter { window } => seat.hovered = Some(window),
            Event::PointerLeave => seat.hovered = None,
            Event::WindowInteraction { window } => seat.interacted = Some(window),
            Event::ShellSurfaceInteraction { shell_surface } => {}
            Event::OpDelta { dx, dy } => (seat.op_dx, seat.op_dy) = (dx, dy),
            Event::OpRelease => seat.op_release = true,
            Event::PointerPosition { x: _, y: _ } => {}
        }
    }
}

impl Dispatch<RiverXkbBindingV1, ObjectId> for WindowManager {
    fn event(
        state: &mut Self,
        proxy: &RiverXkbBindingV1,
        event: <RiverXkbBindingV1 as Proxy>::Event,
        data: &ObjectId,
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        use river::river_xkb_binding_v1::Event;

        let seat = state.wm_data.seats.get_mut(data).expect("Seat not found!");

        let binding = seat
            .xkb_bindings
            .get(&proxy.id())
            .expect("xkb_binding not found");

        match event {
            Event::Pressed => seat.pending_action = binding.action,
            Event::Released => {}
            Event::StopRepeat => {}
        }
    }
}

impl Dispatch<RiverPointerBindingV1, ObjectId> for WindowManager {
    fn event(
        state: &mut Self,
        proxy: &RiverPointerBindingV1,
        event: <RiverPointerBindingV1 as Proxy>::Event,
        data: &ObjectId,
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        use river::river_pointer_binding_v1::Event;

        let seat = state.wm_data.seats.get_mut(data).expect("Seat not found!");
        let binding = seat
            .pointer_bindings
            .get(&proxy.id())
            .expect("xkb_binding not found!");

        match event {
            Event::Pressed => seat.pending_action = binding.action,
            Event::Released => {}
        }
    }
}

// Ignore some of the requests River sends for now
wayland_client::delegate_noop!(WindowManager: ignore RiverXkbBindingsV1);
wayland_client::delegate_noop!(WindowManager: ignore RiverNodeV1);
