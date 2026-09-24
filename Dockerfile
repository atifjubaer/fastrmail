# ==============================================================================
# Stage 1: Build Svelte 5 Frontends (Webmail & Admin)
# ==============================================================================
FROM node:22-alpine AS frontend-builder
WORKDIR /build

# Build webmail frontend
COPY web/webmail/package*.json ./web/webmail/
RUN cd web/webmail && npm ci
COPY web/webmail/ ./web/webmail/
RUN cd web/webmail && npm run build

# Build admin dashboard frontend
COPY web/admin/package*.json ./web/admin/
RUN cd web/admin && npm ci
COPY web/admin/ ./web/admin/
RUN cd web/admin && npm run build

# ==============================================================================
# Stage 2: Build Rust Backend
# ==============================================================================
FROM rust:alpine AS backend-builder

# Install required build toolchain including cmake, perl, clang, and linux-headers for Tantivy & Tokio sys-crates
RUN apk add --no-cache musl-dev sqlite-dev openssl-dev build-base pkgconfig cmake perl clang lld linux-headers

# Musl default stack is 80KB; enlarge stack to 16MB to prevent rustc stack overflow on Alpine
ENV RUST_MIN_STACK=16777216

WORKDIR /app

# Copy workspace manifest and lockfile
COPY Cargo.toml Cargo.lock* ./

# Copy full crate workspace
COPY crates ./crates

# Copy built frontend assets so Axum can serve them
COPY --from=frontend-builder /build/web/webmail/dist ./web/webmail/dist
COPY --from=frontend-builder /build/web/admin/dist ./web/admin/dist

# Build the release binary
RUN cargo build --release -p fastrmail-binary

# ==============================================================================
# Stage 3: Minimal Production Runtime
# ==============================================================================
FROM alpine:latest AS runtime
RUN apk add --no-cache ca-certificates tzdata sqlite-libs libgcc libstdc++ wget

WORKDIR /app

# Copy binary from builder
COPY --from=backend-builder /app/target/release/fastrmail /usr/local/bin/fastrmail

# Copy frontend distribution folders for static serving
COPY --from=frontend-builder /build/web/webmail/dist /app/web/webmail/dist
COPY --from=frontend-builder /build/web/admin/dist /app/web/admin/dist

# Copy container health check script
COPY scripts/healthcheck.sh /bin/healthcheck
RUN chmod +x /bin/healthcheck

# Default data directory for SQLite database, blobs, and Tantivy search index
ENV FASTRMAIL_DATA_DIR=/app/data
ENV RUST_LOG=info
RUN mkdir -p /app/data

# Exposed Ports:
# 2525: Inbound SMTP (RFC 5321)
# 2526: Authenticated SMTP Submission (RFC 6409)
# 1110: POP3 Server (RFC 1939)
# 1143: IMAP4rev2 (RFC 9051)
# 8080: Webmail UI, Admin UI, REST API & JMAP (RFC 8620/8621)
EXPOSE 2525 2526 1110 1143 8080

VOLUME ["/app/data"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["/bin/healthcheck"]

ENTRYPOINT ["/usr/local/bin/fastrmail"]
