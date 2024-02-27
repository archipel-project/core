mod app;
mod networking;

use app::App;

fn main() -> anyhow::Result<()> {
    App::new()?.run()
}
