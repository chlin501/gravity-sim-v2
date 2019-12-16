use specs::prelude::*;

use crate::components::{Kinematics, Mass, Position, Preview, Radius};
use crate::resources::{MainIterations, PreviewIterations, Resolution, StartPoint, DT};
use crate::{Body, Vector, G};

use std::collections::HashSet;

pub struct PhysicsSys;

impl<'a> System<'a> for PhysicsSys {
    type SystemData = (
        WriteStorage<'a, Position>,
        WriteStorage<'a, Kinematics>,
        ReadStorage<'a, Preview>,
        ReadStorage<'a, Radius>,
        ReadStorage<'a, Mass>,
        Read<'a, MainIterations>,
        Read<'a, PreviewIterations>,
        Read<'a, StartPoint>,
        Read<'a, DT>,
    );

    fn run(
        &mut self,
        (
            mut positions,
            mut kinematics,
            previews,
            radii,
            masses,
            main_iterations,
            preview_iterations,
            start_point,
            dt,
        ): Self::SystemData,
    ) {
        let mut preview_only = false;
        (0..std::cmp::max(preview_iterations.0, main_iterations.0)).for_each(|i| {
            preview_only = i >= main_iterations.0;
            integrate_positions(&mut positions, &kinematics, &previews, preview_only, dt.0);
            apply_gravity(
                &positions,
                &mut kinematics,
                &radii,
                &masses,
                &previews,
                preview_only,
            );
            integrate_kinematics(&mut kinematics, &previews, preview_only, dt.0);
        });
    }
}

fn integrate_positions(
    positions: &mut WriteStorage<'_, Position>,
    kinematics: &WriteStorage<'_, Kinematics>,
    previews: &ReadStorage<'_, Preview>,
    preview_only: bool,
    dt: f32,
) {
    // let mut pos_integrate_query = <(Write<Position>, Read<Kinematics>)>::query();

    let int_closure = |(pos, kinematics): (&mut Position, &Kinematics)| {
        pos.0 += kinematics.vel * dt + (kinematics.accel / 2.0) * dt.powi(2);
    };

    if !preview_only {
        (positions, kinematics).join().for_each(int_closure);
    } else {
        (positions, kinematics, previews)
            .join()
            .for_each(|(pos, kine, _)| {
                int_closure((pos, kine));
            });
    }
}

fn apply_gravity(
    positions: &WriteStorage<'_, Position>,
    kinematics: &mut WriteStorage<'_, Kinematics>,
    radii: &ReadStorage<'_, Radius>,
    masses: &ReadStorage<'_, Mass>,
    previews: &ReadStorage<'_, Preview>,
    preview_only: bool,
) {
    let grav_closure = |(current_pos, kinematics, _): (&Position, &mut Kinematics, &Radius)| {
        // kinematics.accel = inner_query.iter_immutable(&world).fold(
        kinematics.accel = (positions, masses).join().fold(
            Vector::new(0.0, 0.0),
            |grav_accel_acc, (other_pos, other_mass)| {
                if current_pos != other_pos {
                    let dist_vec = other_pos.0 - current_pos.0;
                    let dist_mag_sqr = dist_vec.norm_squared();
                    let dist_mag = dist_mag_sqr.powf(0.5);
                    let dist_comp = dist_vec / dist_mag;

                    let grav_accel_mag = other_mass.0 / dist_mag_sqr * G;
                    let grav_accel: Vector = dist_comp * grav_accel_mag;

                    grav_accel_acc + grav_accel
                } else {
                    grav_accel_acc
                }
            },
        );
    };

    if !preview_only {
        (positions, kinematics, radii).join().for_each(grav_closure);
    } else {
        (positions, kinematics, radii, previews)
            .join()
            .for_each(|(p, k, r, _)| grav_closure((p, k, r)));
    }
}

fn integrate_kinematics(
    kinematics: &mut WriteStorage<'_, Kinematics>,
    previews: &ReadStorage<'_, Preview>,
    preview_only: bool,
    dt: f32,
) {
    let kine_int_closure = |kinematics: (&mut Kinematics)| {
        *kinematics.vel = *(kinematics.vel + (kinematics.accel + kinematics.past_accel) / 2.0 * dt);
        kinematics.past_accel = kinematics.accel;
    };

    if !preview_only {
        (kinematics).join().for_each(kine_int_closure);
    } else {
        (kinematics, previews).join().for_each(|(kine, _)| {
            kine_int_closure(kine);
        });
    }
}

fn calc_collisions(
    positions: &WriteStorage<'_, Position>,
    kinematics: &WriteStorage<'_, Kinematics>,
    masses: &WriteStorage<'_, Mass>,
    radii: &WriteStorage<'_, Radius>,
    entities: &Entities,
    start_point: StartPoint,
    resolution: Resolution,
) -> (HashSet<Body>, HashSet<Entity>) {
    let create_set: HashSet<Body> = HashSet::new();
    let delete_set: HashSet<Entity> = HashSet::new();

    (positions, radii, masses, kinematics, entities)
        .join()
        .for_each(|(pos1, r1, m1, k1, e1)| {
            (positions, radii, masses, kinematics, entities)
                .join()
                .for_each(|(pos2, r2, m2, k2, e2)| {
                    if e1 != e2
                        && pos1.dist(*pos2) <= r1.0 + r2.0
                        && !delete_set.contains(&e1)
                        && !delete_set.contains(&e2)
                    {}
                });
        });

    (create_vec, delete_vec)
}
