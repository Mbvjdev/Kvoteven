//! Orchestration: demo mode, per-source rate limiting, in-flight dedup,
//! generation guards, and the DeepSeek balance baseline. All provider and
//! credential I/O is delegated to `ProviderBackend` and `KeyStore`; keyring
//! calls are moved off the async event loop via `spawn_blocking`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::models::{
    codex_mood, deepseek_mood, format_money, iso8601, snapshot_fresh, AppError, AppStatus,
    CodexSource, Mood, Provider, Snapshot, APP_VERSION,
};
use crate::providers::{DeepSeekBalance, PercentQuota, ProviderBackend, RealBackend, Wallet};
use crate::vault::{validate_key, KeyStore, KeyringStore};

const DEMO_SOURCE: &str = "demo_synthetic";
const DEEPSEEK_SOURCE: &str = "deepseek_balance_api";

/// Default minimum interval between attempts for a given source, including
/// failed attempts.
const DEFAULT_MIN_INTERVAL: Duration = Duration::from_secs(15);

/// Cached snapshots and the DeepSeek balance baseline. Guarded together so a
/// generation check, baseline commit and cache commit happen as one atomic
/// transaction (a concurrent credential/demo transition cannot interleave).
#[derive(Default)]
struct ServiceState {
    snapshots: HashMap<String, Snapshot>,
    baseline: HashMap<String, Decimal>,
}

/// Per-source throttle. `in_flight` maps a source key to the generation that
/// started it, so a stale in-flight guard (from before a credential/demo
/// transition) can never remove the new guard's entry.
#[derive(Default)]
struct GateState {
    last_attempt: HashMap<String, Instant>,
    in_flight: HashMap<String, u64>,
}

struct InFlightGuard<'a> {
    gate: &'a Mutex<GateState>,
    key: String,
    gen: u64,
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut g) = self.gate.lock() {
            // Only remove our own entry. After a transition the generation
            // differs and a new guard may own this key.
            if g.in_flight.get(&self.key) == Some(&self.gen) {
                g.in_flight.remove(&self.key);
            }
        }
    }
}

enum Attempt<'a> {
    Proceed(InFlightGuard<'a>),
    ReturnCached(Snapshot),
}

pub struct Service {
    demo: AtomicBool,
    backend: Arc<dyn ProviderBackend>,
    store: Arc<dyn KeyStore>,
    min_interval: Duration,
    gate: Mutex<GateState>,
    state: Mutex<ServiceState>,
    generation: AtomicU64,
}

impl Service {
    pub fn new(demo: bool) -> Self {
        Self::with_backends(
            demo,
            Arc::new(RealBackend::new()),
            Arc::new(KeyringStore::new()),
            DEFAULT_MIN_INTERVAL,
        )
    }

    fn with_backends(
        demo: bool,
        backend: Arc<dyn ProviderBackend>,
        store: Arc<dyn KeyStore>,
        min_interval: Duration,
    ) -> Self {
        Service {
            demo: AtomicBool::new(demo),
            backend,
            store,
            min_interval,
            gate: Mutex::new(GateState::default()),
            state: Mutex::new(ServiceState::default()),
            generation: AtomicU64::new(0),
        }
    }

    fn is_demo(&self) -> bool {
        self.demo.load(Ordering::Acquire)
    }

    /// Run a blocking `KeyStore` call off the event loop and map a panicked
    /// blocking task to a fixed internal error.
    async fn blocking_keyring<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&dyn KeyStore) -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || f(store.as_ref()))
            .await
            .map_err(|_| AppError::new(AppError::INTERNAL))?
    }

    pub async fn status(&self) -> Result<AppStatus, AppError> {
        if self.is_demo() {
            // Demo never probes the environment: mark everything unavailable
            // without touching the credential store or the filesystem.
            return Ok(AppStatus {
                demo: true,
                version: APP_VERSION.to_string(),
                deepseek_configured: false,
                codex_available: false,
                hermes_available: false,
                platform: platform_string(),
            });
        }
        // Degrade gracefully if the keyring is locked/unavailable: report
        // unconfigured rather than crashing. Explicit quota reads still surface
        // `keyring_unavailable`.
        let deepseek_configured = self
            .blocking_keyring(|store| store.get().map(|k| k.is_some()))
            .await
            .unwrap_or(false);
        let codex_available = self.backend.codex_available();
        let hermes_available = self.backend.hermes_available();
        Ok(AppStatus {
            demo: false,
            version: APP_VERSION.to_string(),
            deepseek_configured,
            codex_available,
            hermes_available,
            platform: platform_string(),
        })
    }

    pub async fn read_quota(
        &self,
        provider: Provider,
        codex_source: CodexSource,
    ) -> Result<Snapshot, AppError> {
        if self.is_demo() {
            return Ok(demo_snapshot(provider));
        }
        match provider {
            Provider::Deepseek => self.read_deepseek().await,
            Provider::OpenaiCodex => self.read_codex(codex_source).await,
        }
    }

    pub async fn save_key(&self, key: String) -> Result<(), AppError> {
        if self.is_demo() {
            return Err(AppError::new(AppError::DEMO_REJECTED));
        }
        validate_key(&key)?;
        let key = zeroize::Zeroizing::new(key);
        self.blocking_keyring(move |store| store.set(key.as_str()))
            .await?;
        self.invalidate_state();
        Ok(())
    }

    pub async fn delete_key(&self) -> Result<(), AppError> {
        if self.is_demo() {
            return Err(AppError::new(AppError::DEMO_REJECTED));
        }
        self.blocking_keyring(|store| store.delete()).await?;
        self.invalidate_state();
        Ok(())
    }

    pub async fn set_demo(&self, demo: bool) -> Result<AppStatus, AppError> {
        let prev = self.demo.swap(demo, Ordering::AcqRel);
        if prev != demo {
            self.invalidate_state();
        }
        self.status().await
    }

    /// Bump the generation and reset all cached/baseline/throttle state so a
    /// credential or demo transition can never let old-account data resurface
    /// and the next account read is not throttled by the previous account.
    fn invalidate_state(&self) {
        // 1. Bump generation and reset the throttle under the gate lock, so a
        //    stale in-flight guard (stamped with the previous generation) can
        //    never remove the new guard's entry.
        {
            let mut g = self.gate.lock().unwrap();
            self.generation.fetch_add(1, Ordering::AcqRel);
            g.last_attempt.clear();
            g.in_flight.clear();
        }
        // 2. Clear baseline + snapshots under the state lock.
        {
            let mut s = self.state.lock().unwrap();
            s.baseline.clear();
            s.snapshots.clear();
        }
    }

    /// Per-source gate: enforce the minimum interval and in-flight dedup. A
    /// cached snapshot is only served if it is still fresh/reset-valid.
    fn gate(&self, key: &str) -> Result<Attempt<'_>, AppError> {
        let cached = {
            let s = self.state.lock().unwrap();
            s.snapshots.get(key).cloned()
        };
        let mut g = self.gate.lock().unwrap();
        let now = Instant::now();
        let serve_cached = || -> Result<Attempt<'_>, AppError> {
            match cached.clone().filter(snapshot_fresh) {
                Some(s) => Ok(Attempt::ReturnCached(s)),
                None => Err(AppError::new(AppError::BUSY)),
            }
        };
        if let Some(t) = g.last_attempt.get(key) {
            if now.duration_since(*t) < self.min_interval {
                return serve_cached();
            }
        }
        if g.in_flight.contains_key(key) {
            return serve_cached();
        }
        let gen = self.generation.load(Ordering::Acquire);
        g.in_flight.insert(key.to_string(), gen);
        g.last_attempt.insert(key.to_string(), now);
        drop(g);
        Ok(Attempt::Proceed(InFlightGuard {
            gate: &self.gate,
            key: key.to_string(),
            gen,
        }))
    }

    async fn read_deepseek(&self) -> Result<Snapshot, AppError> {
        let guard = match self.gate("deepseek")? {
            Attempt::ReturnCached(s) => return Ok(s),
            Attempt::Proceed(g) => g,
        };
        let gen = guard.gen;
        let key = self
            .blocking_keyring(|store| store.get())
            .await?
            .ok_or_else(|| AppError::new(AppError::NOT_CONFIGURED))?;
        let key = zeroize::Zeroizing::new(key);
        let result = self.backend.read_deepseek(key.as_str()).await;
        drop(guard);

        let balance = match result {
            Ok(b) => b,
            Err(e) => {
                // Invalidate the cached snapshot on a failed read so a retry
                // inside the interval cannot serve stale healthy data.
                let mut s = self.state.lock().unwrap();
                if self.generation.load(Ordering::Acquire) == gen {
                    s.snapshots.remove("deepseek");
                }
                return Err(e);
            }
        };

        // Atomic commit: generation check + baseline + cache under one lock.
        let mut s = self.state.lock().unwrap();
        if self.generation.load(Ordering::Acquire) != gen {
            let cached = s.snapshots.get("deepseek").cloned();
            drop(s);
            return cached
                .filter(snapshot_fresh)
                .ok_or_else(|| AppError::new(AppError::BUSY));
        }
        match build_deepseek_snapshot(&mut s.baseline, balance) {
            Ok(snapshot) => {
                s.snapshots.insert("deepseek".to_string(), snapshot.clone());
                Ok(snapshot)
            }
            Err(e) => {
                s.snapshots.remove("deepseek");
                Err(e)
            }
        }
    }

    async fn read_codex(&self, source: CodexSource) -> Result<Snapshot, AppError> {
        // Resolve `auto` once and throttle on the *resolved* source, so `auto`
        // and the explicit backend share the same 15-second interval instead of
        // bypassing it via different keys. Never switch backend after an
        // auth/network error.
        let resolved = match source {
            CodexSource::Auto => {
                if self.backend.codex_available() {
                    CodexSource::Codex
                } else if self.backend.hermes_available() {
                    CodexSource::Hermes
                } else {
                    return Err(AppError::new(AppError::CODEX_UNAVAILABLE));
                }
            }
            other => other,
        };
        let key = format!("codex:{}", resolved.as_str());
        let guard = match self.gate(&key)? {
            Attempt::ReturnCached(s) => return Ok(s),
            Attempt::Proceed(g) => g,
        };
        let gen = guard.gen;

        let result = match resolved {
            CodexSource::Codex => {
                if !self.backend.codex_available() {
                    return Err(AppError::new(AppError::CODEX_UNAVAILABLE));
                }
                self.backend.read_codex().await
            }
            CodexSource::Hermes => {
                if !self.backend.hermes_available() {
                    return Err(AppError::new(AppError::HERMES_UNAVAILABLE));
                }
                self.backend.read_hermes().await
            }
            CodexSource::Auto => unreachable!("resolved above"),
        };
        drop(guard);

        let quota = match result {
            Ok(q) => q,
            Err(e) => {
                let mut s = self.state.lock().unwrap();
                if self.generation.load(Ordering::Acquire) == gen {
                    s.snapshots.remove(&key);
                }
                return Err(e);
            }
        };

        let mut s = self.state.lock().unwrap();
        if self.generation.load(Ordering::Acquire) != gen {
            let cached = s.snapshots.get(&key).cloned();
            drop(s);
            return cached
                .filter(snapshot_fresh)
                .ok_or_else(|| AppError::new(AppError::BUSY));
        }
        let snapshot = build_percent_snapshot(Provider::OpenaiCodex, quota);
        s.snapshots.insert(key.clone(), snapshot.clone());
        Ok(snapshot)
    }
}

fn platform_string() -> String {
    std::env::consts::OS.to_string()
}

fn pick_preferred_wallet(wallets: &[Wallet]) -> Option<Wallet> {
    wallets
        .iter()
        .find(|w| w.currency == "USD")
        .cloned()
        .or_else(|| wallets.first().cloned())
}

/// Build a DeepSeek snapshot and update the per-currency baseline under the
/// caller's state lock. Money subtraction uses `checked_sub` so extreme-but-
/// valid wallet values cannot overflow (which would panic/abort); on overflow
/// a sanitized bad-response error is returned instead.
fn build_deepseek_snapshot(
    baseline: &mut HashMap<String, Decimal>,
    balance: DeepSeekBalance,
) -> Result<Snapshot, AppError> {
    let preferred = pick_preferred_wallet(&balance.wallets);

    // Baseline: first sight of each currency per process/key. Net change is
    // current minus that first value, never a billing claim.
    let preferred_first = preferred
        .as_ref()
        .and_then(|w| baseline.get(&w.currency).copied());
    for w in &balance.wallets {
        baseline.entry(w.currency.clone()).or_insert(w.total);
    }
    let net_change = match (&preferred, preferred_first) {
        (Some(w), Some(first)) => Some(
            w.total
                .checked_sub(first)
                .ok_or_else(|| AppError::new(AppError::DEEPSEEK_BAD_RESPONSE))?,
        ),
        _ => None,
    };

    let (currency, balance_str, mood) = match &preferred {
        Some(w) => {
            let mood = if balance.is_available {
                deepseek_mood(w.total)
            } else {
                Mood::Asleep
            };
            (Some(w.currency.clone()), Some(format_money(w.total)), mood)
        }
        None => (None, None, Mood::Unknown),
    };

    Ok(Snapshot {
        provider: Provider::Deepseek,
        source: DEEPSEEK_SOURCE.to_string(),
        fetched_at: iso8601(Utc::now()),
        mood,
        remaining_percent: None,
        resets_at: None,
        currency,
        balance: balance_str,
        net_change: net_change.map(format_money),
        is_available: Some(balance.is_available),
    })
}

fn build_percent_snapshot(provider: Provider, quota: PercentQuota) -> Snapshot {
    let now = Utc::now();
    let remaining = quota.used_percent.map(|u| 100.0 - u);
    let fresh = percent_fresh(quota.fetched_at, now);
    let reset_passed = quota.resets_at.is_some_and(|dt| dt <= now);
    let usable = fresh && !reset_passed && remaining.is_some();
    let (remaining_percent, mood) = if usable {
        let r = remaining.expect("guarded by usable");
        (Some(r), codex_mood(r))
    } else {
        (None, Mood::Unknown)
    };

    Snapshot {
        provider,
        source: quota.source,
        fetched_at: iso8601(quota.fetched_at),
        mood,
        remaining_percent,
        resets_at: quota.resets_at.map(iso8601),
        currency: None,
        balance: None,
        net_change: None,
        is_available: None,
    }
}

fn percent_fresh(fetched_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    let age = now.signed_duration_since(fetched_at).num_seconds();
    (-60i64..=600).contains(&age)
}

fn demo_snapshot(provider: Provider) -> Snapshot {
    // Demo uses the current instant so the host's freshness check (which
    // compares against real wall-clock time) accepts it; the values and the
    // `demo_synthetic` source are the fixed synthetic fixtures.
    let now = Utc::now();
    let fetched_at = iso8601(now);
    let resets_at = DateTime::from_timestamp(now.timestamp() + 6 * 86_400, 0).map(iso8601);
    match provider {
        Provider::Deepseek => Snapshot {
            provider,
            source: DEMO_SOURCE.to_string(),
            fetched_at,
            mood: Mood::Happy,
            remaining_percent: None,
            resets_at: None,
            currency: Some("USD".to_string()),
            balance: Some("42.50".to_string()),
            net_change: None,
            is_available: Some(true),
        },
        Provider::OpenaiCodex => Snapshot {
            provider,
            source: DEMO_SOURCE.to_string(),
            fetched_at,
            mood: Mood::Happy,
            remaining_percent: Some(68.0),
            resets_at,
            currency: None,
            balance: None,
            net_change: None,
            is_available: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppError, CodexSource, Mood, Provider, APP_VERSION};
    use crate::providers::{BoxFuture, DeepSeekBalance, PercentQuota, ProviderBackend, Wallet};
    use crate::vault::KeyStore;
    use rust_decimal::Decimal;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::oneshot;

    struct FakeStore {
        key: Mutex<Option<String>>,
        get_err: Mutex<Option<AppError>>,
    }
    impl FakeStore {
        fn new(key: Option<&str>) -> Self {
            FakeStore {
                key: Mutex::new(key.map(|s| s.to_string())),
                get_err: Mutex::new(None),
            }
        }
        fn fail_get_with(&self, err: AppError) {
            *self.get_err.lock().unwrap() = Some(err);
        }
    }
    impl KeyStore for FakeStore {
        fn get(&self) -> Result<Option<String>, AppError> {
            if let Some(e) = self.get_err.lock().unwrap().clone() {
                return Err(e);
            }
            Ok(self.key.lock().unwrap().clone())
        }
        fn set(&self, secret: &str) -> Result<(), AppError> {
            *self.key.lock().unwrap() = Some(secret.to_string());
            Ok(())
        }
        fn delete(&self) -> Result<(), AppError> {
            *self.key.lock().unwrap() = None;
            Ok(())
        }
    }

    struct FakeBackend {
        calls: Arc<AtomicUsize>,
        entered: Arc<AtomicUsize>,
        codex_avail: bool,
        hermes_avail: bool,
        deepseek: Arc<Mutex<VecDeque<Result<DeepSeekBalance, AppError>>>>,
        percent: Arc<Mutex<VecDeque<Result<PercentQuota, AppError>>>>,
        block: Arc<Mutex<Option<oneshot::Receiver<()>>>>,
    }
    impl FakeBackend {
        fn new(codex_avail: bool, hermes_avail: bool) -> Self {
            FakeBackend {
                calls: Arc::new(AtomicUsize::new(0)),
                entered: Arc::new(AtomicUsize::new(0)),
                codex_avail,
                hermes_avail,
                deepseek: Arc::new(Mutex::new(VecDeque::new())),
                percent: Arc::new(Mutex::new(VecDeque::new())),
                block: Arc::new(Mutex::new(None)),
            }
        }
        fn push_deepseek(&self, r: Result<DeepSeekBalance, AppError>) {
            self.deepseek.lock().unwrap().push_back(r);
        }
        fn push_percent(&self, r: Result<PercentQuota, AppError>) {
            self.percent.lock().unwrap().push_back(r);
        }
        fn set_block(&self, rx: oneshot::Receiver<()>) {
            *self.block.lock().unwrap() = Some(rx);
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
        fn entered(&self) -> usize {
            self.entered.load(Ordering::SeqCst)
        }
    }
    impl ProviderBackend for FakeBackend {
        fn read_deepseek<'a>(
            &'a self,
            _key: &'a str,
        ) -> BoxFuture<'a, Result<DeepSeekBalance, AppError>> {
            let calls = self.calls.clone();
            let entered = self.entered.clone();
            let block = self.block.clone();
            let deepseek = self.deepseek.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                entered.fetch_add(1, Ordering::SeqCst);
                let maybe_rx = { block.lock().unwrap().take() };
                if let Some(rx) = maybe_rx {
                    let _ = rx.await;
                }
                deepseek
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or(Ok(DeepSeekBalance {
                        is_available: true,
                        wallets: vec![],
                    }))
            })
        }
        fn read_codex(&self) -> BoxFuture<'static, Result<PercentQuota, AppError>> {
            let calls = self.calls.clone();
            let percent = self.percent.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                percent
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or(Err(AppError::new(AppError::CODEX_REQUEST_FAILED)))
            })
        }
        fn read_hermes(&self) -> BoxFuture<'static, Result<PercentQuota, AppError>> {
            let calls = self.calls.clone();
            let percent = self.percent.clone();
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                percent
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or(Err(AppError::new(AppError::HERMES_REQUEST_FAILED)))
            })
        }
        fn codex_available(&self) -> bool {
            self.codex_avail
        }
        fn hermes_available(&self) -> bool {
            self.hermes_avail
        }
    }

    fn wallet(currency: &str, total: &str) -> Wallet {
        Wallet {
            currency: currency.to_string(),
            total: total.parse::<Decimal>().unwrap(),
        }
    }
    fn balance(available: bool, wallets: Vec<Wallet>) -> DeepSeekBalance {
        DeepSeekBalance {
            is_available: available,
            wallets,
        }
    }
    fn percent(used: Option<f64>, fetched_now: bool, source: &str) -> PercentQuota {
        PercentQuota {
            source: source.to_string(),
            used_percent: used,
            resets_at: chrono::DateTime::from_timestamp(2_000_000_000, 0),
            fetched_at: if fetched_now {
                chrono::Utc::now()
            } else {
                chrono::Utc::now() - chrono::Duration::seconds(3600)
            },
        }
    }

    fn instant() -> Duration {
        Duration::ZERO
    }
    fn service(
        demo: bool,
        backend: Arc<FakeBackend>,
        store: FakeStore,
        min_interval: Duration,
    ) -> Service {
        Service::with_backends(demo, backend, Arc::new(store), min_interval)
    }

    #[tokio::test]
    async fn demo_read_quota_never_touches_backend_or_store() {
        let backend = Arc::new(FakeBackend::new(false, false));
        let svc = service(true, backend.clone(), FakeStore::new(None), instant());

        let snap = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(snap.source, "demo_synthetic");
        assert_eq!(snap.balance.as_deref(), Some("42.50"));
        assert_eq!(snap.currency.as_deref(), Some("USD"));
        assert_eq!(snap.mood, Mood::Happy);
        assert_eq!(backend.calls(), 0);

        let snap = svc
            .read_quota(Provider::OpenaiCodex, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(snap.source, "demo_synthetic");
        assert_eq!(snap.remaining_percent, Some(68.0));
        assert_eq!(snap.mood, Mood::Happy);
        assert_eq!(backend.calls(), 0);
    }

    #[tokio::test]
    async fn demo_snapshot_is_fresh_against_wall_clock() {
        let snap = demo_snapshot(Provider::OpenaiCodex);
        let fetched = chrono::DateTime::parse_from_rfc3339(&snap.fetched_at).unwrap();
        let age = (chrono::Utc::now() - fetched.with_timezone(&chrono::Utc)).num_seconds();
        assert!(
            (-60i64..=600).contains(&age),
            "demo fetched_at must be fresh, age={age}"
        );
        let resets =
            chrono::DateTime::parse_from_rfc3339(snap.resets_at.as_ref().unwrap()).unwrap();
        assert!(
            resets > chrono::Utc::now(),
            "demo resets_at must be in the future"
        );
    }

    #[tokio::test]
    async fn demo_save_and_delete_key_are_rejected() {
        let svc = service(
            true,
            Arc::new(FakeBackend::new(false, false)),
            FakeStore::new(None),
            instant(),
        );
        assert_eq!(
            svc.save_key("sk-valid000000".to_string())
                .await
                .unwrap_err()
                .code,
            AppError::DEMO_REJECTED
        );
        assert_eq!(
            svc.delete_key().await.unwrap_err().code,
            AppError::DEMO_REJECTED
        );
    }

    #[tokio::test]
    async fn demo_status_marks_unavailable_without_probing() {
        let svc = service(
            true,
            Arc::new(FakeBackend::new(false, false)),
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );
        let st = svc.status().await.unwrap();
        assert!(st.demo);
        assert!(!st.deepseek_configured);
        assert!(!st.codex_available);
        assert!(!st.hermes_available);
        assert_eq!(st.version, APP_VERSION);
    }

    #[tokio::test]
    async fn deepseek_requires_configured_key() {
        let svc = service(
            false,
            Arc::new(FakeBackend::new(false, false)),
            FakeStore::new(None),
            instant(),
        );
        assert_eq!(
            svc.read_quota(Provider::Deepseek, CodexSource::Auto)
                .await
                .unwrap_err()
                .code,
            AppError::NOT_CONFIGURED
        );
    }

    #[tokio::test]
    async fn deepseek_keyring_error_surfaces_not_silently_unconfigured() {
        let store = FakeStore::new(Some("sk-valid000000"));
        store.fail_get_with(AppError::new(AppError::KEYRING_UNAVAILABLE));
        let svc = service(
            false,
            Arc::new(FakeBackend::new(false, false)),
            store,
            instant(),
        );
        assert_eq!(
            svc.read_quota(Provider::Deepseek, CodexSource::Auto)
                .await
                .unwrap_err()
                .code,
            AppError::KEYRING_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn deepseek_builds_snapshot_with_net_change_and_baseline() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "42.50")])));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "40.00")])));
        let svc = service(
            false,
            backend,
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );

        let first = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(first.balance.as_deref(), Some("42.50"));
        assert_eq!(first.net_change, None);
        assert_eq!(first.mood, Mood::Happy);
        assert_eq!(first.is_available, Some(true));

        let second = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(second.balance.as_deref(), Some("40.00"));
        assert_eq!(second.net_change.as_deref(), Some("-2.50"));
    }

    #[tokio::test]
    async fn deepseek_usd_preferred_and_unavailable_is_asleep() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(
            false,
            vec![wallet("CNY", "110.00"), wallet("USD", "3.00")],
        )));
        let svc = service(
            false,
            backend,
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );
        let snap = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(snap.currency.as_deref(), Some("USD"));
        assert_eq!(snap.balance.as_deref(), Some("3.00"));
        assert_eq!(snap.mood, Mood::Asleep);
        assert_eq!(snap.is_available, Some(false));
    }

    #[tokio::test]
    async fn tiny_positive_balance_is_sleepy_not_rounded_away() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "0.001")])));
        let svc = service(
            false,
            backend,
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );
        let snap = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(snap.balance.as_deref(), Some("0.001"));
        assert_eq!(snap.mood, Mood::Sleepy);
    }

    #[tokio::test]
    async fn extreme_wallet_subtraction_returns_sanitized_error_not_panic() {
        let max = "79228162514264337593543950335";
        let min = "-79228162514264337593543950335";
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", max)])));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", min)])));
        let svc = service(
            false,
            backend,
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );
        let _first = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        let err = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap_err();
        assert_eq!(err.code, AppError::DEEPSEEK_BAD_RESPONSE);
    }

    #[tokio::test]
    async fn min_interval_serves_cache_without_new_request() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "42.50")])));
        let svc = service(
            false,
            backend.clone(),
            FakeStore::new(Some("sk-valid000000")),
            Duration::from_secs(15),
        );

        let first = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        let second = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(
            first.fetched_at, second.fetched_at,
            "second read must be cached"
        );
        assert_eq!(backend.calls(), 1, "no new network request within interval");
    }

    #[tokio::test]
    async fn failed_read_never_serves_stale_cache_within_interval() {
        // Zero interval: read ok (caches 42.50), read fails, then a fresh read
        // must re-attempt the backend (returning the default empty balance)
        // rather than serving the stale "42.50".
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "42.50")])));
        backend.push_deepseek(Err(AppError::new(AppError::DEEPSEEK_REQUEST_FAILED)));
        let svc = service(
            false,
            backend.clone(),
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );

        let ok = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(ok.balance.as_deref(), Some("42.50"));
        let err = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap_err();
        assert_eq!(err.code, AppError::DEEPSEEK_REQUEST_FAILED);
        let third = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_ne!(
            third.balance.as_deref(),
            Some("42.50"),
            "stale cache must be gone"
        );
        assert_eq!(backend.calls(), 3, "third read must re-attempt the backend");
    }

    #[tokio::test]
    async fn zero_interval_allows_refetch() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "42.50")])));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "39.00")])));
        let svc = service(
            false,
            backend,
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );
        let first = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        let second = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_ne!(first.balance, second.balance);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn in_flight_request_is_not_duplicated() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "42.50")])));
        let (tx, rx) = oneshot::channel();
        backend.set_block(rx);
        let svc = Arc::new(Service::with_backends(
            false,
            backend.clone(),
            Arc::new(FakeStore::new(Some("sk-valid000000"))),
            instant(),
        ));

        let a = {
            let svc = svc.clone();
            tokio::spawn(async move { svc.read_quota(Provider::Deepseek, CodexSource::Auto).await })
        };

        for _ in 0..200 {
            if backend.entered() > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            backend.entered() > 0,
            "first request must have entered the backend"
        );

        let b = svc.read_quota(Provider::Deepseek, CodexSource::Auto).await;
        assert_eq!(b.unwrap_err().code, AppError::BUSY);

        let _ = tx.send(());
        let a_result = a.await.unwrap().unwrap();
        assert_eq!(a_result.balance.as_deref(), Some("42.50"));
        assert_eq!(backend.calls(), 1, "only one network request");
    }

    #[tokio::test]
    async fn codex_auto_never_falls_back_after_error() {
        let backend = Arc::new(FakeBackend::new(true, true));
        backend.push_percent(Err(AppError::new(AppError::CODEX_REQUEST_FAILED)));
        let svc = service(false, backend, FakeStore::new(None), instant());
        let err = svc
            .read_quota(Provider::OpenaiCodex, CodexSource::Auto)
            .await
            .unwrap_err();
        assert_eq!(err.code, AppError::CODEX_REQUEST_FAILED);
    }

    #[tokio::test]
    async fn codex_auto_resolves_to_hermes_when_codex_absent() {
        let backend = Arc::new(FakeBackend::new(false, true));
        backend.push_percent(Ok(percent(Some(23.0), true, "usage_api")));
        let svc = service(false, backend, FakeStore::new(None), instant());
        let snap = svc
            .read_quota(Provider::OpenaiCodex, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(snap.source, "usage_api");
        assert_eq!(snap.remaining_percent, Some(77.0));
        assert_eq!(snap.mood, Mood::Happy);
    }

    #[tokio::test]
    async fn codex_unavailable_when_neither_backend_exists() {
        let svc = service(
            false,
            Arc::new(FakeBackend::new(false, false)),
            FakeStore::new(None),
            instant(),
        );
        assert_eq!(
            svc.read_quota(Provider::OpenaiCodex, CodexSource::Auto)
                .await
                .unwrap_err()
                .code,
            AppError::CODEX_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn percent_snapshot_stale_reset_passed_or_missing_is_unknown() {
        let snap = build_percent_snapshot(
            Provider::OpenaiCodex,
            percent(Some(23.0), false, "usage_api"),
        );
        assert_eq!(snap.mood, Mood::Unknown);
        assert_eq!(snap.remaining_percent, None);

        let quota = PercentQuota {
            source: "usage_api".to_string(),
            used_percent: Some(23.0),
            resets_at: chrono::DateTime::from_timestamp(1_000_000_000, 0),
            fetched_at: chrono::Utc::now(),
        };
        let snap = build_percent_snapshot(Provider::OpenaiCodex, quota);
        assert_eq!(snap.mood, Mood::Unknown);

        let snap = build_percent_snapshot(Provider::OpenaiCodex, percent(None, true, "usage_api"));
        assert_eq!(snap.mood, Mood::Unknown);
    }

    #[tokio::test]
    async fn credential_change_clears_baseline() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "42.50")])));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "40.00")])));
        let svc = service(
            false,
            backend.clone(),
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );

        let first = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(first.net_change, None);
        let second = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(second.net_change.as_deref(), Some("-2.50"));

        svc.save_key("sk-valid000001".to_string()).await.unwrap();

        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "40.00")])));
        let third = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(
            third.net_change, None,
            "baseline reset after credential change"
        );
    }

    #[tokio::test]
    async fn credential_change_resets_throttle_so_next_read_works() {
        let backend = Arc::new(FakeBackend::new(false, false));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "42.50")])));
        backend.push_deepseek(Ok(balance(true, vec![wallet("USD", "41.00")])));
        let svc = service(
            false,
            backend.clone(),
            FakeStore::new(Some("sk-valid000000")),
            Duration::from_secs(15),
        );

        let first = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(first.balance.as_deref(), Some("42.50"));

        // Change the key: this must clear the throttle so the next read for the
        // new account proceeds instead of being throttled/BUSY.
        svc.save_key("sk-valid000002".to_string()).await.unwrap();
        let second = svc
            .read_quota(Provider::Deepseek, CodexSource::Auto)
            .await
            .unwrap();
        assert_eq!(
            second.balance.as_deref(),
            Some("41.00"),
            "throttle must be reset on key change"
        );
        assert_eq!(backend.calls(), 2);
    }

    #[tokio::test]
    async fn set_demo_transitions_and_reports_status() {
        let backend = Arc::new(FakeBackend::new(true, true));
        let svc = service(
            true,
            backend,
            FakeStore::new(Some("sk-valid000000")),
            instant(),
        );
        assert!(svc.status().await.unwrap().demo);
        let st = svc.set_demo(false).await.unwrap();
        assert!(!st.demo);
        assert!(st.codex_available);
        assert!(st.hermes_available);
        assert!(st.deepseek_configured);
    }
}
