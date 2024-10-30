# Build
FROM rust:1.81.0-alpine as builder

RUN --mount=type=cache,target=/var/cache/apk \
  apk update \
  && apk add musl-dev \
  && apk add inotify-tools \
  && rustup target add aarch64-unknown-linux-musl

WORKDIR /build/task_logger

COPY task_logger/Cargo.toml Cargo.lock ./
RUN mkdir ../task
COPY task/ ../task/
COPY notifier/ ../notifier/

RUN --mount=type=cache,target=/build/task_logger/target \
  mkdir src \
  && echo "fn main() {}" > src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl

COPY task_logger/src/ src/

RUN --mount=type=cache,target=/build/task_logger/target \
  touch src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl \
  && mkdir /output \
  && cp target/aarch64-unknown-linux-musl/release/task_logger* /output/



# Run
FROM alpine as runtime

WORKDIR /opt/task_logger

COPY --from=builder output/task_logger* .

ENTRYPOINT ["./task_logger" ]
