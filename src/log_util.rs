use std::{io::stdout, path::Path};

use time::{format_description, UtcOffset};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, time::OffsetTime},
    layer::SubscriberExt,
};

pub fn init(project_path: &Path) -> (WorkerGuard, WorkerGuard) {
    std::env::set_var("RUST_BACKTRACE", "1");
    let time_zone_offset = UtcOffset::from_hms(8, 0, 0).expect("should get UTC+8 offset!");

    let format = format_description::parse("[year][month][day]_[hour][minute][second]").unwrap();
    let now = time::OffsetDateTime::now_local()
        .unwrap_or(time::OffsetDateTime::now_utc().to_offset(time_zone_offset))
        .format(&format)
        .unwrap();
    let log_file_name = format!("log_{}.log", now);
    let exec_folder = project_path.to_str().unwrap();
    println!(
        "log file path: {}/{}",
        exec_folder.replace("\\", "/"),
        log_file_name
    );
    let log_file = tracing_appender::rolling::never(exec_folder, log_file_name);
    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(log_file);
    let (non_blocking_stdout, _stdout_guard) = tracing_appender::non_blocking(stdout());

    let timer = OffsetTime::new(
        time_zone_offset,
        time::format_description::well_known::Rfc3339,
    );

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

        let _guards = init(std::env::current_dir().unwrap().as_path());

        tracing::info!("test 1");

        tracing::info!("test 2");

        // assert!(false);
    }
}
