use anyhow::Context as _;
use config::Config;

use app::settings::AppSettings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::builder()
        .add_source(config::File::with_name("app_settings.toml"))
        .build()
        .context("Failed to read the app_settings.toml file")?;
    let app_settings: AppSettings = config
        .try_deserialize()
        .context("The contents of the app_settings.toml file is incorrect")?;
    println!("Config: {:?}", app_settings);

    Ok(())
}
