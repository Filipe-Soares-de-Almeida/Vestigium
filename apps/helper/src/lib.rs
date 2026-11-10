//! Vestigium's elevated helper: one job from a closed list, then exit.

pub mod capture;
pub mod chkdsk;
pub mod delete;
#[cfg(test)]
mod dll_search;
pub mod dumps;
pub mod job;
mod lock;
mod output;
mod pinned;
pub mod scan;
pub mod surface;
pub mod tweak;
