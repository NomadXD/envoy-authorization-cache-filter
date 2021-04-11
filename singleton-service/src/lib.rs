use log::{debug, info, trace, warn};
use proxy_wasm::{
    traits::{Context, RootContext},
    types::LogLevel,
};

use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;

const CACHE_KEY: &str = "cache";
const INITIALISATION_TICK: Duration = Duration::from_secs(10);
const POWERED_BY: &str = "cache-filter";

#[derive(Deserialize, Debug)]
#[serde(default)]
struct FilterConfig {
    /// Envoy cluster name that provides ext_authz service. Should provide the cluster
    /// name of the ext_authz cluster in the envoy.yaml file.
    management_service_cluster: String,

    /// The path to call on the HTTP service for cache
    cache_service_path: String,

    /// Time duration for the cache update
    #[serde(with = "serde_humanize_rs")]
    cache_update_duration: Duration,

    /// Cache service request authority header
    cache_service_authority: String,
}

impl Default for FilterConfig {
    fn default() -> Self {
        FilterConfig {
            management_service_cluster: "management-service".to_owned(),
            cache_service_path: "/cache".to_owned(),
            cache_update_duration: Duration::from_secs(360),
            cache_service_authority: "cache_service".to_owned(),
        }
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|context_id| -> Box<dyn RootContext> {
        Box::new(SingletonService {
            context_id,
            config: FilterConfig::default(),
        })
    });
}

struct SingletonService {
    context_id: u32,
    config: FilterConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct Token {
    token: String,
    path: String,
}

impl RootContext for SingletonService {
    fn on_configure(&mut self, _config_size: usize) -> bool {
        //Check for the configuration passed by envoy.yaml
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
                self.config = config;
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
        info!("onTick triggered");
        match self.get_shared_data(CACHE_KEY) {
            (None, _) => debug!("initialising local cache"),
            (Some(_), _) => debug!("updating local cache"),
        }

        // Update the tick to the cache update duration. This can be one of follows.
        // initial_tick_duration -> cache_update_duration
        // cache_update_duration -> cache_update_duration
        // Also when the cache_update request fails, set_tick_period to initial_tick_duration
        self.set_tick_period(Duration::from_secs(20));

        match self.get_shared_data(CACHE_KEY) {
            // No local cache. Send a GET to pull cacheable rules
            (None, _) => {
                info!("initialising local cache");
                self.dispatch_http_call(
                    "management-service",
                    vec![
                        (":method", "GET"),
                        (":path", "/cache"),
                        (":authority", "management-service"),
                        ("Content-Type", "application/json"),
                        ("x-powered-by", POWERED_BY),
                    ],
                    None,
                    vec![],
                    Duration::from_secs(5),
                )
                .map_err(|e| {
                    // HTTP call failed. Reset to an
                    // initialisation tick for a quick retry.
                    self.set_tick_period(INITIALISATION_TICK);

                    warn!("Failed calling management service: {:?}", e)
                })
                .unwrap();
            }
            // Local cache available. Send local cache to management service for synchronization
            // and update the local cache with a new version
            (Some(data), _) => {
                let local_cache = String::from_utf8(data.clone()).unwrap();
                info!("sending local cache to management service {}", local_cache);
                self.dispatch_http_call(
                    "management-service",
                    vec![
                        (":method", "POST"),
                        (":path", "/cache"),
                        (":authority", "management-service"),
                        ("Content-Type", "application/json"),
                        ("x-powered-by", POWERED_BY),
                    ],
                    Some(local_cache.as_bytes()),
                    vec![],
                    Duration::from_secs(5),
                )
                .map_err(|e| {
                    // HTTP call failed. Reset to an
                    // initialisation tick for a quick retry.
                    self.set_tick_period(INITIALISATION_TICK);

                    warn!("Failed calling management service: {:?}", e)
                })
                .unwrap();
            }
        }
    }
}

impl Context for SingletonService {
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
                warn!("management service returned empty body");
                return;
            }
        };

        // Update the local cache with the new version received
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
