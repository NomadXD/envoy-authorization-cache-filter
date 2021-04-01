use log::{debug, info, trace, warn};
use proxy_wasm::{
    traits::{Context, HttpContext, RootContext},
    types::{Action, LogLevel},
};

use serde::Deserialize;
use serde_json::{Map, Value};
use std::{cell::RefCell, collections::HashMap, error::Error, time::Duration};

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
}

impl Default for FilterConfig {
    fn default() -> Self {
        FilterConfig {
            ext_authz_cluster: "ext_authz".to_owned(),
            auth_service_path: "/auth".to_owned(),
            cache_service_cluster: "cache_service".to_owned(),
            cache_service_path: "/cache".to_owned(),
            cache_update_duration: Duration::from_secs(360),
        }
    }
}
