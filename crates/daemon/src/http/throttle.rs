//! A budget for getting the key wrong. Only the failure path is counted, so a
//! busy operator is never throttled by their own dashboard. The window is fixed
//! rather than sliding — the point is a ceiling on guesses per minute.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Rejections one address may collect before it has to wait.
const BUDGET: u32 = 10;

/// How long a budget lasts, and how long an exhausted one takes to refill.
const WINDOW: Duration = Duration::from_secs(60);

/// How many addresses are tracked at once. Bounded because the key is
/// attacker-chosen; full, the oldest window is evicted.
const TRACKED: usize = 4096;

#[derive(Default)]
pub struct Throttle {
    windows: Mutex<HashMap<IpAddr, Window>>,
}

struct Window {
    started: Instant,
    rejections: u32,
}

impl Throttle {
    /// Seconds this address must wait, or `None` while it still has budget.
    pub fn blocked_for(&self, address: IpAddr) -> Option<u32> {
        let mut windows = self.windows.lock().unwrap();
        let window = windows.get(&address)?;
        if window.started.elapsed() >= WINDOW {
            windows.remove(&address);
            return None;
        }
        if window.rejections < BUDGET {
            return None;
        }
        let remaining = WINDOW.saturating_sub(window.started.elapsed());
        Some(remaining.as_secs().max(1) as u32)
    }

    /// Record a rejected key.
    pub fn rejected(&self, address: IpAddr) {
        let mut windows = self.windows.lock().unwrap();
        windows.retain(|_, window| window.started.elapsed() < WINDOW);
        if windows.len() >= TRACKED && !windows.contains_key(&address) {
            let oldest = windows
                .iter()
                .min_by_key(|(_, window)| window.started)
                .map(|(ip, _)| *ip);
            if let Some(oldest) = oldest {
                windows.remove(&oldest);
            }
        }
        let window = windows.entry(address).or_insert(Window {
            started: Instant::now(),
            rejections: 0,
        });
        window.rejections += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(last: u8) -> IpAddr {
        IpAddr::from([203, 0, 113, last])
    }

    #[test]
    fn an_address_with_budget_is_never_blocked() {
        let throttle = Throttle::default();
        assert_eq!(throttle.blocked_for(address(1)), None);
        for _ in 0..BUDGET - 1 {
            throttle.rejected(address(1));
        }
        assert_eq!(throttle.blocked_for(address(1)), None);
    }

    #[test]
    fn spending_the_budget_buys_a_wait() {
        let throttle = Throttle::default();
        for _ in 0..BUDGET {
            throttle.rejected(address(1));
        }
        let wait = throttle.blocked_for(address(1)).expect("blocked");
        assert!(wait > 0 && wait <= WINDOW.as_secs() as u32);
    }

    #[test]
    fn one_addresss_guessing_never_locks_out_another() {
        let throttle = Throttle::default();
        for _ in 0..BUDGET * 3 {
            throttle.rejected(address(1));
        }
        assert!(throttle.blocked_for(address(1)).is_some());
        assert_eq!(throttle.blocked_for(address(2)), None);
    }

    #[test]
    fn the_table_never_grows_past_what_it_tracks() {
        let throttle = Throttle::default();
        for n in 0..(TRACKED + 500) {
            throttle.rejected(IpAddr::from((n as u32).to_be_bytes()));
        }
        assert!(throttle.windows.lock().unwrap().len() <= TRACKED);
    }
}
