FROM envoyproxy/envoy:v1.17.1

COPY ./envoy.yaml /etc/envoy/envoy.yaml
COPY ./cache_filter.wasm /usr/local/bin/cache_filter.wasm
RUN chmod go+r /etc/envoy/envoy.yaml /usr/local/bin/cache_filter.wasm
CMD /usr/local/bin/envoy -c /etc/envoy/envoy.yaml --component-log-level wasm:trace