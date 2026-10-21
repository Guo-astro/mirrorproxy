use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};

use reqwest::Url;
use serde::Serialize;

use crate::config::{Config, UpstreamSelectionConfig};

const MAX_ENDPOINTS: usize = 512;

#[derive(Default)]
struct EndpointState {
    consecutive_failures: u32,
    open_until: Option<Instant>,
    latency_ewma_ms: Option<f64>,
    last_seen: Option<Instant>,
}

#[derive(Default)]
pub struct UpstreamSelector {
    state: Mutex<SelectorState>,
}

#[derive(Default)]
struct SelectorState {
    endpoints: HashMap<String, EndpointState>,
    samples: VecDeque<RequestSample>,
    bases: Vec<Url>,
}

struct RequestSample {
    candidates: Vec<Url>,
    observed_at: i64,
}

#[derive(Serialize)]
pub struct SelectionReport {
    strategy: String,
    samples: Vec<SelectionSample>,
}

#[derive(Serialize)]
struct SelectionSample {
    observed_at: i64,
    endpoints: Vec<EndpointExplanation>,
}

#[derive(Serialize)]
struct EndpointExplanation {
    origin: String,
    configured_position: usize,
    rank: usize,
    reason: &'static str,
    consecutive_failures: u32,
    cooldown_remaining_secs: u64,
    latency_ewma_ms: Option<f64>,
}

impl UpstreamSelector {
    pub fn configured(config: &Config) -> Self {
        let selector = Self::default();
        selector.configure(config);
        selector
    }

    pub fn configure(&self, config: &Config) {
        let mut bases: Vec<_> = config
            .upstream_groups()
            .flat_map(|group| group.split(','))
            .filter_map(|base| Url::parse(base.trim()).ok())
            .collect();
        bases.sort_by_key(|base| std::cmp::Reverse(base.path().len()));
        let mut state = self.state.lock().expect("upstream selector lock poisoned");
        state.bases = bases;
        state.endpoints.clear();
        state.samples.clear();
    }

    pub fn rank(&self, candidates: Vec<Url>, config: &UpstreamSelectionConfig) -> Vec<Url> {
        let now = Instant::now();
        let mut state = self.state.lock().expect("upstream selector lock poisoned");
        // Retain one recent request per origin group, bounded independently of
        // request volume. Full URLs stay private; reports expose origins only.
        if !candidates.is_empty() {
            state.samples.retain(|sample| {
                !sample
                    .candidates
                    .iter()
                    .map(Url::origin)
                    .eq(candidates.iter().map(Url::origin))
            });
            state.samples.push_front(RequestSample {
                candidates: candidates.clone(),
                observed_at: chrono::Utc::now().timestamp(),
            });
            state.samples.truncate(32);
        }
        ranked_candidates(candidates, &state, config, now)
            .into_iter()
            .map(|(_, url)| url)
            .collect()
    }

    pub fn report(&self, config: &UpstreamSelectionConfig) -> SelectionReport {
        self.report_at(config, Instant::now())
    }

    fn report_at(&self, config: &UpstreamSelectionConfig, now: Instant) -> SelectionReport {
        let state = self.state.lock().expect("upstream selector lock poisoned");
        SelectionReport {
            strategy: config.strategy.clone(),
            samples: state
                .samples
                .iter()
                .map(|sample| SelectionSample {
                    observed_at: sample.observed_at,
                    endpoints: ranked_candidates(sample.candidates.clone(), &state, config, now)
                        .into_iter()
                        .enumerate()
                        .map(|(rank, (position, url))| {
                            let endpoint = state.endpoints.get(&endpoint_key(&url, &state.bases));
                            let remaining = endpoint
                                .and_then(|item| item.open_until)
                                .and_then(|deadline| deadline.checked_duration_since(now))
                                .filter(|remaining| !remaining.is_zero());
                            let failures = endpoint.map_or(0, |item| item.consecutive_failures);
                            let latency = endpoint.and_then(|item| item.latency_ewma_ms);
                            EndpointExplanation {
                                origin: url.origin().ascii_serialization(),
                                configured_position: position + 1,
                                rank: rank + 1,
                                reason: if config.strategy != "adaptive" {
                                    "configured_order"
                                } else if remaining.is_some() {
                                    "cooldown"
                                } else if failures > 0 {
                                    "failures"
                                } else if latency.is_some() {
                                    "latency"
                                } else {
                                    "unobserved"
                                },
                                consecutive_failures: failures,
                                cooldown_remaining_secs: remaining.map_or(0, |value| {
                                    value
                                        .as_secs()
                                        .saturating_add(u64::from(value.subsec_nanos() > 0))
                                }),
                                latency_ewma_ms: latency,
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn record_success(&self, url: &Url, elapsed: Duration) {
        let mut state = self.state.lock().expect("upstream selector lock poisoned");
        let key = endpoint_key(url, &state.bases);
        let state = endpoint_state(&mut state, key);
        state.consecutive_failures = 0;
        state.open_until = None;
        let elapsed = elapsed.as_secs_f64() * 1_000.0;
        state.latency_ewma_ms = Some(match state.latency_ewma_ms {
            Some(previous) => previous * 0.8 + elapsed * 0.2,
            None => elapsed,
        });
    }

    pub fn record_failure(&self, url: &Url, config: &UpstreamSelectionConfig) {
        let mut state = self.state.lock().expect("upstream selector lock poisoned");
        let key = endpoint_key(url, &state.bases);
        let state = endpoint_state(&mut state, key);
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures >= config.failure_threshold {
            state.open_until = Some(Instant::now() + Duration::from_secs(config.cooldown_secs));
        }
    }
}

fn endpoint_key(url: &Url, bases: &[Url]) -> String {
    // Longest configured base wins; do not merge separate repositories on one host.
    let base = bases.iter().find(|base| {
        let path = base.path().trim_end_matches('/');
        base.origin() == url.origin()
            && (url.path() == path
                || url
                    .path()
                    .strip_prefix(path)
                    .is_some_and(|suffix| suffix.starts_with('/')))
    });
    match base {
        Some(base) => format!(
            "{}{}",
            base.origin().ascii_serialization(),
            base.path().trim_end_matches('/')
        ),
        None => url.origin().ascii_serialization(),
    }
}

fn endpoint_state(state: &mut SelectorState, key: String) -> &mut EndpointState {
    if state.endpoints.len() >= MAX_ENDPOINTS && !state.endpoints.contains_key(&key) {
        if let Some(oldest) = state
            .endpoints
            .iter()
            .min_by_key(|(_, entry)| entry.last_seen)
            .map(|(key, _)| key.clone())
        {
            state.endpoints.remove(&oldest);
        }
    }
    let entry = state.endpoints.entry(key).or_default();
    entry.last_seen = Some(Instant::now());
    entry
}

fn ranked_candidates(
    candidates: Vec<Url>,
    state: &SelectorState,
    config: &UpstreamSelectionConfig,
    now: Instant,
) -> Vec<(usize, Url)> {
    let mut ranked = candidates.into_iter().enumerate().collect::<Vec<_>>();
    if config.strategy == "adaptive" {
        ranked.sort_by(|(left_index, left), (right_index, right)| {
            endpoint_rank(
                state.endpoints.get(&endpoint_key(left, &state.bases)),
                now,
                *left_index,
            )
            .total_cmp(&endpoint_rank(
                state.endpoints.get(&endpoint_key(right, &state.bases)),
                now,
                *right_index,
            ))
        });
    }
    ranked
}

fn endpoint_rank(state: Option<&EndpointState>, now: Instant, configured_index: usize) -> f64 {
    let Some(state) = state else {
        return configured_index as f64;
    };
    let circuit_penalty = if state.open_until.is_some_and(|deadline| deadline > now) {
        1_000_000.0
    } else {
        0.0
    };
    circuit_penalty
        + f64::from(state.consecutive_failures) * 10_000.0
        + state.latency_ewma_ms.unwrap_or(configured_index as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adaptive() -> UpstreamSelectionConfig {
        UpstreamSelectionConfig {
            strategy: "adaptive".to_string(),
            failure_threshold: 2,
            cooldown_secs: 60,
        }
    }

    #[test]
    fn shares_health_across_packages_but_isolates_repository_bases() {
        let mut config = Config::default();
        config.upstreams.cpan = "https://mirror.example/cpan".into();
        config.upstreams.cran = "https://mirror.example/cran".into();
        let selector = UpstreamSelector::configured(&config);
        let first = Url::parse("https://mirror.example/cpan/a?token=one").unwrap();
        let next = Url::parse("https://mirror.example/cpan/b?token=two").unwrap();
        let other = Url::parse("https://mirror.example/cran/a").unwrap();
        selector.record_failure(&first, &adaptive());
        selector.record_failure(&first, &adaptive());
        assert_eq!(
            selector.rank(vec![next.clone(), other.clone()], &adaptive()),
            vec![other, next]
        );
        assert_eq!(selector.state.lock().unwrap().endpoints.len(), 1);
        selector.configure(&config);
        assert!(selector.state.lock().unwrap().endpoints.is_empty());
    }

    #[test]
    fn bounds_health_state_for_dynamic_hosts_and_ignores_request_secrets() {
        let selector = UpstreamSelector::default();
        for index in 0..MAX_ENDPOINTS + 20 {
            let url = Url::parse(&format!("https://host{index}.example/a?token=private")).unwrap();
            selector.record_success(&url, Duration::from_millis(1));
        }
        let state = selector.state.lock().unwrap();
        assert_eq!(state.endpoints.len(), MAX_ENDPOINTS);
        assert!(!state.endpoints.contains_key("https://host0.example"));
        assert!(state.endpoints.keys().all(|key| !key.contains("private")));
    }

    #[test]
    fn diagnostics_follow_runtime_order_and_do_not_disclose_request_urls() {
        let selector = UpstreamSelector::default();
        let secret = uuid::Uuid::new_v4().to_string();
        let one = Url::parse(&format!(
            "https://user:{secret}@one.example/private/{secret}?token={secret}#secret"
        ))
        .unwrap();
        let two = Url::parse("https://two.example/file").unwrap();
        selector.record_success(&one, Duration::from_millis(500));
        selector.record_success(&two, Duration::from_millis(20));
        let ranked = selector.rank(vec![one.clone(), two.clone()], &adaptive());
        let report = selector.report(&adaptive());
        let endpoints = &report.samples[0].endpoints;
        assert_eq!(
            endpoints[0].origin,
            ranked[0].origin().ascii_serialization()
        );
        assert_eq!(endpoints[0].configured_position, 2);
        assert_eq!(endpoints[0].reason, "latency");
        assert_eq!(endpoints[0].latency_ewma_ms, Some(20.0));
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains(&secret));
        assert!(!json.contains("private"));
        assert!(!json.contains("user:"));

        selector.record_failure(&two, &adaptive());
        selector.record_failure(&two, &adaptive());
        let report = selector.report(&adaptive());
        assert_eq!(
            report.samples[0].endpoints[0].origin,
            one.origin().ascii_serialization()
        );
        let failed = &report.samples[0].endpoints[1];
        assert_eq!(failed.reason, "cooldown");
        assert_eq!(failed.consecutive_failures, 2);
        assert!(failed.cooldown_remaining_secs > 0);

        let expired = selector.report_at(&adaptive(), Instant::now() + Duration::from_secs(61));
        assert_eq!(expired.samples[0].endpoints[1].reason, "failures");
        assert_eq!(expired.samples[0].endpoints[1].cooldown_remaining_secs, 0);
        selector.record_success(&two, Duration::from_millis(10));
        let recovered = selector.report(&adaptive());
        assert_eq!(
            recovered.samples[0].endpoints[0].origin,
            two.origin().ascii_serialization()
        );
        assert_eq!(recovered.samples[0].endpoints[0].consecutive_failures, 0);

        let ordered = selector.report(&UpstreamSelectionConfig::default());
        assert_eq!(ordered.samples[0].endpoints[0].configured_position, 1);
        assert!(ordered.samples[0]
            .endpoints
            .iter()
            .all(|item| item.reason == "configured_order"));
    }

    #[test]
    fn diagnostics_bound_samples_and_replace_paths_within_an_origin_group() {
        let selector = UpstreamSelector::default();
        for index in 0..40 {
            selector.rank(
                vec![Url::parse(&format!("https://site{index}.example/a")).unwrap()],
                &adaptive(),
            );
        }
        assert_eq!(selector.report(&adaptive()).samples.len(), 32);
        selector.rank(
            vec![Url::parse("https://site39.example/new-path?secret=value").unwrap()],
            &adaptive(),
        );
        let report = selector.report(&adaptive());
        assert_eq!(report.samples.len(), 32);
        assert_eq!(report.samples[0].endpoints[0].reason, "unobserved");
        assert_eq!(
            report.samples[0].endpoints[0].origin,
            "https://site39.example"
        );
        assert_eq!(
            UpstreamSelector::default()
                .report(&adaptive())
                .samples
                .len(),
            0
        );
    }

    #[test]
    fn ordered_mode_never_changes_explicit_priority() {
        let selector = UpstreamSelector::default();
        let one = Url::parse("https://one.example/path").unwrap();
        let two = Url::parse("https://two.example/path").unwrap();
        selector.record_failure(&one, &adaptive());
        assert_eq!(
            selector.rank(
                vec![one.clone(), two.clone()],
                &UpstreamSelectionConfig::default()
            ),
            vec![one, two]
        );
    }

    #[test]
    fn adaptive_mode_deprioritizes_failed_and_slower_endpoints() {
        let selector = UpstreamSelector::default();
        let one = Url::parse("https://one.example/path").unwrap();
        let two = Url::parse("https://two.example/path").unwrap();
        selector.record_success(&one, Duration::from_millis(500));
        selector.record_success(&two, Duration::from_millis(20));
        assert_eq!(
            selector.rank(vec![one.clone(), two.clone()], &adaptive()),
            vec![two.clone(), one.clone()]
        );
        selector.record_failure(&two, &adaptive());
        selector.record_failure(&two, &adaptive());
        assert_eq!(selector.rank(vec![two, one.clone()], &adaptive())[0], one);
    }
}
