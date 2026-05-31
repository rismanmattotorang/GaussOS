// src/performance/metrics.rs
//! Convenient wrappers around the `metrics` crate so that callers can emit counters & histograms
//! without depending on the crate directly (and compile even when the `metrics` feature is off).

/// Increment a named counter by 1.
#[inline]
pub fn incr(name: &'static str) {
    #[cfg(feature = "metrics")]
    {
        use metrics::counter;
        counter!(name).increment(1);
    }
    #[cfg(not(feature = "metrics"))]
    let _ = name;
}

/// Increment counter by arbitrary value.
#[inline]
pub fn add(name: &'static str, value: u64) {
    #[cfg(feature = "metrics")]
    {
        use metrics::counter;
        counter!(name).increment(value);
    }
    #[cfg(not(feature = "metrics"))]
    {
        let _ = (name, value);
    }
}

/// Record a histogram sample (f64 value in e.g. milliseconds).
#[inline]
pub fn record(name: &'static str, value: f64) {
    #[cfg(feature = "metrics")]
    {
        use metrics::histogram;
        histogram!(name).record(value);
    }
    #[cfg(not(feature = "metrics"))]
    {
        let _ = (name, value);
    }
}
