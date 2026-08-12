# syntax=docker/dockerfile:1.7

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
	ca-certificates \
	pkg-config \
	libxml2-dev \
	libclang-dev \
	clang \
	&& rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . ./
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (caching Docker layer)
RUN --mount=type=secret,id=local_ca,required=false \
	if [ -f /run/secrets/local_ca ]; then \
		cat /etc/ssl/certs/ca-certificates.crt /run/secrets/local_ca > /tmp/local-ca-bundle.pem \
		&& CARGO_HTTP_CAINFO=/tmp/local-ca-bundle.pem SSL_CERT_FILE=/tmp/local-ca-bundle.pem \
			cargo chef cook --release --recipe-path recipe.json \
		&& rm -f /tmp/local-ca-bundle.pem; \
	else \
		cargo chef cook --release --recipe-path recipe.json; \
	fi

# Build application
COPY . ./
RUN cargo build --release
RUN --mount=type=secret,id=local_ca,required=false \
	if [ -f /run/secrets/local_ca ]; then \
		cat /etc/ssl/certs/ca-certificates.crt /run/secrets/local_ca > /tmp/local-ca-bundle.pem \
		&& cd migration \
		&& CARGO_HTTP_CAINFO=/tmp/local-ca-bundle.pem SSL_CERT_FILE=/tmp/local-ca-bundle.pem \
			cargo build --release \
		&& rm -f /tmp/local-ca-bundle.pem; \
	else \
		cd migration && cargo build --release; \
	fi

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
	libxml2 \
	ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/backend /usr/local/bin/app
COPY --from=builder /app/target/release/seed /usr/local/bin/seed
COPY --from=builder /app/migration/target/release/migration /usr/local/bin/migration

EXPOSE 3000
CMD ["app"]
