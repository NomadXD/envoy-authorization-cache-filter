use log::{debug, info, trace, warn};
use proxy_wasm::{
    traits::{Context, HttpContext, RootContext},
    types::{Action, ContextType, LogLevel},
};

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

const CACHE_KEY: &str = "cache";
const POWERED_BY: &str = "cache-filter";

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
struct FilterConfig {
    /// Envoy cluster name that provides ext_authz service. Should provide the cluster
    /// name of the ext_authz cluster in the envoy.yaml file.
    management_service_cluster: String,

    /// The path to call on the HTTP service for ext_authz
    ext_authz_service_path: String,

    /// External auth request authority header
    ext_authz_authority: String,
}

impl Default for FilterConfig {
    fn default() -> Self {
        FilterConfig {
            management_service_cluster: "management-service".to_owned(),
            ext_authz_service_path: "/auth".to_owned(),
            ext_authz_authority: "ext_authz".to_owned(),
        }
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|context_id| -> Box<dyn RootContext> {
        Box::new(CacheFilterRoot {
            context_id,
            config: FilterConfig::default(),
        })
    });
}

struct CacheFilterRoot {
    context_id: u32,
    config: FilterConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthRequest {
    token: String,
    path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CacheRecord {
    path: String,
    quota: i32,
    used: i32,
}

// Local cache representation
#[derive(Serialize, Deserialize, Debug)]
struct Cache {
    foo_path: String,
    foo_quota: i32,
    foo_used: i32,
    bar_path: String,
    bar_quota: i32,
    bar_used: i32,
}

impl RootContext for CacheFilterRoot {
    fn on_vm_start(&mut self, _vm_configuration_size: usize) -> bool {
        info!("VM started");
        true
    }

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
                return true;
            }
            Err(e) => {
                warn!("Failed to parse envoy.yaml configuration: {:?}", e);
                return false;
            }
        }
    }

    fn create_http_context(&self, _context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(CacheFilter {
            config: self.config.clone(),
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}

impl Context for CacheFilterRoot {}

struct CacheFilter {
    config: FilterConfig,
}

impl HttpContext for CacheFilter {
    fn on_http_request_headers(&mut self, _num_headers: usize) -> Action {
        let path = self.get_http_request_header(":path").unwrap();
        let token = self.get_http_request_header("Authorization");
        info!("request intercepted by the cache-filter with path {}", path);
        // Check for Authorization header
        match token {
            // If Authorization header available
            Some(token) => {
                // Check local cache for cacheable rules and decide request flow
                info!("Authorization header available | path {}", path);
                match self.get_cache_record(path.clone()) {
                    // If a record exist in local cache and quota remaining -> Action::Continue
                    Some(cache) => {
                        info!("Record exist in the local cache");
                        if cache.used < cache.quota {
                            self.update_cache_record(path.clone());
                            info!("Service quota not reached. Proxying the request upstream | path {}", path);
                            return Action::Continue;
                        } else {
                            // Local cache exist but quota reached
                            info!(
                                "Service quota reached. Sending local response | path {}",
                                path
                            );
                            self.send_http_response(
                                429,
                                vec![("x-powered-by", POWERED_BY)],
                                Some(b"Service quota reached.\n"),
                            );
                            return Action::Pause;
                        }
                    }
                    // No record found in the local cache. Send request to management-service for authorization
                    None => {
                        let auth_request: AuthRequest = AuthRequest {
                            token: token,
                            path: path,
                        };
                        info!("No record found in the local cache. Send request to management-service for authorization: {}", auth_request.path);
                        let request_string = serde_json::to_string(&auth_request).unwrap();
                        self.dispatch_http_call(
                            self.config.management_service_cluster.as_str(),
                            vec![
                                (":method", "POST"),
                                (":path", self.config.ext_authz_service_path.as_str()),
                                (":authority", self.config.ext_authz_authority.as_str()),
                                ("Content-Type", "application/json"),
                            ],
                            Some(request_string.as_bytes()),
                            vec![],
                            Duration::from_secs(5),
                        )
                        .unwrap();
                        Action::Pause
                    }
                }
            }
            // Authorization header not available. Send a local response with 403
            None => {
                self.send_http_response(
                    403,
                    vec![("x-powered-by", POWERED_BY)],
                    Some(b"Access forbidden.\n"),
                );
                Action::Pause
            }
        }
    }
}

impl Context for CacheFilter {
    fn on_http_call_response(
        &mut self,
        _token_id: u32,
        _num_headers: usize,
        body_size: usize,
        _num_trailers: usize,
    ) {
        if let Some(body) = self.get_http_call_response_body(0, body_size) {
            let data: Value = serde_json::from_slice(body.as_slice()).unwrap();
            info!(
                "Authorization response received from management service: {}",
                data
            );
            if data.get("status") != None {
                //info!("Error fetching token: {}, {}", data.get("error").unwrap(), data.get("error_description").unwrap());
                //return
                // if(data.get("status").unwrap() == 200){
                //     self.resume_http_request();
                // }else
                let status = data.get("status").unwrap().as_u64().unwrap();
                if status == 200 {
                    self.resume_http_request();
                } else if status == 401 {
                    self.send_http_response(
                        401,
                        vec![("x-powered-by", POWERED_BY)],
                        Some(b"Unauthorized\n"),
                    )
                } else if status == 429 {
                    self.send_http_response(
                        429,
                        vec![("x-powered-by", POWERED_BY)],
                        Some(b"Service Quota Reached\n"),
                    )
                } else {
                    self.send_http_response(
                        500,
                        vec![("x-powered-by", POWERED_BY)],
                        Some(b"Service Unavailable\n"),
                    )
                }
            } else {
                self.send_http_response(
                    500,
                    vec![("x-powered-by", POWERED_BY)],
                    Some(b"Service Unavailable\n"),
                )
            }

            // if data.get("id_token") != None {
            //     info!("id_token found. Setting cookie and redirecting...");
            //     self.send_http_response(
            //         302,
            //         vec![
            //             ("Set-Cookie", format!("oidcToken={};Max-Age={}", data.get("id_token").unwrap(), data.get("expires_in").unwrap()).as_str()),
            //             ("Location", format!("http://{}{}", host, path).as_str()),
            //         ],
            //         Some(b""),
            //     );
            //     return
            // }
        } else {
            self.send_http_response(
                500,
                vec![("x-powered-by", POWERED_BY)],
                Some(b"Service Unavailable\n"),
            )
        }

        //self.resume_http_request()
    }
}

impl CacheFilter {
    // fn parse_headers(&self, headers: Vec<(String, String)>) -> String {
    //     for
    // }

    // fn print_cache_key(&self, key: &[u8]) {
    //     info!("Cache key: {}", String::from_utf8(key.clone()).unwrap())
    // }

    fn get_cache_record(&self, path: String) -> Option<CacheRecord> {
        if path == "/foo" || path == "/bar" {
            match self.get_shared_data(CACHE_KEY) {
                (None, _) => return None,
                (Some(data), _) => {
                    let cache = String::from_utf8(data.clone()).unwrap();
                    info!("Getting local cache: {}", cache);
                    let cache_json: Cache = serde_json::from_str(&cache).unwrap();
                    if path == "/foo" {
                        return Some(CacheRecord {
                            path: cache_json.foo_path,
                            quota: cache_json.foo_quota,
                            used: cache_json.foo_used,
                        });
                    } else {
                        return Some(CacheRecord {
                            path: cache_json.bar_path,
                            quota: cache_json.bar_quota,
                            used: cache_json.bar_used,
                        });
                    }
                }
            }
        } else {
            return None;
        }
    }

    fn update_cache_record(&self, path: String) -> bool {
        match self.get_shared_data(CACHE_KEY) {
            (None, _) => return false,
            (Some(data), _) => {
                let cache = String::from_utf8(data.clone()).unwrap();
                let mut cache_json: Cache = serde_json::from_str(&cache).unwrap();
                if path == "/foo" {
                    cache_json.foo_used += 1
                } else {
                    cache_json.bar_used += 1
                }
                let updated_cache = serde_json::to_vec(&cache_json).unwrap();
                match self.set_shared_data(CACHE_KEY, Some(&updated_cache), None) {
                    Ok(()) => return true,

                    Err(e) => {
                        warn!("cache update failed: {:?}", e);
                        return false;
                    }
                }
            }
        }
    }
}
