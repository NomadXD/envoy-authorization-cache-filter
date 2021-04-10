use log::{debug, info, trace, warn};
use proxy_wasm::{traits::{Context, HttpContext, RootContext}, types::{Action, ContextType, LogLevel}};

use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::{time::Duration};

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
        Box::new(CacheFilterRoot { context_id , config: FilterConfig::default()})
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
                //CONFIGS.with(|configs| configs.borrow_mut().insert(self.context_id, config));
                self.config = config;
            }
            Err(e) => {
                warn!("Failed to parse envoy.yaml configuration: {:?}", e);

                return false;
            }
        }
        return true;
    }

    fn create_http_context(&self, _context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(CacheFilter{
            config: self.config.clone()
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}

impl Context for CacheFilterRoot {}

struct CacheFilter {
    config : FilterConfig,
}

impl HttpContext for CacheFilter {
    fn on_http_request_headers(&mut self, _num_headers: usize) -> Action {
        // let path = self.get_http_request_header(":path").unwrap();
        // info!("path XXXXX: {}", path);
        // match self.get_shared_data(&path) {
        //     (Some(cache), _) => {
        //         debug!(
        //             "using existing path cache: {}",
        //             String::from_utf8(cache.clone()).unwrap()
        //         );

        //         // match self.parse_headers(&cache) {
        //         //     Ok(headers) => {
        //         //         for (name, value) in headers {
        //         //             self.set_http_request_header(&name, value.as_str())
        //         //         }
        //         //     }
        //         //     Err(e) => warn!("no usable headers cached: {:?}", e),
        //         // }

        //         Action::Continue
        //     }
        //     (None, _) => {
        //         warn!("filter not initialised");

        //         self.dispatch_http_call(
        //             self.config.auth_cluster.as_str(), vec![
        //                 (":method", "POST"),
        //                 (":path", self.config.token_uri.as_str()),
        //                 (":authority", self.config.auth_host.as_str()),
        //                 ("Content-Type", "application/x-www-form-urlencoded"),
        //             ],
        //             Some(data.as_bytes()),
        //             vec![],
        //             Duration::from_secs(5)
        //         ).unwrap();

        //         // self.send_http_response(
        //         //     500,
        //         //     vec![("Powered-By", POWERED_BY)],
        //         //     Some(b"Filter not initialised"),
        //         // );

        //         Action::Pause
        //     }
        // }
        let path = self.get_http_request_header(":path").unwrap();
        let token = self.get_http_request_header("Authorization");
        match token {
            Some(token) => {
                let sampleBody: AuthRequest = AuthRequest {
                    token: token,
                    path: path,
                };
                info!("path XXXXX: {}", sampleBody.path);
                info!("auth XXXXX: {}", sampleBody.token);
                let serlizedString = serde_json::to_string(&sampleBody).unwrap();
                self.dispatch_http_call(
                self.config.management_service_cluster.as_str(), 
                vec![
                    (":method", "POST"),
                    (":path", self.config.ext_authz_service_path.as_str()),
                    (":authority", self.config.ext_authz_authority.as_str()),
                    ("Content-Type", "application/json"),
                ],
                Some(serlizedString.as_bytes()),
                vec![],
                Duration::from_secs(5)
                ).unwrap();
                Action::Pause

            }
            None => {
                self.send_http_response(403, vec![("Powered-By", "proxy-wasm")], Some(b"Access forbidden.\n"));
                Action::Pause
            }
        }
        

    }
}

impl Context for CacheFilter {
    fn on_http_call_response(&mut self, _token_id: u32, _num_headers: usize, body_size: usize, _num_trailers: usize) {
        if let Some(body) = self.get_http_call_response_body(0, body_size) {
            let data: Value = serde_json::from_slice(body.as_slice()).unwrap();
            debug!("Received json blob: {}", data);
            if data.get("status") != None {
                //info!("Error fetching token: {}, {}", data.get("error").unwrap(), data.get("error_description").unwrap());
                //return
                // if(data.get("status").unwrap() == 200){
                //     self.resume_http_request();
                // }else
                let status = data.get("status").unwrap().as_u64();
                if status.unwrap() == 200{
                    self.resume_http_request();
                }else if status.unwrap() == 401 {
                    self.send_http_response(401, vec![("Powered-By", "WASM")],
                    Some(b"Unauthorized"))
                }else{
                    self.send_http_response(500, vec![("Powered-By", "WASM")],
                    Some(b"Unauthorized"))
                }
            }else{
                self.send_http_response(500, vec![("Powered-By", "WASM")],
                    Some(b"Unauthorized"))
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
        }
        
        self.resume_http_request()
    }
}

impl CacheFilter {
    // fn parse_headers(&self, headers: Vec<(String, String)>) -> String {
    //     for 
    // }

    // fn print_cache_key(&self, key: &[u8]) {
    //     info!("Cache key: {}", String::from_utf8(key.clone()).unwrap())
    // }
}
