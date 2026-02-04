use crate::{
    actor::{ActorEntityMapping, ensure_actor_entity},
    module_bindings::TransformRow,
};
use bevy::prelude::*;
use bevy_spacetimedb::{ReadInsertMessage, ReadUpdateMessage};
use shared::utils::yaw_from_u8;

/// Cached server transform data for an entity.
#[derive(Component, Debug)]
pub struct NetTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        ((on_transform_inserted, on_transform_updated), interpolate).chain(),
    );
}

fn on_transform_inserted(
    mut commands: Commands,
    mut msgs: ReadInsertMessage<TransformRow>,
    mut oe_mapping: ResMut<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        println!("on_transform_inserted: {:?}", msg.row.actor_id);
        // Ensure the owner entity exists regardless of message ordering.
        let bevy_entity = ensure_actor_entity(&mut commands, &mut oe_mapping, msg.row.actor_id);

        // Use Commands to avoid timing issues with deferred spawns/components.
        let translation: Vec3 = msg.row.data.translation.clone().into();
        let rotation: Quat = Quat::from_rotation_y(yaw_from_u8(msg.row.data.yaw));

        commands.entity(bevy_entity).insert((
            // Make visible now that we have a valid transform. TODO: this might not be necessary once assets for the character are used.
            Visibility::Inherited,
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
            NetTransform {
                translation,
                rotation,
            },
        ));
    }
}

fn on_transform_updated(
    mut transform_q: Query<&mut NetTransform>,
    mut msgs: ReadUpdateMessage<TransformRow>,
    oe_mapping: Res<ActorEntityMapping>,
) {
    for msg in msgs.read() {
        let Some(&bevy_entity) = oe_mapping.0.get(&msg.new.actor_id) else {
            continue;
        };
        let Ok(mut net_transform) = transform_q.get_mut(bevy_entity) else {
            continue;
        };
        // println!("on_transform_updated: {:?}", transform.actor_id);
        net_transform.translation = msg.new.data.translation.clone().into();
        net_transform.rotation = Quat::from_rotation_y(yaw_from_u8(msg.new.data.yaw));
    }
}

fn interpolate(time: Res<Time>, mut transform_q: Query<(&mut Transform, &NetTransform)>) {
    let dt = time.delta_secs();
    transform_q.par_iter_mut().for_each(|(mut transform, net)| {
        transform
            .translation
            .smooth_nudge(&net.translation, 12.0, dt);
        transform.rotation = transform
            .rotation
            .slerp(net.rotation, 1.0 - (-14.0 * dt).exp());
    });
}
