FROM public.ecr.aws/docker/library/rust:bookworm AS build

WORKDIR /app
COPY . .
RUN cargo build --release --bin messenger-api

FROM public.ecr.aws/docker/library/debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libpq-dev libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/messenger-api /usr/local/bin

ENTRYPOINT ["/usr/local/bin/messenger-api"]
