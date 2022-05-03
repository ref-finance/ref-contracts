FROM rust:1.56.1
# Digest: sha256:993a7f2702713250b421e60df250ba57b1c72d557c93283f30d1a428d8087456
# Status: Downloaded newer image for rust:1.56.1
#  ---> ac441dc335cf
LABEL description="Container for builds"

RUN rustup default 1.56.1
RUN rustup target add wasm32-unknown-unknown

RUN apt-get update && apt-get install -y git less vim clang