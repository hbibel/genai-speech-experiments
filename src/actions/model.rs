#[derive(Clone, Copy)]
pub enum Intent {
    Nothing,
    Unclear,
    Ask,
    Brainstorm,
    Change,
}

impl Intent {
    /// Return all the variants of this Enum.
    pub fn variants() -> Vec<Self> {
        // This looks super weird, but it's how I can ensure I don't forget
        // to add new enum variants here.
        // I could also use the strum crate, but I don't want to add a crate
        // just for this.
        let mut vs = Vec::new();
        vs.push(Self::Nothing);
        let mut nxt = Some(Self::Unclear);
        while nxt.is_some() {
            if let Some(intent) = nxt {
                vs.push(intent);
                nxt = match intent {
                    Self::Nothing => Some(Self::Unclear),
                    Self::Unclear => Some(Self::Ask),
                    Self::Ask => Some(Self::Brainstorm),
                    Self::Brainstorm => Some(Self::Change),
                    // last is None
                    Self::Change => None,
                };
            }
        }
        vs
    }
}
