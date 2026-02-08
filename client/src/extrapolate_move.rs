// use crate::ActorEntity;
// use crate::module_bindings::MoveIntentData;
// use crate::movement_state::MovementState;
// use crate::secondary_stats::SecondaryStats;
use bevy::prelude::*;
// use nalgebra::Vector2;
// use shared::{get_desired_delta, yaw_from_xz};

pub(super) fn plugin(app: &mut App) {
    // app.add_systems(PreUpdate, extrapolate_move);
}

// TODO: in a Startup system, build a Resource with the physics world and KCC
// let kcc = KinematicCharacterController {
//     autostep: Some(CharacterAutostep {
//         include_dynamic_bodies: false,
//         max_height: CharacterLength::Relative(0.4),
//         ..CharacterAutostep::default()
//     }),
//     offset: CharacterLength::Relative(0.025),
//     ..KinematicCharacterController::default()
// };
// let world_defs = ctx.db.world_static_tbl().iter().map(row_to_def);
// let query_world = build_static_query_world(world_defs, dt);
// let query_pipeline = query_world.as_query_pipeline(QueryFilter::only_fixed());

// fn extrapolate_move(
//     time: Res<Time>,
//     mut query: Query<(&mut Transform, &MovementState, &SecondaryStats), With<ActorEntity>>,
// ) {
//     let dt = time.delta_secs();

//     query
//         .iter_mut()
//         .for_each(|(mut transform, movement_state, secondary_stats)| {
//             // TODO: add CapuleY to the actor state locally...?
//             if !movement_state.should_move {
//                 return;
//             }

//             let current_planar = transform.translation.xz();
//             let target_planar = match &movement_state.move_intent {
//                 MoveIntentData::Point(point) => Vec2::new((point).x, (point).z),
//                 _ => current_planar,
//             };
//             let movement_speed_mps = secondary_stats.movement_speed;
//             let direction = (target_planar - current_planar)
//                 .try_normalize()
//                 .unwrap_or_default();

//             if let Some(yaw) = yaw_from_xz(Vector2::new(direction.x, direction.y)) {
//                 transform.rotation = Quat::from_rotation_y(yaw);
//             }

//             let desired_delta = get_desired_delta(
//                 Vector2::new(current_planar.x, current_planar.y),
//                 Vector2::new(target_planar.x, target_planar.y),
//                 movement_speed_mps,
//                 movement_state.vertical_velocity,
//                 dt,
//             );

//             println!("Desired Delta: {:?}", desired_delta);

//             transform.translation.x += desired_delta.x;
//             transform.translation.y += desired_delta.y;
//             transform.translation.z += desired_delta.z;
//         });
// }
