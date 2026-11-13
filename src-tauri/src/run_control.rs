use crate::runtime::CancelSignal;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
struct Calls {
    stopped: HashSet<String>,
    signals: HashMap<String, Vec<CancelSignal>>,
    jobs: HashMap<String, usize>,
}

fn calls() -> &'static Mutex<Calls> {
    static CALLS: OnceLock<Mutex<Calls>> = OnceLock::new();
    CALLS.get_or_init(|| Mutex::new(Calls::default()))
}

pub fn register(key: &str) -> CancelSignal {
    let flag = std::sync::Arc::new(AtomicBool::new(false));
    let mut calls = calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if calls.stopped.contains(key) { flag.store(true, Ordering::SeqCst); }
    calls.signals.entry(key.to_owned()).or_default().push(flag.clone());
    flag
}

pub fn release(key: &str, flag: &CancelSignal) {
    let mut calls = calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(signals) = calls.signals.get_mut(key) {
        signals.retain(|item| !std::sync::Arc::ptr_eq(item, flag));
        if signals.is_empty() { calls.signals.remove(key); }
    }
}

pub fn stop(key: &str) {
    let mut calls = calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    calls.stopped.insert(key.to_owned());
    if let Some(signals) = calls.signals.get(key) {
        for flag in signals { flag.store(true, Ordering::SeqCst); }
    }
}

pub fn resume(key: &str) {
    calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).stopped.remove(key);
}

pub fn stopped(key: &str) -> bool {
    calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).stopped.contains(key)
}

pub fn active(key: &str) -> bool {
    let calls = calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    calls.signals.get(key).is_some_and(|signals| !signals.is_empty()) || calls.jobs.get(key).is_some_and(|count| *count > 0)
}

pub struct JobGuard(String);

pub fn begin_job(key: &str) -> JobGuard {
    *calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).jobs.entry(key.to_owned()).or_default() += 1;
    JobGuard(key.to_owned())
}

impl Drop for JobGuard {
    fn drop(&mut self) {
        let mut calls = calls().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(count) = calls.jobs.get_mut(&self.0) {
            *count -= 1;
            if *count == 0 { calls.jobs.remove(&self.0); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stops_current_and_later_calls_until_resumed() {
        let key = "run-control-unit";
        let first = register(key);
        stop(key);
        assert!(first.load(Ordering::SeqCst));
        let second = register(key);
        assert!(second.load(Ordering::SeqCst));
        release(key, &first);
        release(key, &second);
        resume(key);
        let third = register(key);
        assert!(!third.load(Ordering::SeqCst));
        release(key, &third);
    }

    #[test]
    fn active_job_prevents_cleanup_until_worker_returns() {
        let key = "run-control-job-unit";
        let job = begin_job(key);
        assert!(active(key));
        drop(job);
        assert!(!active(key));
    }
}
