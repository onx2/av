use spacetimedb::*;

/// Kinematic Character Controller (KCC) settings shared by server and clients.
///
/// This is intended to be a single-row table (e.g. `id = 1`) that both:
/// - the server reads to configure Rapier's `KinematicCharacterController`, and
/// - clients subscribe to in order to recreate the same controller configuration locally.
///
/// Notes
/// - Values are expressed in meters, seconds, and degrees (converted to radians at runtime).
/// - Autostep and snap-to-ground are always enabled (per current design).
#[table(name = kcc_settings, public)]
pub struct KccSettings {
    /// Unique id (primary key). Use a single row with `id = 1`.
    #[primary_key]
    pub id: u32,

    /// Small gap preserved between the character and its surroundings (meters).
    /// Keep `offset` small but non-zero for numerical stability
    pub offset: f32,

    /// Maximum climbable slope angle (degrees).
    pub max_slope_climb_deg: f32,

    /// Minimum slope angle (degrees) before automatic sliding starts.
    pub min_slope_slide_deg: f32,

    /// Autostep maximum height (meters). Always enabled.
    pub autostep_max_height: f32,

    /// Autostep minimum width (meters). Always enabled.
    pub autostep_min_width: f32,

    /// Whether the controller should slide against obstacles.
    pub slide: bool,

    /// Increase if the character gets stuck when sliding (small, meters).
    pub normal_nudge_factor: f32,

    /// Constant falling speed magnitude (m/s) applied as downward motion when airborne.
    pub fall_speed_mps: f32,

    /// Small downward bias magnitude (m/s) applied while grounded to satisfy snap-to-ground preconditions.
    pub grounded_down_bias_mps: f32,

    /// The squared distance from a point that we consider reached / close enough
    /// Helpful to prevent floating point errors and jitter
    pub point_acceptance_radius_sq: f32,
}
