//! The Vigil state model: three orthogonal dimensions + a coordinator that applies
//! telemetry events and produces transition logs.

pub mod activity_state;
pub mod coordinator;
pub mod display_state;
pub mod system_state;

pub use activity_state::ActivityState;
pub use coordinator::{MonitorState, ScreenTimeKind, Transition};
pub use display_state::DisplayState;
pub use system_state::SystemState;
