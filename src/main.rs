mod app;
mod deploy;
mod logs;
mod models;
mod storage;

fn main() -> Result<(), slint::PlatformError> {
    app::run()
}
