use std::io::stdout;

use time::{format_description, UtcOffset};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, time::OffsetTime},
    layer::SubscriberExt,
};

pub fn init() -> (WorkerGuard, WorkerGuard) {
    std::env::set_var("RUST_BACKTRACE", "1");

    let format = format_description::parse("[year][month][day]_[hour][minute][second]").unwrap();
    let now = time::OffsetDateTime::now_local()
        .unwrap()
        .format(&format)
        .unwrap();
    let log_file_name = format!("log_{}.log", now);
    // exe folder
    let exec_path = std::env::current_exe().unwrap();
    let exec_folder = exec_path.parent().unwrap().to_str().unwrap();
    println!(
        "log file path: {}/{}",
        exec_folder.replace("\\", "/"),
        log_file_name
    );
    let log_file = tracing_appender::rolling::never(exec_folder, log_file_name);
    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(log_file);
    let (non_blocking_stdout, _stdout_guard) = tracing_appender::non_blocking(stdout());

    let offset = UtcOffset::from_hms(8, 0, 0).expect("should get UTC+8 offset!");
    let timer = OffsetTime::new(offset, time::format_description::well_known::Rfc3339);

    tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .with_timer(timer.clone())
            .with_writer(non_blocking_stdout)
            .finish()
            .with(fmt::Layer::default().with_writer(non_blocking_file)),
    )
    .expect("Unable to set global tracing subscriber");

    (_stdout_guard, _file_guard)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_init() {
        println!("test 0");

        let _guards = init();

        tracing::info!("test 1");

        tracing::info!("test 2");

        // assert!(false);
    }
}
