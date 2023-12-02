use ggez::conf;
use ggez::event::{self, EventHandler};
use ggez::glam::*;
use ggez::graphics::{self, Color};
use ggez::input::keyboard::KeyCode;
use ggez::input::keyboard::KeyInput;
use ggez::timer;
use ggez::{Context, ContextBuilder, GameResult};
use oorandom::Rand32;

type Point2 = Vec2;

// convert an angle into a vector, from ggez example

fn vec_from_angle(angle: f32) -> Vec2 {
    let vx = angle.sin();
    let vy = angle.cos();
    Vec2::new(vx, vy)
}

// get screen coordinates from world coordinates, from ggez example

fn world_to_screen_coords(screen_width: f32, screen_height: f32, point: Point2) -> Point2 {
    let x = point.x + screen_width / 2.0;
    let y = screen_height - (point.y + screen_height / 2.0);
    Point2::new(x, y)
}

#[derive(Debug)]
struct InputState {
    xaxis: f32,
    yaxis: f32,
    fire: bool,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            xaxis: 0.0,
            yaxis: 0.0,
            fire: false,
        }
    }
}

#[derive(Debug)]
struct Actor {
    pos: Point2,
    initial_pos: Point2,
    angle: f32,
    life: f32,
    elapsed: f32, // for interceptor
    radius: f32,  // for interceptor
}

const GROUND_HEIGHT: f32 = 150.0;

const CURSOR_VEL: f32 = 600.0;
const CURSOR_WIDTH: f32 = 20.0;
const CURSOR_HEIGHT: f32 = 5.0;

const ROCKET_WIDTH: f32 = 7.5;
const ROCKET_HEIGHT: f32 = 7.5;

const ROCKET_LIFE: f32 = 1.0;
const GROUND_LIFE: f32 = 5.0;

const INTERCEPTOR_BASE_RADIUS: f32 = 20.0;
const INTERCEPTOR_PERIOD: f32 = 5.0;

fn create_player_cursor() -> Actor {
    Actor {
        pos: Point2::ZERO,
        initial_pos: Point2::ZERO,
        angle: 0.0,
        life: GROUND_LIFE,
        elapsed: 0.0,
        radius: 0.0,
    }
}

fn create_rocket() -> Actor {
    Actor {
        pos: Point2::ZERO,
        initial_pos: Point2::ZERO,
        angle: 0.0,
        life: ROCKET_LIFE,
        elapsed: 0.0,
        radius: 0.0,
    }
}

fn create_interceptor() -> Actor {
    Actor {
        pos: Point2::ZERO,
        initial_pos: Point2::ZERO,
        angle: 0.0,
        life: ROCKET_LIFE,
        elapsed: INTERCEPTOR_PERIOD,
        radius: INTERCEPTOR_BASE_RADIUS,
    }
}

const ROCKET_VEL: f32 = 80.0;
const ROCKET_DELAY: f32 = 4.0;
const SHOT_TIMEOUT: f32 = 0.5;

// Prevents the cursor from going out of bounds
fn check_cursor_bound(actor: &mut Actor, x: f32, y: f32) -> bool {
    let screen_x = x / 2.0;
    let screen_y = y / 2.0;

    if actor.pos.x + CURSOR_WIDTH > screen_x {
        // can't let the cursor get stuck, so adjust for each case
        actor.pos -= Vec2::new(1.0, 0.0);
        return false;
    } else if actor.pos.x < -screen_x {
        actor.pos += Vec2::new(1.0, 0.0);
        return false;
    }

    if actor.pos.y > screen_y {
        actor.pos -= Vec2::new(0.0, 1.0);
        return false;
    } else if actor.pos.y - CURSOR_HEIGHT < -screen_y + GROUND_HEIGHT {
        actor.pos += Vec2::new(0.0, 1.0);
        return false;
    }
    true
}

// Move the cursor based on the input supplied
fn cursor_move(actor: &mut Actor, x: f32, y: f32, input: &InputState, dt: f32) {
    if check_cursor_bound(actor, x, y) {
        actor.pos += Vec2::new(input.xaxis * CURSOR_VEL * dt, input.yaxis * CURSOR_VEL * dt);
    }
}

// Move the rocket based on its angle and velocity
fn rocket_move(actor: &mut Actor, dt: f32) {
    actor.pos += vec_from_angle(actor.angle) * ROCKET_VEL * dt;
}

// Keep track of the lifetime of each interceptor, in order to
// facilitate the explosion animation and keep track of lifetime
fn interceptor_elapse(actor: &mut Actor, dt: f32) {
    actor.elapsed -= dt * 3.0; // make it a tad faster

    // https://www.desmos.com/calculator/rwux8jpeud
    // Model explosion radius with this function I randomly came up with
    // by messing around in desmos until it had the behavior I wanted
    actor.radius =
        INTERCEPTOR_BASE_RADIUS * (-(((actor.elapsed - 2.5) * (actor.elapsed - 2.5)) / 2.5) + 2.5);
}

struct MainState {
    player: Actor,
    screen_width: f32,
    screen_height: f32,
    input: InputState,
    rockets: Vec<Actor>,
    interceptors: Vec<Actor>,
    shot_timeout: f32,
    rocket_delay: f32,
    rng: Rand32,
    level_timer: f32,
    level: u32,
    score: i32,
}

const LEVEL_TIME: f32 = 15.0;

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        println!("rust_command Instructions:");
        println!("Use arrow keys to move cursor");
        println!("Use space to fire an interceptor");

        let rng = Rand32::new(1337);

        let player = create_player_cursor();

        let (width, height) = ctx.gfx.drawable_size();

        let s = MainState {
            player,
            screen_width: width,
            screen_height: height,
            input: InputState::default(),
            rockets: Vec::new(),
            interceptors: Vec::new(),
            shot_timeout: 0.0,
            rocket_delay: ROCKET_DELAY,
            rng,
            level_timer: LEVEL_TIME,
            level: 1,
            score: 0,
        };

        Ok(s)
    }

    // Handle the case where a missile hits the side of the screen or the ground
    fn handle_border_collisions(&mut self) -> GameResult {
        let screen_x = self.screen_width / 2.0;
        let screen_y = self.screen_height / 2.0;

        for rocket in &mut self.rockets {
            if rocket.pos.y < -screen_y + GROUND_HEIGHT {
                // hit ground
                rocket.life = 0.0; // kill missile
                self.player.life -= 1.0; // damage player

                // make explosion by recycling the interceptor code
                let mut explosion = create_interceptor();
                explosion.pos = rocket.pos;
                self.interceptors.push(explosion);
            }
            if rocket.pos.x > screen_x || rocket.pos.x < -screen_x {
                // hit side
                rocket.life = 0.0; // kill missile
            }
        }
        Ok(())
    }

    // Handle collisions between interceptors and missiles
    fn handle_interceptions(&mut self) -> GameResult {
        for rocket in &mut self.rockets {
            for interceptor in &mut self.interceptors {
                let dist = rocket.pos - interceptor.pos;
                if dist.length() < interceptor.radius {
                    // collision
                    rocket.life = 0.0; // kill rocket, interceptor will be killed by elapse system
                    self.score += 150;
                }
            }
        }
        Ok(())
    }

    // Fire a new interceptor by adding it to state
    fn fire_interceptor(&mut self) {
        self.shot_timeout = SHOT_TIMEOUT;
        let mut shot = create_interceptor();

        shot.pos = self.player.pos;
        self.interceptors.push(shot);
    }

    // create a wave of rockets, adapted from the ggez example create_rock method
    fn create_rockets(&mut self, num: u32, x: f32, y: f32) -> Vec<Actor> {
        self.rocket_delay = ROCKET_DELAY;

        let screen_x = x / 2.0;
        let screen_y = y / 2.0;

        let new_rocket = |_| {
            let mut rocket = create_rocket();
            // random starting pos at top of screen
            let start_pos = Vec2::new(self.rng.rand_float() * x - screen_x, screen_y);
            // generate a random angle between 0.75 PI and 1.25 PI
            // an angle of PI sends the rocket straight downward
            let angle =
                self.rng.rand_float() * 0.5 * std::f32::consts::PI + 0.75 * std::f32::consts::PI;
            rocket.pos = start_pos;
            rocket.initial_pos = start_pos;
            rocket.angle = angle;
            rocket
        };
        (0..num).map(new_rocket).collect()
    }
}

fn draw_ground(canvas: &mut graphics::Canvas, world_coords: (f32, f32)) {
    let (screen_w, screen_h) = world_coords;
    let rect = graphics::Rect::new(0.0, screen_h - GROUND_HEIGHT, screen_w, GROUND_HEIGHT);
    canvas.draw(
        &graphics::Quad,
        graphics::DrawParam::new()
            .dest(rect.point())
            .scale(rect.size())
            .color(Color::WHITE),
    );
}

fn draw_cursor(canvas: &mut graphics::Canvas, actor: &Actor, world_coords: (f32, f32)) {
    let (screen_w, screen_h) = world_coords;
    let pos = world_to_screen_coords(screen_w, screen_h, actor.pos);
    let rect1 = graphics::Rect::new(pos.x, pos.y, CURSOR_WIDTH, CURSOR_HEIGHT);
    canvas.draw(
        &graphics::Quad,
        graphics::DrawParam::new()
            .dest(rect1.point())
            .scale(rect1.size())
            .color(Color::WHITE),
    );

    let rect2 = graphics::Rect::new(
        pos.x + CURSOR_WIDTH / 2.0 - CURSOR_HEIGHT / 2.0,
        pos.y + CURSOR_HEIGHT / 2.0 - CURSOR_WIDTH / 2.0,
        CURSOR_HEIGHT,
        CURSOR_WIDTH,
    );

    canvas.draw(
        &graphics::Quad,
        graphics::DrawParam::new()
            .dest(rect2.point())
            .scale(rect2.size())
            .color(Color::WHITE),
    );
}

fn draw_rocket(
    canvas: &mut graphics::Canvas,
    ctx: &mut Context,
    actor: &Actor,
    world_coords: (f32, f32),
    level: u32,
) {
    let (screen_w, screen_h) = world_coords;

    let endpoint = Vec2::new(
        actor.pos.x + ROCKET_WIDTH / 2.0,
        actor.pos.y - ROCKET_HEIGHT / 2.0,
    );

    let points = &[
        world_to_screen_coords(screen_w, screen_h, actor.initial_pos),
        world_to_screen_coords(screen_w, screen_h, endpoint),
    ];

    // tracer line
    let mut modifier = level as f32 / 10.0;

    if modifier > 1.0 {
        modifier = 1.0;
    }

    let tracer_color = Color::new(modifier, 1.0 - modifier, 0.0, 1.0);
    let line = graphics::Mesh::new_line(ctx, points, 5.0, tracer_color).unwrap();

    canvas.draw(&line, Vec2::new(0.0, 0.0));

    let pos = world_to_screen_coords(screen_w, screen_h, actor.pos);
    let rect = graphics::Rect::new(pos.x, pos.y, ROCKET_WIDTH, ROCKET_HEIGHT);

    // rocket body
    canvas.draw(
        &graphics::Quad,
        graphics::DrawParam::new()
            .dest(rect.point())
            .scale(rect.size())
            .color(Color::WHITE),
    );
}

fn draw_interceptor(
    canvas: &mut graphics::Canvas,
    ctx: &mut Context,
    actor: &Actor,
    world_coords: (f32, f32),
) {
    let (screen_w, screen_h) = world_coords;
    let pos = world_to_screen_coords(screen_w, screen_h, actor.pos);

    let points = &[
        Vec2::new(screen_w / 2.0, screen_h - GROUND_HEIGHT),
        world_to_screen_coords(screen_w, screen_h, actor.pos),
    ];

    let tracer_color: Color = Color::new(1.0, 1.0, 1.0, actor.elapsed / INTERCEPTOR_PERIOD);
    // tracer line
    let line = graphics::Mesh::new_line(ctx, points, 5.0, tracer_color).unwrap();

    canvas.draw(&line, Vec2::new(0.0, 0.0));

    let circle = graphics::Mesh::new_circle(
        ctx,
        graphics::DrawMode::fill(),
        pos,
        actor.radius,
        10.0, // for weird pixellated polygon action
        Color::WHITE,
    )
    .unwrap();

    // explosion
    canvas.draw(&circle, Vec2::new(0.0, 0.0));
}

const HEALTHBAR_WIDTH: f32 = 200.0;
const HEALTHBAR_HEIGHT: f32 = 50.0;

fn draw_healthbar(canvas: &mut graphics::Canvas, actor: &Actor, screen_h: f32) {
    let container = graphics::Rect::new(25.0, screen_h - 75.0, HEALTHBAR_WIDTH, HEALTHBAR_HEIGHT);

    let bar_width = (actor.life / GROUND_LIFE) * (HEALTHBAR_WIDTH - 10.0);
    let bar_color = Color::new(
        1.0 - actor.life / GROUND_LIFE,
        actor.life / GROUND_LIFE,
        0.0,
        1.0,
    );
    let health_bar = graphics::Rect::new(
        25.0 + 5.0,
        screen_h - 75.0 + 5.0,
        bar_width,
        HEALTHBAR_HEIGHT - 10.0,
    );

    canvas.draw(
        &graphics::Quad,
        graphics::DrawParam::new()
            .dest(container.point())
            .scale(container.size())
            .color(Color::BLACK),
    );

    canvas.draw(
        &graphics::Quad,
        graphics::DrawParam::new()
            .dest(health_bar.point())
            .scale(health_bar.size())
            .color(bar_color),
    );
}

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        const DESIRED_FPS: u32 = 60;

        while ctx.time.check_update_time(DESIRED_FPS) {
            let seconds = 1.0 / (DESIRED_FPS as f32);

            self.level_timer -= seconds;
            if self.level_timer <= 0.0 {
                self.level += 1;
                self.level_timer = LEVEL_TIME;
            }

            cursor_move(
                &mut self.player,
                self.screen_width,
                self.screen_height,
                &self.input,
                seconds,
            );

            self.shot_timeout -= seconds;

            if self.input.fire && self.shot_timeout <= 0.0 {
                self.fire_interceptor();
            }

            self.rocket_delay -= seconds;

            let num_rockets = self.rng.rand_range((1 + self.level)..(3 + self.level));

            if self.rocket_delay <= 0.0 {
                for rocket in
                    self.create_rockets(num_rockets, self.screen_width, self.screen_height)
                {
                    self.rockets.push(rocket);
                }
            }

            for rocket in &mut self.rockets {
                rocket_move(rocket, seconds);
            }

            for interceptor in &mut self.interceptors {
                interceptor_elapse(interceptor, seconds);
            }

            self.handle_border_collisions()?;
            self.handle_interceptions()?;

            // kill dead missiles and elapsed interceptors
            self.rockets.retain(|r| r.life > 0.0);
            self.interceptors.retain(|i| i.elapsed > 0.0);

            if self.player.life <= 0.0 {
                println!("Game Over!");
                println!("Score: {}", self.score);
                ctx.request_quit();
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);

        let coords = (self.screen_width, self.screen_height);

        draw_ground(&mut canvas, coords);

        draw_cursor(&mut canvas, &self.player, coords);

        for rocket in &self.rockets {
            draw_rocket(&mut canvas, ctx, rocket, coords, self.level);
        }

        for interceptor in &self.interceptors {
            draw_interceptor(&mut canvas, ctx, interceptor, coords);
        }

        draw_healthbar(&mut canvas, &self.player, coords.1);

        canvas.draw(
            &graphics::Text::new(format!("Score: {}", self.score)),
            graphics::DrawParam::new()
                .dest(Vec2::new(self.screen_width / 2.0, 10.0))
                .color(Color::WHITE),
        );

        canvas.draw(
            &graphics::Text::new(format!("Level: {}", self.level)),
            graphics::DrawParam::new()
                .dest(Vec2::new(20.0, 10.0))
                .color(Color::WHITE),
        );

        canvas.finish(ctx)?;

        timer::yield_now();
        Ok(())
    }

    // input handler keydown adapted from ggez example
    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: KeyInput,
        _repeated: bool,
    ) -> GameResult {
        match input.keycode {
            Some(KeyCode::Up) => {
                self.input.yaxis = 1.0;
            }
            Some(KeyCode::Down) => {
                self.input.yaxis = -1.0;
            }
            Some(KeyCode::Left) => {
                self.input.xaxis = -1.0;
            }
            Some(KeyCode::Right) => {
                self.input.xaxis = 1.0;
            }
            Some(KeyCode::Space) => {
                self.input.fire = true;
            }
            Some(KeyCode::Escape) => ctx.request_quit(),
            _ => (),
        }
        Ok(())
    }

    // input handler keyup adapted from ggez example
    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        match input.keycode {
            Some(KeyCode::Up | KeyCode::Down) => {
                self.input.yaxis = 0.0;
            }
            Some(KeyCode::Left | KeyCode::Right) => {
                self.input.xaxis = 0.0;
            }
            Some(KeyCode::Space) => {
                self.input.fire = false;
            }
            _ => (),
        }
        Ok(())
    }
}

pub fn main() -> GameResult {
    let cb = ContextBuilder::new("rust_command", "Reid Luttrell")
        .window_setup(conf::WindowSetup::default().title("rust_command"))
        .window_mode(conf::WindowMode::default().dimensions(1280.0, 760.0));

    let (mut ctx, events_loop) = cb.build()?;

    let game = MainState::new(&mut ctx)?;
    event::run(ctx, events_loop, game)
}
