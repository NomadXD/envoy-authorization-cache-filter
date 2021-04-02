use log::{debug, info, trace, warn};
use proxy_wasm::{
    traits::{Context, HttpContext, RootContext},
    types::{Action, LogLevel},
};

use serde::Deserialize;
use serde_json::{Map, Value};
use std::{cell::RefCell, collections::HashMap, error::Error, time::Duration};

const CACHE_KEY: &str = "cache";
const INITIALISATION_TICK: Duration = Duration::from_secs(3);

#[derive(Deserialize, Debug)]
#[serde(default)]
struct FilterConfig {
    /// Envoy cluster name that provides ext_authz service. Should provide the cluster
    /// name of the ext_authz cluster in the envoy.yaml file.
    ext_authz_cluster: String,

    /// The path to call on the HTTP service for ext_authz
    auth_service_path: String,

    /// Envoy cluster name that provides cache service. Should provide the cluster
    /// name of the cache cluster in the envoy.yaml file.
    cache_service_cluster: String,

    /// The path to call on the HTTP service for cache
    cache_service_path: String,

    /// Time duration for the cache update
    #[serde(with = "serde_humanize_rs")]
    cache_update_duration: Duration,

    /// External auth request authority header
    ext_authz_authority: String,

    /// Cache service request authority header
    cache_service_authority: String,
}

impl Default for FilterConfig {
    fn default() -> Self {
        FilterConfig {
            ext_authz_cluster: "ext_authz".to_owned(),
            auth_service_path: "/auth".to_owned(),
            cache_service_cluster: "cache_service".to_owned(),
            cache_service_path: "/cache".to_owned(),
            cache_update_duration: Duration::from_secs(360),
            ext_authz_authority: "ext_authz".to_owned(),
            cache_service_authority: "cache_service".to_owned(),
        }
    }
}

thread_local! {
    static CONFIGS: RefCell<HashMap<u32, FilterConfig>> = RefCell::new(HashMap::new())
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|context_id| -> Box<dyn RootContext> {
        CONFIGS.with(|configs| {
            configs
                .borrow_mut()
                .insert(context_id, FilterConfig::default());
        });

        Box::new(CacheFilterRoot { context_id })
    });
    proxy_wasm::set_http_context(|_context_id, _root_context_id| -> Box<dyn HttpContext> {
        Box::new(CacheFilter {})
    })
}

struct CacheFilterRoot {
    context_id: u32,
}

impl RootContext for CacheFilterRoot {
    fn on_configure(&mut self, _config_size: usize) -> bool {
        // Check for the configuration passed by envoy.yaml
        let configuration: Vec<u8> = match self.get_configuration() {
            Some(c) => c,
            None => {
                warn!("Configuration missing. Please check the envoy.yaml file for filter configuration");

                return false;
            }
        };

        // Parse and store the configuration passed by envoy.yaml
        match serde_json::from_slice::<FilterConfig>(configuration.as_ref()) {
            Ok(config) => {
                debug!("configuring {}: {:?}", self.context_id, config);
                CONFIGS.with(|configs| configs.borrow_mut().insert(self.context_id, config));
            }
            Err(e) => {
                warn!("Failed to parse envoy.yaml configuration: {:?}", e);

                return false;
            }
        }

        // Configure an initialisation tick and the cache.
        self.set_tick_period(INITIALISATION_TICK);
        self.set_shared_data(CACHE_KEY, None, None).is_ok()
    }

    fn on_tick(&mut self) {
        // Log the action that is about to be taken. It could be one of initialization, cache update or a retry
        match self.get_shared_data(CACHE_KEY) {
            (None, _) => debug!("initialising cache map"),
            (Some(_), _) => debug!("updating cache map"),
        }

        CONFIGS.with(|configs| {
            configs.borrow().get(&self.context_id).map(|config| {
                // Update the tick to the cache update duration. This can be one of follows.
                // initial_tick_duration -> cache_update_duration
                // cache_update_duration -> cache_update_duration
                // Also when the cache_update request fails, set_tick_period to initial_tick_duration
                self.set_tick_period(config.cache_update_duration);

                // Dispatch an async HTTP call to the configured cluster.
                self.dispatch_http_call(
                    &config.cache_service_cluster,
                    vec![
                        (":method", "GET"),
                        (":path", &config.cache_service_path),
                        (":authority", &config.cache_service_authority),
                    ],
                    None,
                    vec![],
                    Duration::from_secs(5),
                )
                .map_err(|e| {
                    // HTTP call failed. Reset to an
                    // initialisation tick for a quick retry.
                    self.set_tick_period(INITIALISATION_TICK);

                    warn!("Failed calling cache service: {:?}", e)
                })
            })
        });
    }
}

impl Context for CacheFilterRoot {
    // Callbacks for cache update request
    fn on_http_call_response(
        &mut self,
        _token_id: u32,
        _num_headers: usize,
        body_size: usize,
        _num_trailers: usize,
    ) {
        // Read the body of the HTTP response from the cache service
        let body = match self.get_http_call_response_body(0, body_size) {
            Some(body) => body,
            None => {
                warn!("cache service returned empty body");

                return;
            }
        };

        // Store the body in the shared cache.
        match self.set_shared_data(CACHE_KEY, Some(&body), None) {
            Ok(()) => debug!(
                "cache update successful: {}",
                String::from_utf8(body.clone()).unwrap()
            ),

            Err(e) => {
                warn!("cache update failed: {:?}", e);

                // Reset to an initialisation tick for a quick retry.
                self.set_tick_period(INITIALISATION_TICK)
            }
        }
    }
}

struct CacheFilter {}
