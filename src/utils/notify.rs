use indicatif::{ProgressBar, ProgressStyle};
use pyo3::prelude::*;
use std::borrow::Cow;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;


static ENABLED: AtomicBool = AtomicBool::new(true);

pub enum Notifier {
    Disabled,
    Enabled(ProgressBar),
}

#[derive(FromPyObject)]
pub enum NotifyArg {
    Flag(bool),
    Message(String),
}

pub fn get() -> bool {
    ENABLED.load(Relaxed)
}

pub fn set(value: bool) {
    ENABLED.store(value, Relaxed);
}

impl Notifier {
    pub fn disabled() -> Self {
        Self::Disabled
    }

    pub fn from_arg(arg: Option<NotifyArg>, steps: usize, default: &'static str) -> Self {
        match arg {
            Some(arg) => match arg {
                NotifyArg::Flag(notify) => if notify {
                    Notifier::new(steps, default)
                } else {
                    Notifier::disabled()
                },
                NotifyArg::Message(msg) => Notifier::new(steps, msg),
            },
            None => Notifier::new(steps, default),
        }
    }

    pub fn new(steps: usize, message: impl Into<Cow<'static, str>>) -> Self {
        if (steps > 1) && ENABLED.load(Relaxed) {
            let bar = ProgressBar::new(steps as u64);
            let bar_style = ProgressStyle::with_template(
                "{msg} [{wide_bar:.dim}] {percent}%, {elapsed})"
            )
                .unwrap()
                .progress_chars("=> ");
            bar.set_style(bar_style);
            bar.set_message(message);
            bar.set_position(0);
            Self::Enabled(bar)
        } else {
            Self::Disabled
        }
    }

    pub fn tic(&self) {
        match self {
            Self::Enabled(bar) => bar.inc(1),
            Self::Disabled => (),
        }
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        match self {
            Self::Enabled(bar) => bar.finish_and_clear(),
            Self::Disabled => (),
        }
    }
}
