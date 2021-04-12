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
* [Contact](#contact)
* [Acknowledgements](#acknowledgements)



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

## Basic overview

This simple POC comprises of 3 main components.
* 2 instances of envoy proxy as edge proxies with a custom HTTP filter and a singleton service built using WASM Rust SDK
* Backend service with 3 endpoints named `/foo`, `/bar` and `/baz`. (`/foo` and `/bar` are cacheable resources while `/baz` is not cacheable)
* Management service that holds the global rules for authorization.

The main intention of the POC is to demonstrate the capability of using a WASM HTTP filter and a singleton service to implement a local authorization cache in envoy that is periodically synced with a global management service. The following factors are considered when implementing the POC.

* A global level storage to store cache inside envoy where the cache is accessible from all the worker threads. Envoy has this support with their shared data feature. 
* A singleton service that periodically sends the local cache to a management service and then updates the local cache based on the response from the management service. Envoy supports singleton as a boostrap extension.
* A HTTP filter that is capabale of intercepting requests and performing authorization based on a local cache. If not found in local cache sends an HTTP call to the management service. HTTP filter should be able to update the cache when requests pass through the filter. Also the HTTP filter should be able to add response headers like rate limit headers. 

#### All these features are currently supported with envoy. But sending a HTTP request from a singleton service is currently broken for all the release versions of envoy. But the issue is fixed in the main and in the next release of envoy (1.18), this will be fixed. So for this POC, we are using the `envoyproxy/envoy-dev:latest` image.




<!-- ROADMAP -->
## Example demonstrations

This sections demonstrates few examples of the features that are implemented in the POC. 

### Periodical cache update

1. Start the service using docker-compose

    `docker-compose up`

2. Get a JWT token using the token endpoint

    `curl -X GET localhost:9098/token`

    Note the token endpoint is accessed directly from the service without proxying through envoy.

3. Send 2 subsequent requests to `/foo` endpoint and see the logs for cache update process. By default cache update duration is `20s` and if backend connection or anything is wrong it will do a retry every `10s`. Cache update duration is configurable using the envoy.yaml but retry time is hardcoded for now.

    `curl -X GET localhost:9095/foo -H "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.YXV0aC10b2tlbg.9SMiDKOqXy9R28XBelHlMAAO7K1SRXBwD9s3TpKdO0Q"`

    See the logs after sending the request. In the first cache update happens after the request only 1 of the local cache's will contain `foo_used: 2`, but in the next cache update both the local caches, will contain `foo_used: 2` as the local caches are resolved in the management service and envoy is updated with the newest version of the cache using the response of the cache update request. 

### Exceeding the global limit

One issue of having a local cache is the accuracy of the authorization process but it comes with the benefit of less latencies. All the resources have a quota of 10 requests. So let's try to exploit the local cache and send more than 10 requests.

1. Start the service using docker-compose

    `docker-compose up`

2. Get a JWT token using the token endpoint

    `curl -X GET localhost:9098/token`

    Note the token endpoint is accessed directly from the service without proxying through envoy.

3. Send requests continously to one proxy untill you get a `429 Service quota exceeded` message and instantly send continous requests to the other edge proxy (this has to happen before the cache update. Default duration is 20s , so start the test right after a cache update.). You will be able to get successfull responses untill the next cache update or until the local limit runs out.   





<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE` for more information.










[product-screenshot]: img/overview.png