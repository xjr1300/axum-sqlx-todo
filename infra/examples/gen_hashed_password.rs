use secrecy::{ExposeSecret as _, SecretString};

use domain::models::RawPassword;
use infra::{password::create_hashed_password, settings::load_app_settings};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        anyhow::bail!("Usage: gen_hashed_password <password>");
    }
    let app_settings = load_app_settings("app_settings.toml").expect("Failed to load app settings");
    let raw_password = RawPassword(SecretString::new(args[1].as_str().into()));
    let hashed_password = create_hashed_password(&app_settings.password, &raw_password)?;
    println!("{}", hashed_password.0.expose_secret());
    Ok(())
}
