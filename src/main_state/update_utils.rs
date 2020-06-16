use crate::ecs::{
    components::{AccelGraph, Preview, SpeedGraph, Trail, XVelGraph, YVelGraph},
    resources::{
        EnableTrails, FollowSelectedBody, MainIterations, MousePos, NewPreview, Paused,
        PreviewIterations, RelativeTrails, Resolution, StartPoint,
    },
    systems::graph_sys::GraphType,
};

use crate::ecs::entities::{create_preview, new_preview};
use crate::gui::imgui_wrapper::{UiChoice, UiSignal};
use crate::main_state::state::scale_pos;
use crate::main_state::state::MainState;
use crate::saveload::{load_world, save_world_to_lua};
use crate::Vector;

use specs::prelude::*;

use ggez::{input, input::keyboard::KeyCode, Context};

use std::collections::HashSet;

const CAMERA_SPEED: f32 = 1.5;

impl<'a, 'b> MainState<'a, 'b> {
    pub fn run_physics_systems(&mut self, ctx: &mut ggez::Context) {
        let preview_iterations = self.world.fetch::<PreviewIterations>().0;
        if !self.world.fetch::<Paused>().0 {
            let main_iterations = self.world.fetch::<MainIterations>().0;

            // do_physics(&mut self.world, ctx);
            (0..main_iterations).for_each(|_| {
                self.main_dispatcher.dispatch_par(&self.world);
                self.world.maintain();
            });
        }
        if let Some(e) = self.selected_entity {
            if !self.world.is_alive(e) {
                self.selected_entity = None;
            }
        }

        (0..preview_iterations).for_each(|_| {
            self.preview_dispatcher.dispatch(&self.world);
            // if preview collided, delete it and make a new one
            if self.world.fetch::<NewPreview>().0 {
                self.delete_preview();

                let coords = ggez::graphics::screen_coordinates(ctx);

                let start_point = self.world.fetch::<StartPoint>().0;
                if let Some(sp) = start_point {
                    let resolution = self.world.fetch::<Resolution>().0;
                    let mouse_pos = self.world.fetch::<MousePos>().0;
                    let p = scale_pos([mouse_pos.x, mouse_pos.y], coords, resolution);

                    create_preview(
                        &mut self.world,
                        new_preview(
                            sp,
                            (sp - p) * 0.025,
                            self.imgui_wrapper.render_data.create_rad,
                        ),
                    );
                }

                self.world.insert(NewPreview(false));
            }
        });
    }

    pub fn process_gui_signals(&mut self) {
        self.imgui_wrapper
            .sent_signals
            .clone()
            .iter()
            .for_each(|signal| match signal {
                UiSignal::Create => self.creating = !self.creating,
                UiSignal::Delete => {
                    if let Some(e) = self.selected_entity {
                        self.world.insert(FollowSelectedBody(false));
                        self.world
                            .delete_entity(e)
                            .expect("error deleting selected_entity");
                        self.selected_entity = None;
                    }
                }
                UiSignal::AddGraph(graph_type) => {
                    // adds graph component to selected entity
                    macro_rules! add_graphs {
                        ( $ent:expr, $gt:expr, $( [$graph_type:pat, $component:ty] ),* ) => {{
                            match $gt {
                                $(
                                    $graph_type => {
                                        let mut graphs = self.world.write_storage::<$component>();
                                        if graphs.get($ent).is_none() {
                                            graphs
                                                .insert($ent, <$component>::new())
                                                .expect("error adding graph");
                                            } else {
                                                graphs.get_mut($ent).unwrap().display = true;
                                        }
                                    },
                                )*
                            };
                        }}}

                    if let Some(e) = self.selected_entity {
                        add_graphs!(
                            e,
                            graph_type,
                            [GraphType::Speed, SpeedGraph],
                            [GraphType::XVel, XVelGraph],
                            [GraphType::YVel, YVelGraph],
                            [GraphType::Accel, AccelGraph]
                        );
                        if !self.imgui_wrapper.shown_menus.contains(&UiChoice::Graph) {
                            self.imgui_wrapper.shown_menus.insert(UiChoice::Graph);
                        }
                    }
                }
                UiSignal::ToggleGraphs => {
                    macro_rules! undisplay_graphs {
                        ( $( $component:ty ),* ) => {
                            $(
                                let mut graphs = self.world.write_storage::<$component>();
                                (&mut graphs).join().for_each(|graph|{
                                    graph.display = !graph.display;
                                });
                            )*
                        };
                    }
                    undisplay_graphs!(SpeedGraph, XVelGraph, YVelGraph, AccelGraph);
                }
                UiSignal::SaveState => {
                    match save_world_to_lua(
                        &self.world,
                        format!(
                            "saved_systems/{}",
                            self.imgui_wrapper.render_data.save_filename.to_string()
                        ),
                    ) {
                        Ok(()) => println!("Successfully saved the universe"),
                        Err(e) => println!("Error saving the universe: {}", e),
                    }
                }
                UiSignal::LoadState => {
                    self.world.delete_all();
                    match load_world(
                        &self.world,
                        format!(
                            "saved_systems/{}",
                            self.imgui_wrapper.render_data.load_filename.to_string()
                        ),
                    ) {
                        Ok(()) => println!("Successfully loaded previous save"),
                        Err(e) => println!("Error loading save: {}", e),
                    }
                }
                UiSignal::DeleteAll => {
                    self.world.delete_all();
                }
                UiSignal::ToggleFollowBody => {
                    self.world.get_mut::<FollowSelectedBody>().unwrap().toggle();
                }
                UiSignal::ToggleTrails => {
                    self.world.get_mut::<EnableTrails>().unwrap().toggle();
                }
                UiSignal::ToggleRelativeTrails => {
                    self.world.get_mut::<RelativeTrails>().unwrap().toggle();
                    (&mut self.world.write_storage::<Trail>())
                        .join()
                        .for_each(|trail| {
                            trail.points.clear();
                        });
                }
                UiSignal::Pause => {
                    self.world.get_mut::<Paused>().unwrap().toggle();
                }
            });
        self.imgui_wrapper.sent_signals.clear();
    }

    // there's gotta be a better way to do this but its performance doesn't matter
    pub fn delete_preview(&mut self) {
        let mut delset: HashSet<Entity> = HashSet::new();
        {
            let previews = self.world.read_storage::<Preview>();
            let entities = self.world.entities();

            (&entities, &previews).join().for_each(|(entity, _)| {
                delset.insert(entity);
            });
        }

        delset.drain().for_each(|entity| {
            self.world
                .delete_entity(entity)
                .expect("error deleting collided preview");
        });
    }
}

pub fn calc_offset(ctx: &Context) -> Vector {
    let mut offset: Vector = Vector::new(0.0, 0.0);

    if input::keyboard::is_key_pressed(ctx, KeyCode::Up)
        || input::keyboard::is_key_pressed(ctx, KeyCode::W)
    {
        offset.y -= CAMERA_SPEED;
    }
    if input::keyboard::is_key_pressed(ctx, KeyCode::Down)
        || input::keyboard::is_key_pressed(ctx, KeyCode::S)
    {
        offset.y += CAMERA_SPEED;
    }
    if input::keyboard::is_key_pressed(ctx, KeyCode::Left)
        || input::keyboard::is_key_pressed(ctx, KeyCode::A)
    {
        offset.x -= CAMERA_SPEED;
    }
    if input::keyboard::is_key_pressed(ctx, KeyCode::Right)
        || input::keyboard::is_key_pressed(ctx, KeyCode::D)
    {
        offset.x += CAMERA_SPEED;
    }

    offset
}
