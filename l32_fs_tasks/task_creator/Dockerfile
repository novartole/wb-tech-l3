# Build
FROM rust:1.81.0-alpine as builder

RUN --mount=type=cache,target=/var/cache/apk \
  apk update \
  && apk add musl-dev \
  && rustup target add aarch64-unknown-linux-musl

WORKDIR /build/task_creator

COPY task_creator/Cargo.toml Cargo.lock ./
RUN mkdir ../task
COPY task/ ../task/

RUN --mount=type=cache,target=/build/task_creator/target \
  mkdir src \
  && echo "fn main() {}" > src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl

COPY task_creator/src/ src/

RUN --mount=type=cache,target=/build/task_creator/target \
  touch src/main.rs \
  && cargo build --release --target aarch64-unknown-linux-musl \
  && mkdir /output \
  && cp target/aarch64-unknown-linux-musl/release/task_creator* /output/



# Run
FROM alpine as runtime

WORKDIR /opt/task_creator

COPY --from=builder output/task_creator* .

ENTRYPOINT ["./task_creator" ]
