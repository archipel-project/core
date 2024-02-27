mod app;
mod networking;
mod graphic;
use app::App;

fn main() -> anyhow::Result<()> {
    let (app, event_loop) = App::new()?;
    app.run(event_loop)
}
