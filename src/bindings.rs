use crate::river;

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
    pub proxy: river::RiverXkbBindingV1,
    pub action: Action,
}

pub struct PointerBinding {
    pub proxy: river::RiverPointerBindingV1,
    pub action: Action,
}

pub struct Output {
    pub proxy: river::RiverOutputV1,
    pub removed: bool,
}
