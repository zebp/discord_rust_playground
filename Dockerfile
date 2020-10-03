FROM rust:1.46

WORKDIR /usr/src/discord_rust_playground
COPY . .
RUN cargo install --path .

CMD ["discord_rust_playground"]