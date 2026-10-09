use crate::river::{
    river_seat_v1::{Modifiers, RiverSeatV1},
    river_window_manager_v1::RiverWindowManagerV1,
    river_window_v1::{Edges, RiverWindowV1},
    river_xkb_bindings_v1::RiverXkbBindingsV1,
};
use crate::{Action, PointerBinding, Window, WindowManager, XkbBinding, river};
use std::collections::{HashMap, VecDeque};
use wayland_client::{Proxy, QueueHandle};
use wayland_backend::client::ObjectId;

#[derive(Debug)]
pub struct Seat {
    pub proxy: RiverSeatV1,
    pub new: bool,
    pub removed: bool,
    pub focused: Option<RiverWindowV1>,
    pub hovered: Option<RiverWindowV1>,
    pub interacted: Option<RiverWindowV1>,
    pub xkb_bindings: HashMap<ObjectId, XkbBinding>,
    pub pointer_bindings: HashMap<ObjectId, PointerBinding>,
    pub pending_action: Action,
    pub op: SeatOp,
    pub op_dx: i32,
    pub op_dy: i32,
    pub op_release: bool,
}

#[derive(Debug, Clone)]
pub enum SeatOp {
    None,
    Move {
        window_proxy: RiverWindowV1,
        start_x: i32,
        start_y: i32,
    },
    Resize {
        window_proxy: RiverWindowV1,
        start_x: i32,
        start_y: i32,
        start_width: i32,
        start_height: i32,
        edges: Edges,
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

    pub fn create_xkb_binding(
        &mut self,
        river_xkb: &RiverXkbBindingsV1,
        qhandle: &QueueHandle<WindowManager>,
        mods: Modifiers,
        keysym: u32,
        action: Action,
    ) {
        let proxy = river_xkb.get_xkb_binding(&self.proxy, keysym, mods, qhandle, self.proxy.id());
        proxy.enable();

        let binding = XkbBinding { proxy, action };

        self.xkb_bindings.insert(binding.proxy.id(), binding);
    }

    pub fn create_pointer_binding(
        &mut self,
        qhandle: &QueueHandle<WindowManager>,
        mods: Modifiers,
        button: u32,
        action: Action,
    ) {
        let proxy = self
            .proxy
            .get_pointer_binding(button, mods, qhandle, self.proxy.id());
        proxy.enable();

        let binding = PointerBinding { proxy, action };
        self.pointer_bindings.insert(binding.proxy.id(), binding);
    }

    pub fn do_action(&mut self, windows: &mut VecDeque<Window>, wm_proxy: &RiverWindowManagerV1) {
        match self.pending_action {
            Action::None => {}

            Action::SpawnFoot => {
                // Removing WAYLAND_DEBUG avoids lots of noise
                match std::process::Command::new("foot")
                    .env_remove("WAYLAND_DEBUG")
                    .spawn()
                {
                    Ok(_) => {}
                    Err(e) => {
                        crate::exit!("Unable to spawn foot: {e}");
                    }
                }
            }

            Action::Close => {
                if let Some(window_proxy) = self.focused.as_ref() {
                    window_proxy.close();
                }
            }

            Action::FocusNext => {
                if !windows.is_empty() {
                    windows.rotate_left(1);
                    self.focus_top(windows);
                }
            }

            Action::Move => {
                if let (Some(window_proxy), SeatOp::None) = (self.hovered.as_ref(), &self.op) {
                    let window = windows
                        .iter()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Hovered window not found");

                    self.pointer_move(window);
                }
            }

            Action::Resize => {
                if let (Some(window_proxy), SeatOp::None) = (self.hovered.as_ref(), &self.op) {
                    let window = windows
                        .iter()
                        .find(|window| &window.proxy == window_proxy)
                        .expect("Hovered window not found");

                    self.pointer_resize(window, Edges::Bottom.union(Edges::Right));
                }
            }

            Action::Exit => wm_proxy.exit_session(),
        }

        self.pending_action = Action::None;
    }

    pub fn op_end(&mut self) {
        if let SeatOp::Resize { window_proxy, .. } = &self.op {
            window_proxy.inform_resize_end();
        }

        self.proxy.op_end();
        self.op = SeatOp::None;
    }

    pub fn op_manage(&mut self) {
        match &self.op {
            SeatOp::None | SeatOp::Move { .. } => {}

            SeatOp::Resize {
                window_proxy,
                start_width,
                start_height,
                edges,
                ..
            } => {
                let (mut width, mut height) = (*start_width, *start_height);

                if edges.contains(Edges::Left) {
                    width -= self.op_dx;
                }
                if edges.contains(Edges::Right) {
                    width += self.op_dx;
                }
                if edges.contains(Edges::Top) {
                    height -= self.op_dy;
                }
                if edges.contains(Edges::Bottom) {
                    height += self.op_dy;
                }

                window_proxy.propose_dimensions(width.max(1), height.max(1));
            }
        }
    }

    pub fn focus_top(&mut self, windows: &VecDeque<Window>) {
        match windows.back() {
            Some(window) => {
                self.proxy.focus_window(&window.proxy);
                window.node.place_top();
                self.focused = Some(window.proxy.clone());
            }

            None => {
                self.proxy.clear_focus();
                self.focused = None;
            }
        }
    }

    pub fn pointer_move(&mut self, window: &Window) {
        self.interacted = Some(window.proxy.clone());

        self.proxy.op_start_pointer();

        self.op = SeatOp::Move {
            window_proxy: window.proxy.clone(),
            start_x: window.x,
            start_y: window.y,
        };

        self.op_dx = 0;
        self.op_dy = 0;
    }

    pub fn pointer_resize(&mut self, window: &Window, edges: Edges) {
        self.interacted = Some(window.proxy.clone());
        self.proxy.op_start_pointer();

        window.proxy.inform_resize_start();

        self.op = SeatOp::Resize {
            window_proxy: window.proxy.clone(),
            start_x: window.x,
            start_y: window.y,
            start_width: window.width,
            start_height: window.height,
            edges
        };

        self.op_dx = 0;
        self.op_dy = 0;
    }
}
