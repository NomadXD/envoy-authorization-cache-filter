FROM rustlang/rust:nightly

WORKDIR /usr/src/management-service

COPY . .

RUN cargo build --release

RUN cargo install --path .

CMD ["/usr/local/cargo/bin/management-service"]