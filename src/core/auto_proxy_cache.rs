//! Automatic Proxy Resolution Ladder, Predictive Lookahead Prefetch, and LRU Cache Budget Manager.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProxyTier {
    Full,    // 1.0x
    Half,    // 0.5x
    Quarter, // 0.25x
    Eighth,  // 0.125x
}

impl ProxyTier {
    #[inline]
    pub fn scale(self) -> f32 {
        match self {
            ProxyTier::Full => 1.0,
            ProxyTier::Half => 0.5,
            ProxyTier::Quarter => 0.25,
            ProxyTier::Eighth => 0.125,
        }
    }

    pub fn step_down(self) -> Self {
        match self {
            ProxyTier::Full => ProxyTier::Half,
            ProxyTier::Half => ProxyTier::Quarter,
            ProxyTier::Quarter | ProxyTier::Eighth => ProxyTier::Eighth,
        }
    }

    pub fn step_up(self) -> Self {
        match self {
            ProxyTier::Eighth => ProxyTier::Quarter,
            ProxyTier::Quarter => ProxyTier::Half,
            ProxyTier::Half | ProxyTier::Full => ProxyTier::Full,
        }
    }
}

/// Hysteresis-guarded adaptive proxy selector that keeps real-time playback within frame budget.
#[derive(Debug, Clone)]
pub struct AutoProxyController {
    pub current_tier: ProxyTier,
    pub target_fps: f32,
    slow_streak: u32,
    fast_streak: u32,
}

impl AutoProxyController {
    pub fn new(target_fps: f32) -> Self {
        Self {
            current_tier: ProxyTier::Full,
            target_fps: target_fps.max(1.0),
            slow_streak: 0,
            fast_streak: 0,
        }
    }

    /// Feeds the latest frame render duration (`render_ms`) and returns the updated [`ProxyTier`].
    pub fn record_frame_time(&mut self, render_ms: f32) -> ProxyTier {
        let budget_ms = 1000.0 / self.target_fps;
        if render_ms > budget_ms * 1.15 {
            self.slow_streak += 1;
            self.fast_streak = 0;
            if self.slow_streak >= 3 {
                self.current_tier = self.current_tier.step_down();
                self.slow_streak = 0;
            }
        } else if render_ms < budget_ms * 0.45 {
            self.fast_streak += 1;
            self.slow_streak = 0;
            if self.fast_streak >= 12 {
                self.current_tier = self.current_tier.step_up();
                self.fast_streak = 0;
            }
        } else {
            self.slow_streak = 0;
            self.fast_streak = 0;
        }
        self.current_tier
    }
}

/// Plans uncached lookahead frames in playback direction (`+1` forward, `-1` backward).
pub fn plan_prefetch_frames(
    playhead: u32,
    direction: i32,
    total_frames: u32,
    lookahead_count: u32,
    cached: &HashSet<u32>,
) -> Vec<u32> {
    let dir = if direction < 0 { -1_i64 } else { 1_i64 };
    let mut plan = Vec::new();
    for step in 1..=lookahead_count {
        let f = playhead as i64 + dir * step as i64;
        if f < 0 || f >= total_frames as i64 {
            break;
        }
        let fu = f as u32;
        if !cached.contains(&fu) {
            plan.push(fu);
        }
    }
    plan
}

/// Budget-aware LRU frame cache tracking byte usage and evicting oldest entries automatically.
#[derive(Debug)]
pub struct AutoLruFrameCache {
    pub max_bytes: usize,
    pub used_bytes: usize,
    tick: u64,
    entries: HashMap<u32, (Vec<u8>, u64)>,
}

impl AutoLruFrameCache {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            used_bytes: 0,
            tick: 0,
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, frame: u32, data: Vec<u8>) {
        let size = data.len();
        if size > self.max_bytes {
            return;
        }
        if let Some((old, _)) = self.entries.remove(&frame) {
            self.used_bytes = self.used_bytes.saturating_sub(old.len());
        }
        while self.used_bytes + size > self.max_bytes {
            let Some((&oldest_k, _)) = self.entries.iter().min_by_key(|(_, (_, t))| *t) else {
                break;
            };
            if let Some((evicted, _)) = self.entries.remove(&oldest_k) {
                self.used_bytes = self.used_bytes.saturating_sub(evicted.len());
            }
        }
        self.tick += 1;
        self.used_bytes += size;
        self.entries.insert(frame, (data, self.tick));
    }

    pub fn get(&mut self, frame: u32) -> Option<&[u8]> {
        self.tick += 1;
        let t = self.tick;
        let entry = self.entries.get_mut(&frame)?;
        entry.1 = t;
        Some(&entry.0)
    }

    pub fn cached_frames(&self) -> HashSet<u32> {
        self.entries.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_proxy_steps_down_on_overbudget_and_recovers_with_hysteresis() {
        let mut ctrl = AutoProxyController::new(60.0); // budget 16.67ms
        assert_eq!(ctrl.current_tier, ProxyTier::Full);
        for _ in 0..3 {
            ctrl.record_frame_time(25.0);
        }
        assert_eq!(ctrl.current_tier, ProxyTier::Half);
        for _ in 0..12 {
            ctrl.record_frame_time(4.0);
        }
        assert_eq!(ctrl.current_tier, ProxyTier::Full);
    }

    #[test]
    fn auto_lru_cache_and_prefetch_planner_work_together() {
        let mut cache = AutoLruFrameCache::new(10);
        cache.insert(1, vec![1; 4]);
        cache.insert(2, vec![2; 4]);
        // Touch frame 1 so frame 2 becomes LRU
        assert!(cache.get(1).is_some());
        cache.insert(3, vec![3; 4]);
        assert!(cache.get(2).is_none());
        assert!(cache.get(1).is_some());

        let plan = plan_prefetch_frames(0, 1, 10, 4, &cache.cached_frames());
        assert_eq!(plan, vec![2, 4]);
    }
}
