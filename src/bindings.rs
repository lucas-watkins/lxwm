use crate::river::{RiverOutputV1, RiverXkbBindingV1, RiverPointerBindingV1};

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

pub struct XkbBinding {
    pub proxy: RiverXkbBindingV1,
    pub action: Action,
}

pub struct PointerBinding {
    pub proxy: RiverPointerBindingV1,
    pub action: Action,
}

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
