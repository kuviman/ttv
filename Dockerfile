FROM kuviman/geng AS builder

WORKDIR /src
# First create a layer with built dependencies to cache them in separate docker layer
COPY Cargo.toml .
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo geng build --release && \
    cargo geng build --release --web && \
    rm -rf src
# Now actually compile the project
COPY . .
RUN touch src/main.rs && \
    cargo geng build --release --web && \
    mv target/geng target/web && \
    cargo geng build --release --all-features && \
    mv target/geng target/server && \
    echo DONE

# Now create a small image
FROM debian:bullseye-slim
WORKDIR /root
RUN apt update && apt install --yes \
    libasound2-dev \
    libfreetype-dev
COPY --from=builder /src/target/web web
COPY --from=builder /src/target/server server
CMD ["/root/server/ttv", "--server=0.0.0.0:80", "--serve=/root/web"]
EXPOSE 80 8000