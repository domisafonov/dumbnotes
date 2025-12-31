// adapted from Rocket-0.5.1's rocket::execute

use boolean_enums::gen_boolean_enum;
use figment::Figment;

pub fn execute<R, F>(
    figment: Figment,
    future: impl FnOnce(Figment) -> F,
) -> R
    where F: Future<Output = R> + Send
{
    async_main(figment, future)
}

macro_rules! config_bail {
    ($e:ident) => { crate::error_exit!("failed to load config: {}", $e) };
}

fn async_main<R, F>(
    figment: Figment,
    fut: impl FnOnce(Figment) -> F,
) -> R
    where F: Future<Output = R> + Send
{
    use rocket::Config;
    let workers: usize = figment.extract_inner(Config::WORKERS).unwrap_or_else(|e| config_bail!(e));
    let max_blocking: usize = figment.extract_inner(Config::MAX_BLOCKING).unwrap_or_else(|e| config_bail!(e));
    let force: ForceEnd = figment.focus(Config::SHUTDOWN).extract_inner::<bool>("force").unwrap_or_else(|e| config_bail!(e)).into();
    async_run(fut(figment), workers, max_blocking, force, "rocket-worker-thread")
}

fn async_run<F, R>(
    fut: F,
    workers: usize,
    sync: usize,
    force_end: ForceEnd,
    name: &str,
) -> R
    where F: Future<Output = R>
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name(name)
        .worker_threads(workers)
        .max_blocking_threads(sync)
        .enable_all()
        .build()
        .expect("create tokio runtime");

    let result = runtime.block_on(fut);
    if force_end.into() {
        runtime.shutdown_timeout(std::time::Duration::from_millis(500));
    }

    result
}
gen_boolean_enum!(ForceEnd);
