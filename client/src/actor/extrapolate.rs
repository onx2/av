// use crate::{actor::MovementData, module_bindings::MoveIntent};
// use bevy::prelude::*;
// use shared::compute_desired_translation;

// pub(super) fn extrapolate_movement(
//     time: Res<Time>,
//     mut actor_q: Query<(&mut Transform, &MovementData)>,
// ) {
//     let dt = time.delta_secs();
//     actor_q
//         .par_iter_mut()
//         .for_each(|(mut transform, movement_data)| {
//             let current_planar = [transform.translation.x, transform.translation.z];

//             match &movement_data.move_intent {
//                 MoveIntent::Point(point) => {
//                     let target_planar = [point.x, point.z];
//                     let desired_translation = compute_desired_translation(
//                         current_planar,
//                         target_planar,
//                         movement_data.movement_speed,
//                         dt,
//                         movement_data.point_acceptance_radius_sq,
//                     );
//                     println!("Desired Translation: {:?}", desired_translation);
//                     transform.translation.x += desired_translation[0];
//                     // local_actor_q.0.translation.y += desired_translation[1];
//                     transform.translation.z += desired_translation[2];
//                 }
//                 MoveIntent::None => {}
//                 _ => {
//                     println!("Unsupported move_intent")
//                 }
//             }
//         });
// }
