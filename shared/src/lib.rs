use glam::Vec2;

#[derive(Debug)]
pub struct MovementResult2D {
    pub new_position: [f32; 2],
    pub step: [f32; 2],
    pub movement_finished: bool,
}

#[derive(Debug)]
pub struct CalculateStepArgs {
    pub current_position: [f32; 2],
    pub target_position: [f32; 2],
    pub acceptance_radius: f32,
    pub movement_speed: f32,
    pub delta_time_seconds: f32,
}

/// Moves from `current_position` toward `target_position` at `movement_speed` (m/s) for `delta_time_seconds`,
/// stopping on the boundary of `acceptance_radius` around the target (never overlapping).
pub fn calculate_step_2d(args: CalculateStepArgs) -> MovementResult2D {
    let CalculateStepArgs {
        current_position,
        target_position,
        acceptance_radius,
        movement_speed,
        delta_time_seconds,
    } = args;

    let clamped_acceptance_radius = acceptance_radius.max(0.05);
    let clamped_speed = movement_speed.max(0.0);
    let clamped_delta_time = delta_time_seconds.max(0.0);

    let current = Vec2::from_array(current_position);
    let target = Vec2::from_array(target_position);
    let vector_to_target = target - current;

    let distance_to_target = vector_to_target.length();

    // 1) Already within the acceptance radius → no movement, finished.
    if distance_to_target <= clamped_acceptance_radius {
        return MovementResult2D {
            new_position: current_position,
            step: [0.0, 0.0],
            movement_finished: true,
        };
    }

    // 2) Compute how far we can move this frame and how far to the boundary.
    let max_distance_this_frame = clamped_speed * clamped_delta_time;
    let distance_to_boundary = distance_to_target - clamped_acceptance_radius;

    // 3) If we can reach (or pass) the boundary this frame → land exactly on boundary and finish.
    if max_distance_this_frame >= distance_to_boundary {
        // Direction is safe here: distance_to_target > acceptance_radius >= 0
        let direction_to_target = vector_to_target / distance_to_target;
        let boundary_point = target - direction_to_target * clamped_acceptance_radius;
        let step_vector = boundary_point - current;

        return MovementResult2D {
            new_position: boundary_point.to_array(),
            step: step_vector.to_array(),
            movement_finished: true,
        };
    }

    // 4) Otherwise take a partial step toward the target and continue next tick.
    let direction_to_target = vector_to_target / distance_to_target; // normalize
    let step_vector = direction_to_target * max_distance_this_frame;

    MovementResult2D {
        new_position: (current + step_vector).to_array(),
        step: step_vector.to_array(),
        movement_finished: false,
    }
}
