FROM rustlang/rust:nightly

WORKDIR /usr/src/backend-service

COPY . .

RUN cargo build --release

RUN cargo install --path .

EXPOSE 8000

CMD ["/usr/local/cargo/bin/backend-service"]
