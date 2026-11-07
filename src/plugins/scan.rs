//! Rescan). On Android there is no scanner process: CLAP files are scanned inside the app.

use std::collections::{HashMap, HashSet};
#[cfg(not(target_os = "android"))]
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(not(target_os = "android"))]
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
#[cfg(not(target_os = "android"))]
use std::time::{Duration, Instant};

use os_host::{Descriptor, Format, Kind, Library};
use serde::{Deserialize, Serialize};

/// The scanner's command-line switch: `honeydew-native --scan-plugin <format> <path>`.
pub const ARG: &str = "--scan-plugin";
/// The scanner's one line on standard output starts with this.
#[cfg(not(target_os = "android"))]
const TAG: &str = "HDSCAN ";
/// A scanner still running after this is ended (status `timed out`).
#[cfg(not(target_os = "android"))]
const TIMEOUT: Duration = Duration::from_secs(30);
/// Scanners running at the same time. Android scans one file at a time, inside the app.
const WORKERS: usize = if cfg!(target_os = "android") { 1 } else { 4 };
/// How deep below a search folder plugin files are looked for.
const MAX_DEPTH: u32 = 8;

/// Android: the folders searched, set once by the app at start-up (see [`set_android_folders`]).
#[cfg(target_os = "android")]
static ANDROID_FOLDERS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
