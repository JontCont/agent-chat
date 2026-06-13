# Build stage
FROM rust:1.86-slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/rust-axum-agent-bridge .
COPY --from=builder /app/src/frontend ./src/frontend
COPY --from=builder /app/template ./template
EXPOSE 8080
ENV DATABASE_URL="sqlite:///data/sqlite/agent.db"
ENV PORT=8080
CMD ["./rust-axum-agent-bridge"]
