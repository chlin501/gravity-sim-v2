extern crate ggez;
use ggez::*;

use legion::prelude::*;

mod components;
use components::{Draw, Kinematics, Mass, Point, Position, Radius, Vector};

mod main_state;
use main_state::MainState;

mod physics;
mod imgui_wrapper;
use imgui_wrapper::ImGuiWrapper;

const G: f32 = 66.74;
const SCREEN_X: f32 = 30.0;
const SCREEN_Y: f32 = 30.0;

type Body = (Position, Kinematics, Mass, Draw, Radius);

pub fn new_body(pos: impl Into<Point>, vel: impl Into<Vector>, mass: f32, rad: f32) -> Body {
    (
        Position(pos.into()),
        Kinematics::new(vel.into()),
        Mass(mass),
        Draw(ggez::graphics::WHITE),
        Radius(rad),
    )
}

fn main() -> GameResult {
    let (ctx, event_loop) = &mut ggez::ContextBuilder::new("N-body gravity sim", "Mikail Khan")
        .window_setup(ggez::conf::WindowSetup::default().title("Gravity"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(600.0, 600.0))
        .build()
        .expect("error building context");

    let universe = Universe::new(None);
    let mut world = universe.create_world();

        world.insert_from(
            (),
            vec![
                // new_body([28.0, 15.0], [0.3, -0.3], 0.01, 0.5),
                new_body([15.0, 15.0], [0.0, 0.0], 100.0, 1.0),
                // new_body([0.0, 0.0], [-0.3, -0.1], 1.0, 0.1),
            ],
        );

    // world.insert_from(
    //     (),
    //     (0..1100).map(|i| {
    //         (new_body(
    //             [(i / 10) as f32 * 100.0, (i % 10) as f32 * 100.0],
    //             [0.0, 0.0],
    //             -0.1,
    //             5.0,
    //         ))
    //     }),
    // );
    let hidpi_factor = event_loop.get_primary_monitor().get_hidpi_factor() as f32;
    let dimensions = event_loop.get_primary_monitor().get_dimensions();
    let aspect_ratio = dimensions.height / dimensions.width;
    graphics::set_mode(
        ctx,
        ggez::conf::WindowMode::default()
            .dimensions(dimensions.width as f32, dimensions.height as f32),
    )
    .expect("error resizing window");

    graphics::set_screen_coordinates(ctx, graphics::Rect::new(0., 0., SCREEN_X, SCREEN_Y / aspect_ratio as f32)).unwrap();

    let main_state = &mut MainState::new(universe, world, ImGuiWrapper::new(ctx, hidpi_factor), hidpi_factor);
    event::run(ctx, event_loop, main_state)
}
