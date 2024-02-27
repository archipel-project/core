mod app;
mod graphic;
mod networking;
use app::App;

fn main() -> anyhow::Result<()> {
    let (app, event_loop) = App::new()?;
    app.run(event_loop)
}
