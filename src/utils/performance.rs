use std::time::Duration;
use std::collections::VecDeque;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref PERFORMANCE_MONITOR: Mutex<PerformanceMonitor> = Mutex::new(PerformanceMonitor::new(100));
}

pub struct PerformanceMonitor {
    command_timings: VecDeque<(String, Duration)>,
    max_samples: usize,
}

impl PerformanceMonitor {
    pub fn new(max_samples: usize) -> Self {
        PerformanceMonitor {
            command_timings: VecDeque::new(),
            max_samples,
        }
    }

    pub fn record_execution(&mut self, command: &str, duration: Duration) {
        self.command_timings.push_back((command.to_string(), duration));
        if self.command_timings.len() > self.max_samples {
            self.command_timings.pop_front();
        }
    }

    pub fn get_average_duration(&self) -> Duration {
        if self.command_timings.is_empty() {
            return Duration::from_secs(0);
        }
        
        let total = self.command_timings
            .iter()
            .map(|(_, duration)| duration.as_millis())
            .sum::<u128>();
            
        Duration::from_millis((total / self.command_timings.len() as u128) as u64)
    }
}