use std::{net::IpAddr, num::NonZeroU32, time::Duration};

use governor::{
    DefaultKeyedRateLimiter, Quota, RateLimiter,
    clock::{Clock, DefaultClock},
};

/// What a client IP is limited on.
#[derive(Clone, Copy, Debug)]
pub enum Bucket {
    Login,
    Registration,
}

/// Per-client-IP limits on the endpoints a brute force goes through. Held in
/// memory, so each replica counts on its own.
pub struct RateLimits {
    login: DefaultKeyedRateLimiter<IpAddr>,
    registration: DefaultKeyedRateLimiter<IpAddr>,
    clock: DefaultClock,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            // 10 attempts at once, then one a minute.
            login: RateLimiter::dashmap(quota(Duration::from_secs(60), 10)),
            // 5 accounts at once, then one every 10 minutes.
            registration: RateLimiter::dashmap(quota(Duration::from_secs(600), 5)),
            clock: DefaultClock::default(),
        }
    }
}

impl RateLimits {
    /// Counts one request from `ip`; on refusal, says how long to wait.
    pub fn check(&self, bucket: Bucket, ip: IpAddr) -> Result<(), Duration> {
        let limiter = match bucket {
            Bucket::Login => &self.login,
            Bucket::Registration => &self.registration,
        };
        limiter
            .check_key(&ip)
            .map_err(|not_until| not_until.wait_time_from(self.clock.now()))
    }

    /// Forgets the clients whose limit has fully replenished, so that the
    /// maps don't grow with every address ever seen.
    pub fn forget_idle_clients(&self) {
        self.login.retain_recent();
        self.registration.retain_recent();
    }
}

fn quota(replenish_every: Duration, burst: u32) -> Quota {
    Quota::with_period(replenish_every)
        .expect("a non-zero period")
        .allow_burst(NonZeroU32::new(burst).expect("a non-zero burst"))
}
