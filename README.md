<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/github_username/repo">
    <img src="img/cover.jpg" alt="Logo" width="800" height="250">
  </a>

  <h3 align="center">Envoy proxy authorization cache</h3>

  <p align="center">
   🛰 A POC to demonstrate possibility of implementing a local cache🛰️
    <br />
    <a href="#"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="#">View Demo</a>
    ·
    <a href="#">Report Bug</a>
    ·
    <a href="#">Request Feature</a>
  </p>
</p>



<!-- TABLE OF CONTENTS -->
## Table of Contents

* [About the Project](#about-the-project)
* [Getting Started](#getting-started)
  * [Prerequisites](#prerequisites)
  * [Installation](#installation)
* [Basic overview](#Basic-overview)
* [Example demonstrations](#Example-demonstrations)
* [License](#license)


<!-- ABOUT THE PROJECT -->
## About The Project
**Simple POC for envoy local cache using a WASM HTTP filter and a singleton service that synchronize local cache with a global level management-service**
[![Product Name Screen Shot][product-screenshot]](https://example.com)

<!-- GETTING STARTED -->
## Getting Started

To get a local copy up and running follow these simple steps.

### Prerequisites

* docker
* docker-compose

####  Build Prerequisites for filters and singleton service
* Rust
* Cargo
* Make

### Installation
 
1. Clone the repo 
```sh
git clone https://github.com/NomadXD/envoy-authorization-cache-filter.git
cd envoy-authorization-cache-filter
```
2. Build the project with docker-compose
```sh
docker-compose build
```
3. Start the services with docker-compose
```sh
docker-compose up
```

### Building WASM modules for cache filter and singleton service.

1. Go to the respective directory (singleton-service or cache-filter)

2. Execute `make build` from the root of that directory. See build prerequisites above to see whether everything is installed.

3. The WASM modules will be there in the envoy folder after build process is successfully completed.

## Basic overview

This simple POC comprises of 3 main components.
* 2 instances of envoy proxy as edge proxies with a custom HTTP filter and a singleton service built using WASM Rust SDK
* Backend service with 3 endpoints named `/foo`, `/bar` and `/baz`. (`/foo` and `/bar` are cacheable resources while `/baz` is not cacheable)
* Management service that holds the global rules for authorization.

The main intention of the POC is to demonstrate the capability of using a WASM HTTP filter and a singleton service to implement a local authorization cache in envoy that is periodically synced with a global management service. The following factors are considered when implementing the POC.

* A global level storage to store cache inside envoy where the cache is accessible from all the worker threads. Envoy has this support with their shared data feature. 
* A singleton service that periodically sends the local cache to a management service and then updates the local cache based on the response from the management service. Envoy WASM has a singleton service that can be used to execute processes outside the request life cycle. 
* A HTTP filter that is capabale of intercepting requests and performing authorization based on a local cache. If not found in local cache sends an HTTP call to the management service. HTTP filter should be able to update the cache when requests pass through the filter. Also the HTTP filter should be able to add response headers like rate limit headers. 

#### All these features are currently supported with envoy. But sending a HTTP request from a singleton service is currently broken for all the release versions of envoy. But the issue is fixed in the main and in the next release of envoy (v1.18), this will be fixed. So for this POC, we are using the `envoyproxy/envoy-dev:latest` image.

#### Also note that this POC is just to ensure that envoy related components (HTTP filter, shared data and singleton service) that are required for the project are working as expected and just to provide a very high level idea about how things work at envoy level.  

## Configurations

### Cache filter configuration

Following code block represents the cache filter configuration. We can pass `management_service_cluster`, `ext_authz_service_path` and `ext_authz_authority` from envoy.yaml. The config provided by the envoy.yaml will override the default hardcoded config values via the `on_configure()` method of the RootContext. 

```yaml

- name: envoy.filters.http.wasm
  typed_config:
  "@type": type.googleapis.com/udpa.type.v1.TypedStruct
  type_url: type.googleapis.com/envoy.extensions.filters.http.wasm.v3.Wasm
    value:
      config:
        name: "cache_filter"
        root_id: "cache_filter"
        configuration: 
          "@type": type.googleapis.com/google.protobuf.StringValue
          value: |
            {
                "management_service_cluster": "management-service",
                "ext_authz_service_path": "/auth",
                "ext_authz_authority": "ext_authz"
            }
        vm_config:
          runtime: "envoy.wasm.runtime.v8"
          vm_id: "my_vm_id"
          code:
            local:
              filename: "/usr/local/bin/cache_filter.wasm"
          configuration: {}
          allow_precompiled: true

```

### Singleton service configuration

Following code block represents the singleton service configuration. Like in the previous case, we can configure the values in the configuration section from envoy.yaml and the default hard coded values will get override by the provided values. 

```yaml
- name: envoy.bootstrap.wasm
  typed_config:
    '@type': type.googleapis.com/envoy.extensions.wasm.v3.WasmService
    singleton: true
    config:
      name: "singleton_service"
      root_id: "singleton_service"
      configuration: 
        "@type": type.googleapis.com/google.protobuf.StringValue
        value: |
          {
            "management_service_cluster": "management-service",
            "cache_service_path": "/cache",
            "cache_update_duration": "20s",
            "cache_service_authority": "cache-filter"
          }
      vm_config:
        runtime: "envoy.wasm.runtime.v8"
        vm_id: "my_vm_id"
        code:
          local:
            filename: "/usr/local/bin/singleton_service.wasm"
        configuration: {}
        allow_precompiled: true

```


<!-- ROADMAP -->
## Example demonstrations

For the demonstrations below, sample endpoints from the backend service will be used. `/foo` and `/bar` are cacheable resources and `/baz` is a non cacheable resource. For cacheable resources authorization will be done in the cache filter if a cache record exist and if not will be sent to management service for authorization. For non cacheable resources, authorization will be always performed by the management service. 

This sections demonstrates few examples of the features that are implemented in the POC.


### Periodical cache update

1. Start the service using docker-compose

    `docker-compose up`

2. Get a JWT token using the token endpoint

    `curl -X GET localhost:9098/token`

    Note the token endpoint is accessed directly from the service without proxying through envoy.

3. Send 2 subsequent requests to `/foo` endpoint and see the logs for cache update process. By default cache update duration is `20s` and if backend connection or anything is wrong it will do a retry every `10s`. Cache update duration is configurable using the envoy.yaml but retry time is hardcoded for now.

    `curl -X GET localhost:9095/foo -H "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.YXV0aC10b2tlbg.9SMiDKOqXy9R28XBelHlMAAO7K1SRXBwD9s3TpKdO0Q"`

    See the logs after sending the request. In the first cache update happens after the 2 requests, only 1 of the local cache's will contain `foo_used: 2`. This means only the proxy that handled those 2 requests know about the requests. After receiving a cache update, the management service will update the global state and sends back the updated snapshot back to proxies. **If the cache update request of the proxy that did not intercept the 2 requests take place first, then it will not know about the 2 requests beacause the new cache update will not contain those information. But in the next cache update both the local caches, will contain `foo_used: 2` as the global state contain those 2 requests from the cache update of the other proxy that intercepted the requests. So we cannot say a certain cache update will contain all the global state cache information at a particular time. But here since only 2 proxies are there, it is guranteed that after 2 cache updates the information will be there. For the simplicity the whole cache snapshot is sent here as the cache update. But in a more production level scenario we should calculate deltas and send only the changes.**  



### Exceeding the global limit

One issue of having a local cache is the accuracy of the authorization process but it comes with the benefit of less latencies. All the resources have a quota of 10 requests. So let's try to exploit the local cache and send more than 10 requests.

1. Start the service using docker-compose

    `docker-compose up`

2. Get a JWT token using the token endpoint

    `curl -X GET localhost:9098/token`

    Note the token endpoint is accessed directly from the service without proxying through envoy.

3. Send requests continously to one proxy untill you get a `429 Service quota exceeded` message and instantly send continous requests to the other edge proxy (this has to happen before the cache update. Default duration is 20s , so start the test right after a cache update.). You will be able to get successfull responses untill the next cache update or until the local limit runs out.   

### Request flow of non-cacheable resources

1. Start the service using docker-compose

    `docker-compose up`

2. Get a JWT token using the token endpoint

    `curl -X GET localhost:9098/token`

    Note the token endpoint is accessed directly from the service without proxying through envoy.

3. Send the request. 

    `curl -X GET localhost:9095/baz -H "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.YXV0aC10b2tlbg.9SMiDKOqXy9R28XBelHlMAAO7K1SRXBwD9s3TpKdO0Q" -v`

    The request will be intercepted by the cache filter and it will search the local cache using the path header of the request. Since local cache conatin only `/foo` and `/bar`, the cache filter will make a external HTTP call to the management-service and will hold the request untill the response from the management service. If the response is 200, it will pass the request to the next filter (in this case envoy router filter). If the response is 401 or 429, it will send a local reply with 401 or 429.

### Initial Cache update

The initial cache update will occur after 10s from starting the service. Before that initial cache update all the resources will be authroized using the management service by doing a HTTP call. To demonstrate that start the service and immediately send a request to `/foo` or `/bar`



<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE` for more information.










[product-screenshot]: img/overview.png