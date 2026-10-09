use lxwm::river;
use std::error::Error;
use wayland_client::Connection;


fn main() -> Result<(), Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;
    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let registry = display.get_registry(&event_queue.handle(), ());

    let mut window_manager = lxwm::WindowManager::default();

    event_queue.roundtrip(&mut window_manager)?;

    if window_manager.river_wm_global.is_none() {
        lxwm::exit!(
            "river_window_manager_v1 global has not been found. Are you sure River is running?"
        );
    }

    if window_manager.river_xkb_global.is_none() {
        lxwm::exit!(
          "river_xkb_bindings_v1 global has not been found. Is River running with xkb support?"
        );
    }

    loop {
        event_queue.blocking_dispatch(&mut window_manager)?;
    }
}
