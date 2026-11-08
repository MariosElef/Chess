/// The thread counts offered, kept to those this device has.
const THREAD_STEPS: [usize; 6] = [1, 2, 4, 8, 16, 32];
/// The default never takes more than this many threads: plenty for analysis, and quiet.
const DEFAULT_MAX_THREADS: usize = 4;
pub const HASH_CHOICES: [usize; 5] = [64, 128, 256, 512, 1024];
const DEFAULT_HASH_MB: usize = 256;
/// `None` searches until told to stop.
pub const DEPTH_CHOICES: [Option<u32>; 4] = [Some(18), Some(24), Some(30), None];
const DEFAULT_DEPTH: Option<u32> = Some(24);

/// How hard Stockfish works: chosen in Settings, with defaults that fit the device it runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EngineSettings {
    pub threads: usize,
    pub hash_mb: usize,
    /// Stop at this depth and rest until the position changes; `None` never stops.
    pub depth: Option<u32>,
}

impl EngineSettings {
    /// Half the device's cores, at most four, stopping at depth 24: strong and quiet.
    pub fn for_cores(cores: usize) -> Self {
        Self {
            threads: (cores / 2).clamp(1, DEFAULT_MAX_THREADS),
            hash_mb: DEFAULT_HASH_MB,
            depth: DEFAULT_DEPTH,
        }
    }

    /// Settings saved as `threads=4 hash=256 depth=24` (or `depth=infinite`); anything missing
    /// or unreadable keeps the device default, and threads never exceed the device's cores.
    pub fn from_text(text: &str, cores: usize) -> Self {
        let mut settings = Self::for_cores(cores);
        for (key, value) in text
            .split_whitespace()
            .filter_map(|pair| pair.split_once('='))
        {
            settings.apply_saved(key, value);
        }
        settings.threads = settings.threads.clamp(1, cores.max(1));
        settings
    }

    pub fn to_text(self) -> String {
        format!(
            "threads={} hash={} depth={}",
            self.threads,
            self.hash_mb,
            depth_key(self.depth)
        )
    }

    fn apply_saved(&mut self, key: &str, value: &str) {
        match key {
            "threads" => self.threads = value.parse().unwrap_or(self.threads),
            "hash" => self.hash_mb = value.parse().unwrap_or(self.hash_mb),
            "depth" => self.depth = parse_depth(value).unwrap_or(self.depth),
            _ => {}
        }
    }
}

/// The thread counts to offer on a device with `cores` cores: the powers of two below it, then all.
pub fn thread_choices(cores: usize) -> Vec<usize> {
    let mut choices: Vec<usize> = THREAD_STEPS
        .into_iter()
        .filter(|&threads| threads < cores)
        .collect();
    choices.push(cores.max(1));
    choices
}

/// `24`, or `Infinite` for no limit, as the settings page shows it.
pub fn depth_label(depth: Option<u32>) -> String {
    depth.map_or_else(|| "Infinite".to_owned(), |depth| depth.to_string())
}

fn depth_key(depth: Option<u32>) -> String {
    depth.map_or_else(|| "infinite".to_owned(), |depth| depth.to_string())
}

/// `Some(None)` for infinite, `Some(Some(n))` for a depth, `None` when unreadable.
fn parse_depth(value: &str) -> Option<Option<u32>> {
    if value == "infinite" {
        Some(None)
    } else {
        value.parse().ok().map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::{EngineSettings, thread_choices};

    #[test]
    fn defaults_fit_the_device() {
        assert_eq!(EngineSettings::for_cores(18).threads, 4);
        assert_eq!(EngineSettings::for_cores(4).threads, 2);
        assert_eq!(EngineSettings::for_cores(1).threads, 1);
        assert_eq!(EngineSettings::for_cores(18).depth, Some(24));
    }

    #[test]
    fn thread_choices_end_with_every_core() {
        assert_eq!(thread_choices(18), [1, 2, 4, 8, 16, 18]);
        assert_eq!(thread_choices(4), [1, 2, 4]);
        assert_eq!(thread_choices(1), [1]);
    }

    #[test]
    fn saved_settings_read_back() {
        let settings = EngineSettings {
            threads: 8,
            hash_mb: 512,
            depth: None,
        };
        assert_eq!(EngineSettings::from_text(&settings.to_text(), 18), settings);
    }

    #[test]
    fn a_settings_file_from_a_bigger_machine_is_capped_to_this_one() {
        let settings = EngineSettings::from_text("threads=16 hash=128 depth=30", 4);
        assert_eq!(settings.threads, 4);
        assert_eq!(settings.hash_mb, 128);
        assert_eq!(settings.depth, Some(30));
    }

    #[test]
    fn unreadable_values_keep_the_defaults() {
        assert_eq!(
            EngineSettings::from_text("threads=lots depth=deep", 18),
            EngineSettings::for_cores(18)
        );
    }
}
