# Build
FROM rust:1.81.0-alpine as builder

RUN --mount=type=cache,target=/var/cache/apk \
  apk update \
  && apk add musl-dev \
  && apk add inotify-tools \
  && rustup target add aarch64-unknown-linux-musl

WORKDIR /build/task_processor

COPY task_processor/Cargo.toml Cargo.lock ./
RUN mkdir ../task
COPY task/ ../task/
COPY notifier/ ../notifier/

RUN --mount=type=cache,target=/build/task_processor/target \
  mkdir src \
  && echo "fn main() {}" > src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl

COPY task_processor/src/ src/

RUN --mount=type=cache,target=/build/task_processor/target \
  touch src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl \
  && mkdir /output \
  && cp target/aarch64-unknown-linux-musl/release/task_processor* /output/



# Run
FROM alpine as runtime

WORKDIR /opt/task_processor

COPY --from=builder output/task_processor* .

ENTRYPOINT ["./task_processor" ]
