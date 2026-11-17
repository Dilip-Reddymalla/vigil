//! **SystemState** — OS-level dimension: `Unlocked ⇄ Locked ⇄ Sleeping`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemState {
    Unlocked,
    Locked,
    Sleeping,
}

impl SystemState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemState::Unlocked => "unlocked",
            SystemState::Locked => "locked",
            SystemState::Sleeping => "sleeping",
        }
    }

    pub fn is_unlocked(&self) -> bool {
        matches!(self, SystemState::Unlocked)
    }
}

impl Default for SystemState {
    fn default() -> Self {
        SystemState::Unlocked
    }
}
