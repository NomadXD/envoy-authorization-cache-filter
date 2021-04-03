FROM rustlang/rust:nightly

WORKDIR /usr/src/backend-service

COPY . .

RUN cargo build --release

RUN cargo install --path .

CMD ["/usr/local/cargo/bin/backend-service"]