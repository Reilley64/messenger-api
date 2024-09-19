FROM public.ecr.aws/docker/library/rust:bookworm AS build

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && mkdir migrations && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN cargo build --release
RUN rm src/main.rs

COPY migrations/ migrations/
COPY src/ src/
RUN cargo build --release

FROM public.ecr.aws/docker/library/debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libpq-dev libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/messenger-api messenger-api

CMD ["./messenger-api"]
